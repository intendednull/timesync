use axum::{routing::get, Router};
use std::sync::Arc;

use crate::{handlers, ApiState};

pub fn routes() -> Router<Arc<ApiState>> {
    Router::new().route(
        "/api/availability/match",
        get(handlers::availability::match_availability),
    )
}