use color_eyre::eyre::Result;
use dotenv::dotenv;
use timesync_api::config::ApiConfig;
use timesync_db::{create_pool, schema::initialize_database};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize error handling
    color_eyre::install()?;

    // Load environment variables
    dotenv().ok();

    // Load configuration
    let config = ApiConfig::from_env()?;

    // Create database connection pool
    let db_pool = create_pool(&config.database_url).await?;

    // Initialize database schema
    initialize_database(&db_pool).await?;

    // Start API server
    timesync_api::start_server(config, db_pool).await?;

    Ok(())
}
