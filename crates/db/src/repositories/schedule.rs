use crate::models::DbSchedule;
use chrono::Utc;
use eyre::{eyre, Result};
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use argon2::{Argon2, PasswordVerifier};

pub async fn create_schedule(
    pool: &Pool<Postgres>,
    name: &str,
    password_hash: Option<&str>,
) -> Result<DbSchedule> {
    let id = Uuid::new_v4();
    let now = Utc::now();

    let schedule = sqlx::query_as::<_, DbSchedule>(
        r#"
        INSERT INTO schedules (id, name, password_hash, created_at)
        VALUES ($1, $2, $3, $4)
        RETURNING id, name, password_hash, created_at
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(password_hash)
    .bind(now)
    .fetch_one(pool)
    .await?;

    Ok(schedule)
}

pub async fn get_schedule_by_id(pool: &Pool<Postgres>, id: Uuid) -> Result<Option<DbSchedule>> {
    let schedule = sqlx::query_as::<_, DbSchedule>(
        r#"
        SELECT id, name, password_hash, created_at
        FROM schedules
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(schedule)
}

pub async fn update_schedule(
    pool: &Pool<Postgres>,
    id: Uuid,
    name: Option<&str>,
) -> Result<DbSchedule> {
    let schedule = get_schedule_by_id(pool, id)
        .await?
        .ok_or_else(|| eyre!("Schedule not found"))?;

    let name = name.unwrap_or(&schedule.name);

    let updated_schedule = sqlx::query_as::<_, DbSchedule>(
        r#"
        UPDATE schedules
        SET name = $2
        WHERE id = $1
        RETURNING id, name, password_hash, created_at
        "#,
    )
    .bind(id)
    .bind(name)
    .fetch_one(pool)
    .await?;

    Ok(updated_schedule)
}

pub async fn verify_password(
    pool: &Pool<Postgres>,
    id: Uuid,
    password: &str,
) -> Result<bool> {
    let schedule = get_schedule_by_id(pool, id)
        .await?
        .ok_or_else(|| eyre!("Schedule not found"))?;

    match schedule.password_hash {
        Some(hash) => {
            let parsed_hash = argon2::PasswordHash::new(&hash)
                .map_err(|e| eyre!("Invalid password hash: {}", e))?;
            let is_valid = Argon2::default()
                .verify_password(password.as_bytes(), &parsed_hash)
                .is_ok();
            Ok(is_valid)
        }
        None => Ok(true), // If no password is set, consider any password valid
    }
}