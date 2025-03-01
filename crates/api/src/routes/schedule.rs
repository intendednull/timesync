use axum::{
    routing::{get, post, put},
    Router,
};
use std::sync::Arc;

use crate::{handlers, ApiState};

pub fn routes() -> Router<Arc<ApiState>> {
    Router::new()
        .route("/schedules", post(handlers::schedule::create_schedule))
        .route("/schedules/:id", get(handlers::schedule::get_schedule))
        .route("/schedules/:id", put(handlers::schedule::update_schedule))
        .route(
            "/schedules/:id/verify",
            post(handlers::schedule::verify_password),
        )
}