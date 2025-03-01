pub mod repositories;

#[cfg(test)]
pub async fn create_test_pool() -> crate::DbPool {
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
        "postgres://postgres:postgres@localhost:5432/timesync_test".to_string()
    });
    
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database");
    
    // Initialize test schema
    crate::schema::initialize_database(&pool).await
        .expect("Failed to initialize test database schema");
    
    pool
}