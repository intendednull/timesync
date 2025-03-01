use axum::{
    routing::get,
    Json, Router,
};
use serde::Serialize;
use std::sync::Arc;

use crate::ApiState;

#[derive(Serialize)]
struct HealthResponse {
    status: String,
}

#[derive(Serialize)]
struct VersionResponse {
    version: String,
}

async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

async fn version() -> Json<VersionResponse> {
    Json(VersionResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

pub fn routes() -> Router<Arc<ApiState>> {
    Router::new()
        .route("/health", get(health_check))
        .route("/version", get(version))
}