use color_eyre::eyre::Result;
use dotenv::dotenv;
use timesync_discord_bot::config::BotConfig;
use timesync_db::{create_pool, schema::initialize_database};
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize error handling
    color_eyre::install()?;
    
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    
    info!("Starting TimeSync Discord bot");
    
    // Load environment variables
    dotenv().ok();
    
    // Load configuration
    let config = BotConfig::from_env()?;
    
    // Create database connection pool
    let db_pool = create_pool(&config.database_url).await?;
    
    // Initialize database schema
    initialize_database(&db_pool).await?;
    
    // Start the Discord bot
    match timesync_discord_bot::start_bot(config, db_pool).await {
        Ok(_) => info!("Discord bot shut down gracefully"),
        Err(e) => error!("Discord bot error: {}", e),
    }
    
    Ok(())
}