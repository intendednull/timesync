use eyre::Result;
use sqlx::{Pool, Postgres};
use tracing::info;

pub async fn initialize_database(pool: &Pool<Postgres>) -> Result<()> {
    info!("Initializing database schema...");
    
    // Create schedules table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS schedules (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name VARCHAR(255) NOT NULL,
            password_hash VARCHAR(255) NULL,
            created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
        );
        "#,
    )
    .execute(pool)
    .await?;

    // Create time_slots table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS time_slots (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            schedule_id UUID NOT NULL REFERENCES schedules(id),
            start_time TIMESTAMP WITH TIME ZONE NOT NULL,
            end_time TIMESTAMP WITH TIME ZONE NOT NULL,
            created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
            CONSTRAINT valid_time_range CHECK (end_time > start_time)
        );
        "#,
    )
    .execute(pool)
    .await?;

    // Create discord_users table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS discord_users (
            discord_id VARCHAR(255) PRIMARY KEY,
            schedule_id UUID REFERENCES schedules(id),
            created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
        );
        "#,
    )
    .execute(pool)
    .await?;

    // Create discord_groups table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS discord_groups (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name VARCHAR(255) NOT NULL,
            server_id VARCHAR(255) NOT NULL,
            created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
        );
        "#,
    )
    .execute(pool)
    .await?;

    // Create group_members table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS group_members (
            group_id UUID NOT NULL REFERENCES discord_groups(id),
            discord_id VARCHAR(255) NOT NULL REFERENCES discord_users(discord_id),
            PRIMARY KEY (group_id, discord_id)
        );
        "#,
    )
    .execute(pool)
    .await?;

    // Create indexes
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_time_slots_schedule_id ON time_slots(schedule_id);
        CREATE INDEX IF NOT EXISTS idx_time_slots_start_time ON time_slots(start_time);
        CREATE INDEX IF NOT EXISTS idx_time_slots_end_time ON time_slots(end_time);
        CREATE INDEX IF NOT EXISTS idx_discord_users_schedule_id ON discord_users(schedule_id);
        CREATE INDEX IF NOT EXISTS idx_group_members_group_id ON group_members(group_id);
        CREATE INDEX IF NOT EXISTS idx_group_members_discord_id ON group_members(discord_id);
        CREATE INDEX IF NOT EXISTS idx_discord_groups_server_id ON discord_groups(server_id);
        "#,
    )
    .execute(pool)
    .await?;

    info!("Database schema initialized successfully.");
    Ok(())
}