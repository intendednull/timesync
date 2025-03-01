use axum::{
    routing::{get, post, put},
    Router,
};
use std::sync::Arc;

use crate::{handlers, ApiState};

pub fn routes() -> Router<Arc<ApiState>> {
    Router::new()
        .route("/api/schedules", post(handlers::schedule::create_schedule))
        .route("/api/schedules/:id", get(handlers::schedule::get_schedule))
        .route("/api/schedules/:id", put(handlers::schedule::update_schedule))
        .route(
            "/api/schedules/:id/verify",
            post(handlers::schedule::verify_password),
        )
}