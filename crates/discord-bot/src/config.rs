use eyre::{eyre, Result};
use serde::Deserialize;
use std::env;

/// Configuration for the Discord bot.
/// 
/// This struct contains all the necessary configuration parameters
/// needed for the bot to function properly, including connection details
/// for Discord and the database.
#[derive(Debug, Clone, Deserialize)]
pub struct BotConfig {
    /// Discord bot token (required)
    pub token: String,
    /// Application ID for Discord bot (required)
    pub application_id: u64,
    /// Base URL for the web application (required for schedule links)
    pub web_base_url: String,
    /// Database connection URL (required)
    pub database_url: String,
    /// Prefix for commands (defaults to "!")
    pub command_prefix: Option<String>,
    /// Test guild ID for faster command registration during development
    pub test_guild_id: Option<u64>,
}

impl BotConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let token = env::var("DISCORD_TOKEN")
            .map_err(|_| eyre!("DISCORD_TOKEN environment variable not set"))?;
            
        let application_id = env::var("DISCORD_APPLICATION_ID")
            .map_err(|_| eyre!("DISCORD_APPLICATION_ID environment variable not set"))?
            .parse::<u64>()
            .map_err(|_| eyre!("DISCORD_APPLICATION_ID must be a valid u64"))?;
            
        let web_base_url = env::var("WEB_BASE_URL")
            .map_err(|_| eyre!("WEB_BASE_URL environment variable not set"))?;
            
        let database_url = env::var("DATABASE_URL")
            .map_err(|_| eyre!("DATABASE_URL environment variable not set"))?;
            
        let command_prefix = env::var("DISCORD_COMMAND_PREFIX").ok();
        
        // Optional test guild ID for development
        let test_guild_id = env::var("DISCORD_TEST_GUILD_ID")
            .ok()
            .and_then(|id| id.parse::<u64>().ok());
        
        Ok(Self {
            token,
            application_id,
            web_base_url,
            database_url,
            command_prefix,
            test_guild_id,
        })
    }
    
    /// Get the command prefix (defaults to "!" if not set)
    pub fn command_prefix(&self) -> &str {
        self.command_prefix.as_deref().unwrap_or("!")
    }
}