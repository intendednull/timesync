//! # API Configuration Module
//!
//! This module handles loading and managing configuration for the TimeSync API server.
//! It retrieves configuration values from environment variables and provides defaults
//! where appropriate.
//!
//! ## Environment Variables
//!
//! The following environment variables are used:
//!
//! - `API_HOST`: The host address to bind the server to (default: "0.0.0.0")
//! - `API_PORT`: The port to listen on (default: 3000)
//! - `DATABASE_URL`: PostgreSQL connection string (required)
//! - `LOG_LEVEL`: Logging level (default: "info")
//! - `API_CORS_ORIGINS`: Comma-separated list of allowed CORS origins
//! - `JWT_SECRET`: Secret key for JWT token generation (for protected endpoints)

use eyre::{Result, WrapErr};
use std::env;
use tracing::Level;

/// Configuration for the TimeSync API server
///
/// This struct encapsulates all configuration options for the API server,
/// including networking, database connections, and security settings.
///
/// # Example
///
/// ```
/// use eyre::Result;
/// use timesync_api::config::ApiConfig;
/// 
/// fn example() -> Result<()> {
///     let config = ApiConfig::from_env()?;
///     println!("Starting server on {}:{}", config.host, config.port);
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ApiConfig {
    /// Host address for the API server (e.g., "127.0.0.1", "0.0.0.0")
    pub host: String,
    
    /// Port for the API server to listen on
    pub port: u16,
    
    /// PostgreSQL database connection string
    pub database_url: String,
    
    /// Log level for the application
    pub log_level: Level,
    
    /// CORS allowed origins (optional)
    pub cors_origins: Option<Vec<String>>,
    
    /// JWT secret for authentication (optional)
    pub jwt_secret: Option<String>,
    
    /// Request timeout in seconds
    pub request_timeout: u64,
}

impl ApiConfig {
    /// Creates a new ApiConfig from environment variables
    ///
    /// This function loads configuration values from environment variables,
    /// providing sensible defaults where possible. Some values like DATABASE_URL
    /// are required and will cause an error if not set.
    ///
    /// # Returns
    ///
    /// * `Result<Self>` - Configuration object or error
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The DATABASE_URL environment variable is not set
    /// - The API_PORT value cannot be parsed as a u16
    /// - The LOG_LEVEL value cannot be parsed as a valid log level
    pub fn from_env() -> Result<Self> {
        // Network settings
        let host = env::var("API_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = env::var("API_PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse()
            .wrap_err("Invalid API_PORT value")?;
        
        // Database settings
        let database_url = env::var("DATABASE_URL")
            .wrap_err("DATABASE_URL environment variable must be set")?;
        
        // Logging settings
        let log_level = match env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()).as_str() {
            "trace" => Level::TRACE,
            "debug" => Level::DEBUG,
            "info" => Level::INFO,
            "warn" => Level::WARN,
            "error" => Level::ERROR,
            _ => Level::INFO,
        };
        
        // CORS settings
        let cors_origins = env::var("API_CORS_ORIGINS").ok().map(|origins| {
            origins.split(',').map(|s| s.trim().to_string()).collect()
        });
        
        // Security settings
        let jwt_secret = env::var("JWT_SECRET").ok();
        
        // Performance settings
        let request_timeout = env::var("API_REQUEST_TIMEOUT_SECONDS")
            .unwrap_or_else(|_| "30".to_string())
            .parse()
            .unwrap_or(30);
        
        Ok(Self {
            host,
            port,
            database_url,
            log_level,
            cors_origins,
            jwt_secret,
            request_timeout,
        })
    }
    
    /// Returns the server address as a string
    ///
    /// # Returns
    ///
    /// * `String` - Formatted server address (e.g., "127.0.0.1:8080")
    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}