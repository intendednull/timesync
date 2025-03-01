use axum::{
    routing::{get, post, put},
    Router,
};
use std::sync::Arc;

use crate::{handlers, ApiState};

pub fn routes() -> Router<Arc<ApiState>> {
    Router::new()
        .route(
            "/discord/users",
            post(handlers::discord::create_discord_user),
        )
        .route(
            "/discord/users/:discord_id",
            get(handlers::discord::get_discord_user),
        )
        .route(
            "/discord/groups",
            post(handlers::discord::create_discord_group),
        )
        .route(
            "/discord/groups/:id",
            get(handlers::discord::get_discord_group),
        )
        .route(
            "/discord/groups/:id",
            put(handlers::discord::update_discord_group),
        )
}