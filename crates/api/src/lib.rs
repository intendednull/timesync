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

use axum::{
    Router,
    routing::{get, get_service},
    response::{IntoResponse, Html},
};
use eyre::Result;
use sqlx::PgPool;
use tokio::net::TcpListener;
use tower_http::services::ServeDir;
use tracing::info;
use tracing_subscriber::FmtSubscriber;

/// Shared application state that is accessible to all request handlers
///
/// This struct encapsulates dependencies that are shared across the application,
/// such as database connections, caches, and configuration values.
///
/// # Example
///
/// ```no_run
/// use std::sync::Arc;
/// use axum::Router;
/// use sqlx::PgPool;
/// use timesync_api::ApiState;
/// 
/// # async fn example() {
/// #     let db_pool = PgPool::connect("postgres://postgres:password@localhost/test").await.unwrap();
///     let state = Arc::new(ApiState { db_pool });
///     let app: Router = Router::new().with_state(state);
///     // Use app...
/// # }
/// # fn main() {}
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
/// ```
/// use eyre::Result;
/// use sqlx::PgPool;
/// use timesync_api::{config, start_server};
/// 
/// async fn main_example() -> Result<()> {
///     let config = config::ApiConfig::from_env()?;
///     let db_pool = PgPool::connect(&config.database_url).await?;
///     start_server(config, db_pool).await?;
///     Ok(())
/// }
/// # 
/// # // This enables doctests to compile but not run the example
/// # fn main() {}
/// ```
pub async fn start_server(config: config::ApiConfig, db_pool: PgPool) -> Result<()> {
    // Initialize tracing for logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(config.log_level)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Create shared state with dependencies
    let state = Arc::new(ApiState { db_pool });

    // Define static file handler
    let static_dir = std::env::current_dir()?.join("src");
    
    // Use a properly configured static file service with MIME types
    let _static_service = tower_http::services::ServeDir::new(static_dir.clone())
        .append_index_html_on_directories(true)
        .precompressed_br()
        .precompressed_gzip();
    
    // Define the frontend fallback handler
    async fn serve_frontend_fallback(path: Option<axum::extract::Path<String>>) -> impl IntoResponse {
        // Determine which HTML file to serve based on the path
        let file_path = match &path {
            // If path contains "edit", serve edit.html
            Some(p) if p.0.contains("edit") => "src/edit.html",
            // If path is just a UUID, serve view.html
            Some(p) if !p.0.is_empty() && !p.0.contains("/") => "src/view.html",
            // If path is "availability", serve availability.html
            Some(p) if p.0 == "availability" => "src/availability.html",
            // If path is "create", serve index.html (this is for discord integration)
            Some(p) if p.0 == "create" => "src/index.html",
            // Default to index.html
            _ => "src/index.html",
        };
        
        let html_content = tokio::fs::read_to_string(file_path).await.unwrap_or_else(|_| {
            eprintln!("Error: Could not load {}", file_path);
            "<!DOCTYPE html><html><body><h1>Error: Could not load requested page</h1></body></html>".to_string()
        });
        
        Html(html_content)
    }
    
    // Build the application router with all routes
    let app = Router::new()
        // API routes - must be first to ensure they're matched properly
        .nest("/api", Router::new()
            // Health check endpoints
            .merge(routes::health::routes())
            // Schedule management endpoints
            .merge(routes::schedule::routes())
            // Discord integration endpoints
            .merge(routes::discord::routes())
            // Availability management endpoints
            .merge(routes::availability::routes())
        )
        // Static file routes - serve specific directories first
        .nest_service("/assets", get_service(ServeDir::new(std::env::current_dir()?.join("src/assets"))))
        .nest_service("/js", get_service(ServeDir::new(std::env::current_dir()?.join("src/js"))))
        .nest_service("/css", get_service(ServeDir::new(std::env::current_dir()?.join("src/css"))))
        
        // Serve specific static files directly
        .route_service("/index.html", get_service(ServeDir::new(std::env::current_dir()?.join("src"))))
        .route_service("/view.html", get_service(ServeDir::new(std::env::current_dir()?.join("src"))))
        .route_service("/edit.html", get_service(ServeDir::new(std::env::current_dir()?.join("src"))))
        .route_service("/availability.html", get_service(ServeDir::new(std::env::current_dir()?.join("src"))))
        .route_service("/styles.css", get_service(ServeDir::new(std::env::current_dir()?.join("src"))))
        .route_service("/scripts.js", get_service(ServeDir::new(std::env::current_dir()?.join("src"))))
        .route_service("/view.js", get_service(ServeDir::new(std::env::current_dir()?.join("src"))))
        .route_service("/edit.js", get_service(ServeDir::new(std::env::current_dir()?.join("src"))))
        .route_service("/availability.js", get_service(ServeDir::new(std::env::current_dir()?.join("src"))))
        
        // Frontend application routes
        .route("/", get(|| serve_frontend_fallback(None)))
        .route("/availability", get(|| serve_frontend_fallback(Some(axum::extract::Path("availability".to_string())))))
        .route("/create", get(|| async { 
            // For /create path, always render the index.html directly without treating "create" as an ID
            let html_content = tokio::fs::read_to_string("src/index.html").await.unwrap_or_else(|_| {
                eprintln!("Error: Could not load src/index.html");
                "<!DOCTYPE html><html><body><h1>Error: Could not load requested page</h1></body></html>".to_string()
            });
            Html(html_content)
        }))
        .route("/:id/edit", get(|path: axum::extract::Path<String>| serve_frontend_fallback(Some(path))))
        .route("/:id", get(|path: axum::extract::Path<String>| serve_frontend_fallback(Some(path))))
        
        // Fallback - use fallback function for any other routes not matched
        .fallback(get(|| serve_frontend_fallback(None)))
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

    // Note: In a production app, you might want to add request timeout middleware
    // For simplicity, we're omitting it in this version

    // Start the HTTP server
    let addr = config.server_addr();
    let listener = TcpListener::bind(&addr).await?;
    info!("Server listening on http://{}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}