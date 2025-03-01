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
    timezone: &str,
) -> Result<DbSchedule> {
    let id = Uuid::new_v4();
    let now = Utc::now();

    tracing::debug!(
        "Creating schedule: id={}, name={}, has_password={}, timezone={}",
        id, name, password_hash.is_some(), timezone
    );

    // First check if the timezone column exists
    let column_exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1 
            FROM information_schema.columns 
            WHERE table_name = 'schedules' AND column_name = 'timezone'
        );
        "#,
    )
    .fetch_one(pool)
    .await?;

    let schedule = if column_exists {
        // Use the timezone column
        tracing::debug!("Using the timezone column");
        sqlx::query_as::<_, DbSchedule>(
            r#"
            INSERT INTO schedules (id, name, password_hash, timezone, created_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, name, password_hash, timezone, created_at
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(password_hash)
        .bind(timezone)
        .bind(now)
        .fetch_one(pool)
        .await?
    } else {
        // Legacy schema without timezone column
        tracing::debug!("Legacy schema without timezone column - using default value");
        sqlx::query_as::<_, DbSchedule>(
            r#"
            INSERT INTO schedules (id, name, password_hash, created_at)
            VALUES ($1, $2, $3, $4)
            RETURNING id, name, password_hash, 'UTC' as timezone, created_at
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(password_hash)
        .bind(now)
        .fetch_one(pool)
        .await?
    };

    tracing::debug!("Schedule created successfully: id={}", id);
    Ok(schedule)
}

pub async fn get_schedule_by_id(pool: &Pool<Postgres>, id: Uuid) -> Result<Option<DbSchedule>> {
    tracing::debug!("Getting schedule by id: {}", id);

    // First check if the timezone column exists
    let column_exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1 
            FROM information_schema.columns 
            WHERE table_name = 'schedules' AND column_name = 'timezone'
        );
        "#,
    )
    .fetch_one(pool)
    .await?;

    let schedule = if column_exists {
        // Use the timezone column
        tracing::debug!("Using the timezone column");
        sqlx::query_as::<_, DbSchedule>(
            r#"
            SELECT id, name, password_hash, timezone, created_at
            FROM schedules
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
    } else {
        // Legacy schema without timezone column
        tracing::debug!("Legacy schema without timezone column - using default value");
        sqlx::query_as::<_, DbSchedule>(
            r#"
            SELECT id, name, password_hash, 'UTC' as timezone, created_at
            FROM schedules
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
    };

    if let Some(s) = &schedule {
        tracing::debug!("Schedule found: id={}, name={}", s.id, s.name);
    } else {
        tracing::debug!("Schedule not found: id={}", id);
    }

    Ok(schedule)
}

pub async fn update_schedule(
    pool: &Pool<Postgres>,
    id: Uuid,
    name: Option<&str>,
    timezone: Option<&str>,
) -> Result<DbSchedule> {
    let schedule = get_schedule_by_id(pool, id)
        .await?
        .ok_or_else(|| eyre!("Schedule not found"))?;

    let name = name.unwrap_or(&schedule.name);
    let timezone = timezone.unwrap_or(&schedule.timezone);

    let updated_schedule = sqlx::query_as::<_, DbSchedule>(
        r#"
        UPDATE schedules
        SET name = $2, timezone = $3
        WHERE id = $1
        RETURNING id, name, password_hash, timezone, created_at
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(timezone)
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