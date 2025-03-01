use color_eyre::eyre::Result;
use dotenv::dotenv;
use timesync_db::schema::initialize_database;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize error handling
    color_eyre::install()?;

    // Load environment variables
    dotenv().ok();

    // Get database connection string from environment variable
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/timesync".to_string());
    
    println!("Connecting to database...");
    // Create database connection pool
    let db_pool = timesync_db::create_pool(&database_url).await?;
    
    // Initialize database schema
    println!("Initializing database schema...");
    initialize_database(&db_pool).await?;
    println!("Database schema initialized successfully.");
    
    Ok(())
}