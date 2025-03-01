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
    
    // Add timezone column to schedules table if it doesn't exist
    info!("Checking for timezone column in schedules table...");
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
    
    if !column_exists {
        info!("Adding timezone column to schedules table...");
        sqlx::query(
            r#"
            ALTER TABLE schedules
            ADD COLUMN timezone VARCHAR(100) DEFAULT 'UTC' NOT NULL;
            "#,
        )
        .execute(pool)
        .await?;
        info!("Timezone column added successfully.");
    } else {
        info!("Timezone column already exists.");
    }

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
    
    // Add is_recurring column to time_slots table if it doesn't exist
    info!("Checking for is_recurring column in time_slots table...");
    let is_recurring_exists = sqlx::query_scalar::<_, bool>(
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
    
    if !is_recurring_exists {
        info!("Adding is_recurring column to time_slots table...");
        sqlx::query(
            r#"
            ALTER TABLE time_slots
            ADD COLUMN is_recurring BOOLEAN NOT NULL DEFAULT FALSE;
            "#,
        )
        .execute(pool)
        .await?;
        info!("is_recurring column added successfully.");
    } else {
        info!("is_recurring column already exists.");
    }

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

    // Create indexes - one at a time
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_time_slots_schedule_id ON time_slots(schedule_id)"
    )
    .execute(pool)
    .await?;
    
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_time_slots_start_time ON time_slots(start_time)"
    )
    .execute(pool)
    .await?;
    
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_time_slots_end_time ON time_slots(end_time)"
    )
    .execute(pool)
    .await?;
    
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_discord_users_schedule_id ON discord_users(schedule_id)"
    )
    .execute(pool)
    .await?;
    
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_group_members_group_id ON group_members(group_id)"
    )
    .execute(pool)
    .await?;
    
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_group_members_discord_id ON group_members(discord_id)"
    )
    .execute(pool)
    .await?;
    
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_discord_groups_server_id ON discord_groups(server_id)"
    )
    .execute(pool)
    .await?;

    info!("Database schema initialized successfully.");
    Ok(())
}