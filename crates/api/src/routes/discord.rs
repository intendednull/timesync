use axum::{
    routing::{get, post, put},
    Router,
};
use std::sync::Arc;

use crate::{handlers, ApiState};

pub fn routes() -> Router<Arc<ApiState>> {
    Router::new()
        .route(
            "/api/discord/users",
            post(handlers::discord::create_discord_user),
        )
        .route(
            "/api/discord/users/:discord_id",
            get(handlers::discord::get_discord_user),
        )
        .route(
            "/api/discord/groups",
            post(handlers::discord::create_discord_group),
        )
        .route(
            "/api/discord/groups/:id",
            get(handlers::discord::get_discord_group),
        )
        .route(
            "/api/discord/groups/:id",
            put(handlers::discord::update_discord_group),
        )
}