//! # TimeSync API
//!
//! The API crate provides the web server implementation for the TimeSync scheduling service.
//! It defines RESTful endpoints for managing schedules, availability, and integrations.
//!
//! ## Architecture
//!
//! This crate follows a layered architecture:
//!
//! - **Routes**: Define API endpoints and URL structure
//! - **Handlers**: Implement request processing logic
//! - **Middleware**: Provide cross-cutting concerns like authentication and error handling
//! - **Config**: Handle environment and application configuration
//!
//! The API uses Axum as the web framework and SQLx for database interactions.

/// Configuration module for API settings
pub mod config;
/// Request handlers that implement business logic
pub mod handlers;
/// Middleware for authentication, logging, and error handling
pub mod middleware;
/// Route definitions and API endpoint structure
pub mod routes;

use std::sync::Arc;

use axum::Router;
use eyre::Result;
use sqlx::PgPool;
use tokio::net::TcpListener;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

/// Shared application state that is accessible to all request handlers
///
/// This struct encapsulates dependencies that are shared across the application,
/// such as database connections, caches, and configuration values.
///
/// # Example
///
/// ```rust
/// let state = Arc::new(ApiState { db_pool });
/// let app = Router::new().with_state(state);
/// ```
pub struct ApiState {
    /// PostgreSQL connection pool for database operations
    pub db_pool: PgPool,
}

/// Starts the API server with the provided configuration and database connection
///
/// This function initializes the application, sets up logging, configures routes,
/// and starts the HTTP server.
///
/// # Arguments
///
/// * `config` - API configuration including host, port, and other settings
/// * `db_pool` - PostgreSQL connection pool for database operations
///
/// # Returns
///
/// * `Result<()>` - Success or error result
///
/// # Example
///
/// ```rust
/// let config = config::load_config()?;
/// let db_pool = db::create_pool(&config.database_url).await?;
/// start_server(config, db_pool).await?;
/// ```
pub async fn start_server(config: config::ApiConfig, db_pool: PgPool) -> Result<()> {
    // Initialize tracing for logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(config.log_level)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Create shared state with dependencies
    let state = Arc::new(ApiState { db_pool });

    // Build the application router with all routes
    let app = Router::new()
        // Health check endpoints
        .merge(routes::health::routes())
        // Schedule management endpoints
        .merge(routes::schedule::routes())
        // Discord integration endpoints
        .merge(routes::discord::routes())
        // Availability management endpoints
        .merge(routes::availability::routes())
        // Attach shared state to all routes
        .with_state(state);

    // Apply CORS configuration if origins are specified
    let app = if let Some(origins) = &config.cors_origins {
        let cors = tower_http::cors::CorsLayer::new()
            .allow_methods([
                axum::http::Method::GET,
                axum::http::Method::POST,
                axum::http::Method::PUT,
                axum::http::Method::DELETE,
                axum::http::Method::OPTIONS,
            ])
            .allow_headers([
                axum::http::header::CONTENT_TYPE,
                axum::http::header::AUTHORIZATION,
                axum::http::header::ACCEPT,
            ])
            .allow_origin(
                origins
                    .iter()
                    .map(|origin| origin.parse().unwrap())
                    .collect::<Vec<_>>(),
            )
            .allow_credentials(true);
        
        app.layer(cors)
    } else {
        app
    };

    // Add request timeout middleware
    let app = app.layer(
        tower::ServiceBuilder::new()
            .timeout(std::time::Duration::from_secs(config.request_timeout))
            .into_inner(),
    );

    // Start the HTTP server
    let addr = config.server_addr();
    let listener = TcpListener::bind(&addr).await?;
    info!("Server listening on http://{}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}