use std::sync::Arc;

use sqlx::{postgres::PgPoolOptions, PgPool};
use timesync_api::ApiState;
use timesync_db::mock::repositories::{
    MockDiscordGroupRepo, MockDiscordUserRepo, MockScheduleRepo, MockTimeSlotRepo,
};


pub struct TestContext {
    // Add mocks for each repository
    pub schedule_repo: MockScheduleRepo,
    pub time_slot_repo: MockTimeSlotRepo,
    pub discord_user_repo: MockDiscordUserRepo,
    pub discord_group_repo: MockDiscordGroupRepo,
}

impl TestContext {
    pub fn new() -> Self {
        Self {
            schedule_repo: MockScheduleRepo::new(),
            time_slot_repo: MockTimeSlotRepo::new(),
            discord_user_repo: MockDiscordUserRepo::new(),
            discord_group_repo: MockDiscordGroupRepo::new(),
        }
    }

    // Build state with mock repositories
    pub fn build_state(&self) -> Arc<ApiState> {
        // Create a mock database connection
        let pool = PgPool::connect_lazy("postgres://fake:fake@localhost/fake").unwrap_or_else(|_| {
            // Fallback to a dummy db URL to avoid panicking 
            PgPool::connect_lazy("sqlite::memory:").unwrap()
        });
        
        Arc::new(ApiState { db_pool: pool })
    }
    
    // Tests now use direct mocking rather than this approach
}

// Helper function to create an in-memory database for real integration tests
// We're not using this in our unit tests, but it would be useful for integration tests
pub async fn create_test_db() -> PgPool {
    // This would connect to a real test database in integration tests
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://postgres:postgres@localhost:5432/timesync_test")
        .await
        .unwrap();
    
    // Initialize database schema
    timesync_db::schema::initialize_database(&pool).await.unwrap();
    
    pool
}