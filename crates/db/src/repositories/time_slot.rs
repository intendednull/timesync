use crate::models::DbTimeSlot;
use chrono::{DateTime, Utc};
use eyre::Result;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

pub async fn create_time_slot(
    pool: &Pool<Postgres>,
    schedule_id: Uuid,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    is_recurring: bool,
) -> Result<DbTimeSlot> {
    let id = Uuid::new_v4();
    let now = Utc::now();

    tracing::debug!(
        "Creating time slot: id={}, schedule_id={}, start_time={}, end_time={}, is_recurring={}",
        id, schedule_id, start_time, end_time, is_recurring
    );

    // First check if the is_recurring column exists
    let column_exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1 
            FROM information_schema.columns 
            WHERE table_name = 'time_slots' AND column_name = 'is_recurring'
        );
        "#,
    )
    .fetch_one(pool)
    .await?;

    let time_slot = if column_exists {
        // Use the is_recurring column
        tracing::debug!("Using the is_recurring column");
        sqlx::query_as::<_, DbTimeSlot>(
            r#"
            INSERT INTO time_slots (id, schedule_id, start_time, end_time, is_recurring, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, schedule_id, start_time, end_time, is_recurring, created_at
            "#,
        )
        .bind(id)
        .bind(schedule_id)
        .bind(start_time)
        .bind(end_time)
        .bind(is_recurring)
        .bind(now)
        .fetch_one(pool)
        .await?
    } else {
        // Legacy schema without is_recurring column
        tracing::debug!("Legacy schema without is_recurring column - using default value");
        sqlx::query_as::<_, DbTimeSlot>(
            r#"
            INSERT INTO time_slots (id, schedule_id, start_time, end_time, created_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, schedule_id, start_time, end_time, false as is_recurring, created_at
            "#,
        )
        .bind(id)
        .bind(schedule_id)
        .bind(start_time)
        .bind(end_time)
        .bind(now)
        .fetch_one(pool)
        .await?
    };

    tracing::debug!("Time slot created successfully: id={}", id);
    Ok(time_slot)
}

pub async fn get_time_slots_by_schedule_id(
    pool: &Pool<Postgres>,
    schedule_id: Uuid,
) -> Result<Vec<DbTimeSlot>> {
    let time_slots = sqlx::query_as::<_, DbTimeSlot>(
        r#"
        SELECT id, schedule_id, start_time, end_time, is_recurring, created_at
        FROM time_slots
        WHERE schedule_id = $1
        ORDER BY start_time ASC
        "#,
    )
    .bind(schedule_id)
    .fetch_all(pool)
    .await?;

    Ok(time_slots)
}

pub async fn delete_time_slots_by_schedule_id(
    pool: &Pool<Postgres>,
    schedule_id: Uuid,
) -> Result<()> {
    sqlx::query(
        r#"
        DELETE FROM time_slots
        WHERE schedule_id = $1
        "#,
    )
    .bind(schedule_id)
    .execute(pool)
    .await?;

    Ok(())
}