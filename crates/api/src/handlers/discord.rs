use axum::{extract::{Path, State}, Json};
use eyre::Result;
use std::sync::Arc;
use timesync_core::{
    errors::TimeError,
    models::discord::{
        CreateDiscordGroupRequest, CreateDiscordGroupResponse, CreateDiscordUserRequest,
        CreateDiscordUserResponse, DiscordGroupMember, GetDiscordGroupResponse,
        GetDiscordUserResponse, UpdateDiscordGroupRequest, UpdateDiscordGroupResponse,
        UpdateDiscordGroupRoleRequest, UpdateDiscordGroupRoleResponse,
    },
};
use uuid::Uuid;

use crate::{ApiState, middleware::error_handling::AppError};

#[axum::debug_handler]
pub async fn create_discord_user(
    State(state): State<Arc<ApiState>>,
    Json(payload): Json<CreateDiscordUserRequest>,
) -> Result<Json<CreateDiscordUserResponse>, AppError> {
    // Check if schedule exists
    let schedule_exists = timesync_db::repositories::schedule::get_schedule_by_id(
        &state.db_pool,
        payload.schedule_id,
    )
    .await
    .map_err(TimeError::Database)?
    .is_some();

    if !schedule_exists {
        return Err(AppError(TimeError::NotFound(format!(
            "Schedule with ID {} not found",
            payload.schedule_id
        ))));
    }

    // Create discord user in database
    let db_discord_user = timesync_db::repositories::discord::create_discord_user(
        &state.db_pool,
        &payload.discord_id,
        Some(payload.schedule_id),
    )
    .await
    .map_err(TimeError::Database)?;

    let response = CreateDiscordUserResponse {
        discord_id: db_discord_user.discord_id,
        schedule_id: db_discord_user.schedule_id.unwrap(), // Safe due to the check above
    };

    Ok(Json(response))
}

#[axum::debug_handler]
pub async fn get_discord_user(
    State(state): State<Arc<ApiState>>,
    Path(discord_id): Path<String>,
) -> Result<Json<GetDiscordUserResponse>, AppError> {
    // Get discord user from database
    let db_discord_user =
        timesync_db::repositories::discord::get_discord_user_by_id(&state.db_pool, &discord_id)
            .await
            .map_err(TimeError::Database)?
            .ok_or_else(|| {
                TimeError::NotFound(format!("Discord user with ID {} not found", discord_id))
            })?;

    let response = GetDiscordUserResponse {
        discord_id: db_discord_user.discord_id,
        schedule_id: db_discord_user.schedule_id,
    };

    Ok(Json(response))
}

#[axum::debug_handler]
pub async fn create_discord_group(
    State(state): State<Arc<ApiState>>,
    Json(payload): Json<CreateDiscordGroupRequest>,
) -> Result<Json<CreateDiscordGroupResponse>, AppError> {
    // Create discord group in database
    let db_discord_group = timesync_db::repositories::discord::create_discord_group(
        &state.db_pool,
        &payload.name,
        &payload.server_id,
        None, // Initially no role_id; it will be set by the Discord bot
    )
    .await
    .map_err(TimeError::Database)?;

    // Add members to group
    for discord_id in &payload.member_ids {
        // Ensure the user exists
        let user_exists =
            timesync_db::repositories::discord::get_discord_user_by_id(&state.db_pool, discord_id)
                .await
                .map_err(TimeError::Database)?
                .is_some();

        if !user_exists {
            // Create user if not exists
            timesync_db::repositories::discord::create_discord_user(
                &state.db_pool,
                discord_id,
                None,
            )
            .await
            .map_err(TimeError::Database)?;
        }

        // Add user to group
        timesync_db::repositories::discord::add_member_to_group(
            &state.db_pool,
            db_discord_group.id,
            discord_id,
        )
        .await
        .map_err(TimeError::Database)?;
    }

    let response = CreateDiscordGroupResponse {
        id: db_discord_group.id,
        name: db_discord_group.name,
        server_id: db_discord_group.server_id,
        role_id: db_discord_group.role_id,
    };

    Ok(Json(response))
}

#[axum::debug_handler]
pub async fn get_discord_group(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<GetDiscordGroupResponse>, AppError> {
    // Get discord group from database
    let db_discord_group =
        timesync_db::repositories::discord::get_discord_group_by_id(&state.db_pool, id)
            .await
            .map_err(TimeError::Database)?
            .ok_or_else(|| TimeError::NotFound(format!("Discord group with ID {} not found", id)))?;

    // Get group members
    let db_members = timesync_db::repositories::discord::get_group_members(&state.db_pool, id)
        .await
        .map_err(TimeError::Database)?;

    // Fetch schedule_id for each member
    let mut members = Vec::new();
    for member in db_members {
        let user =
            timesync_db::repositories::discord::get_discord_user_by_id(
                &state.db_pool,
                &member.discord_id,
            )
            .await
            .map_err(TimeError::Database)?
            .ok_or_else(|| {
                TimeError::NotFound(format!(
                    "Discord user with ID {} not found",
                    member.discord_id
                ))
            })?;

        members.push(DiscordGroupMember {
            discord_id: member.discord_id,
            schedule_id: user.schedule_id,
        });
    }

    let response = GetDiscordGroupResponse {
        id: db_discord_group.id,
        name: db_discord_group.name,
        server_id: db_discord_group.server_id,
        role_id: db_discord_group.role_id,
        members,
    };

    Ok(Json(response))
}

#[axum::debug_handler]
pub async fn update_discord_group(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateDiscordGroupRequest>,
) -> Result<Json<UpdateDiscordGroupResponse>, AppError> {
    // Check if group exists
    let group_exists =
        timesync_db::repositories::discord::get_discord_group_by_id(&state.db_pool, id)
            .await
            .map_err(TimeError::Database)?
            .is_some();

    if !group_exists {
        return Err(AppError(TimeError::NotFound(format!(
            "Discord group with ID {} not found",
            id
        ))));
    }

    // Update group name if provided
    if let Some(name) = &payload.name {
        timesync_db::repositories::discord::update_discord_group(&state.db_pool, id, Some(name), None)
            .await
            .map_err(TimeError::Database)?;
    }

    // Add new members if provided
    if let Some(add_member_ids) = &payload.add_member_ids {
        for discord_id in add_member_ids {
            // Ensure the user exists
            let user_exists = timesync_db::repositories::discord::get_discord_user_by_id(
                &state.db_pool,
                discord_id,
            )
            .await
            .map_err(TimeError::Database)?
            .is_some();

            if !user_exists {
                // Create user if not exists
                timesync_db::repositories::discord::create_discord_user(
                    &state.db_pool,
                    discord_id,
                    None,
                )
                .await
                .map_err(TimeError::Database)?;
            }

            // Add user to group
            timesync_db::repositories::discord::add_member_to_group(&state.db_pool, id, discord_id)
                .await
                .map_err(TimeError::Database)?;
        }
    }

    // Remove members if provided
    if let Some(remove_member_ids) = &payload.remove_member_ids {
        for discord_id in remove_member_ids {
            timesync_db::repositories::discord::remove_member_from_group(
                &state.db_pool,
                id,
                discord_id,
            )
            .await
            .map_err(TimeError::Database)?;
        }
    }

    let response = UpdateDiscordGroupResponse {
        id,
        updated_at: chrono::Utc::now(),
    };

    Ok(Json(response))
}

#[axum::debug_handler]
pub async fn update_discord_group_role(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateDiscordGroupRoleRequest>,
) -> Result<Json<UpdateDiscordGroupRoleResponse>, AppError> {
    // Update the role ID for the group
    // Update the group's role ID
    let _updated_group = timesync_db::repositories::discord::update_group_role_id(
        &state.db_pool,
        id,
        &payload.role_id,
    )
    .await
    .map_err(TimeError::Database)?;

    let response = UpdateDiscordGroupRoleResponse {
        id,
        role_id: payload.role_id,
        updated_at: chrono::Utc::now(),
    };

    Ok(Json(response))
}