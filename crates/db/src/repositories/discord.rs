use crate::models::{DbDiscordGroup, DbDiscordUser, DbGroupMember};
use chrono::Utc;
use eyre::{eyre, Result};
use sqlx::{Pool, Postgres};
use uuid::Uuid;

// Discord User Repository

pub async fn create_discord_user(
    pool: &Pool<Postgres>,
    discord_id: &str,
    schedule_id: Option<Uuid>,
) -> Result<DbDiscordUser> {
    let now = Utc::now();

    let discord_user = sqlx::query_as::<_, DbDiscordUser>(
        r#"
        INSERT INTO discord_users (discord_id, schedule_id, created_at)
        VALUES ($1, $2, $3)
        ON CONFLICT (discord_id) 
        DO UPDATE SET schedule_id = $2
        RETURNING discord_id, schedule_id, created_at
        "#,
    )
    .bind(discord_id)
    .bind(schedule_id)
    .bind(now)
    .fetch_one(pool)
    .await?;

    Ok(discord_user)
}

pub async fn get_discord_user_by_id(
    pool: &Pool<Postgres>,
    discord_id: &str,
) -> Result<Option<DbDiscordUser>> {
    let discord_user = sqlx::query_as::<_, DbDiscordUser>(
        r#"
        SELECT discord_id, schedule_id, created_at
        FROM discord_users
        WHERE discord_id = $1
        "#,
    )
    .bind(discord_id)
    .fetch_optional(pool)
    .await?;

    Ok(discord_user)
}

// Discord Group Repository

pub async fn create_discord_group(
    pool: &Pool<Postgres>,
    name: &str,
    server_id: &str,
    role_id: Option<&str>,
) -> Result<DbDiscordGroup> {
    let id = Uuid::new_v4();
    let now = Utc::now();

    let discord_group = sqlx::query_as::<_, DbDiscordGroup>(
        r#"
        INSERT INTO discord_groups (id, name, server_id, role_id, created_at)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, name, server_id, role_id, created_at
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(server_id)
    .bind(role_id)
    .bind(now)
    .fetch_one(pool)
    .await?;

    Ok(discord_group)
}

pub async fn get_discord_group_by_id(
    pool: &Pool<Postgres>,
    id: Uuid,
) -> Result<Option<DbDiscordGroup>> {
    let discord_group = sqlx::query_as::<_, DbDiscordGroup>(
        r#"
        SELECT id, name, server_id, role_id, created_at
        FROM discord_groups
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(discord_group)
}

pub async fn update_discord_group(
    pool: &Pool<Postgres>,
    id: Uuid,
    name: Option<&str>,
    role_id: Option<&str>,
) -> Result<DbDiscordGroup> {
    let group = get_discord_group_by_id(pool, id)
        .await?
        .ok_or_else(|| eyre!("Discord group not found"))?;

    let name = name.unwrap_or(&group.name);

    let updated_group = sqlx::query_as::<_, DbDiscordGroup>(
        r#"
        UPDATE discord_groups
        SET name = $2, role_id = $3
        WHERE id = $1
        RETURNING id, name, server_id, role_id, created_at
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(role_id.or_else(|| group.role_id.as_deref()))
    .fetch_one(pool)
    .await?;

    Ok(updated_group)
}

// Group Membership Repository

pub async fn add_member_to_group(
    pool: &Pool<Postgres>,
    group_id: Uuid,
    discord_id: &str,
) -> Result<DbGroupMember> {
    let group_member = sqlx::query_as::<_, DbGroupMember>(
        r#"
        INSERT INTO group_members (group_id, discord_id)
        VALUES ($1, $2)
        ON CONFLICT (group_id, discord_id) DO NOTHING
        RETURNING group_id, discord_id
        "#,
    )
    .bind(group_id)
    .bind(discord_id)
    .fetch_one(pool)
    .await?;

    Ok(group_member)
}

pub async fn remove_member_from_group(
    pool: &Pool<Postgres>,
    group_id: Uuid,
    discord_id: &str,
) -> Result<()> {
    sqlx::query(
        r#"
        DELETE FROM group_members
        WHERE group_id = $1 AND discord_id = $2
        "#,
    )
    .bind(group_id)
    .bind(discord_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_group_members(
    pool: &Pool<Postgres>,
    group_id: Uuid,
) -> Result<Vec<DbGroupMember>> {
    let members = sqlx::query_as::<_, DbGroupMember>(
        r#"
        SELECT group_id, discord_id
        FROM group_members
        WHERE group_id = $1
        "#,
    )
    .bind(group_id)
    .fetch_all(pool)
    .await?;

    Ok(members)
}

pub async fn get_user_groups(
    pool: &Pool<Postgres>,
    discord_id: &str,
) -> Result<Vec<DbDiscordGroup>> {
    let groups = sqlx::query_as::<_, DbDiscordGroup>(
        r#"
        SELECT g.id, g.name, g.server_id, g.role_id, g.created_at
        FROM discord_groups g
        JOIN group_members gm ON g.id = gm.group_id
        WHERE gm.discord_id = $1
        "#,
    )
    .bind(discord_id)
    .fetch_all(pool)
    .await?;

    Ok(groups)
}

pub async fn update_group_role_id(
    pool: &Pool<Postgres>,
    id: Uuid,
    role_id: &str,
) -> Result<DbDiscordGroup> {
    let updated_group = sqlx::query_as::<_, DbDiscordGroup>(
        r#"
        UPDATE discord_groups
        SET role_id = $2
        WHERE id = $1
        RETURNING id, name, server_id, role_id, created_at
        "#,
    )
    .bind(id)
    .bind(role_id)
    .fetch_one(pool)
    .await?;

    Ok(updated_group)
}