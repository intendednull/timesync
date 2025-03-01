use eyre::Result;
use serenity::{
    Client, 
    prelude::GatewayIntents,
};
use sqlx::PgPool;
use tracing::info;

pub mod commands;
pub mod handlers;
pub mod config;

/// Start the Discord bot with the provided configuration and database connection.
///
/// This function initializes and runs the Discord bot with the given configuration
/// and database pool. It will continue running until the bot disconnects or an error occurs.
///
/// # Arguments
///
/// * `config` - The bot configuration containing token, application ID, etc.
/// * `db_pool` - A PostgreSQL connection pool for database operations
///
/// # Returns
///
/// * `Ok(())` if the bot shut down gracefully
/// * `Err` if an error occurred during initialization or operation
pub async fn start_bot(config: config::BotConfig, db_pool: PgPool) -> Result<()> {
    info!("Starting Discord bot");
    
    // Create a new Discord client
    let handler = handlers::Handler::new(config.clone(), db_pool);
    
    // Configure the client
    let mut client = Client::builder(&config.token, GatewayIntents::non_privileged())
        .event_handler(handler)
        .await?;
        
    // Start the client
    info!("Connecting to Discord...");
    client.start().await?;
    
    Ok(())
}