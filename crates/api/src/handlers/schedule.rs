use axum::{extract::{Path, State}, Json};
use chrono::Utc;
use eyre::Result;
use std::sync::Arc;
use timesync_core::{
    errors::TimeError,
    models::schedule::{
        CreateScheduleRequest, CreateScheduleResponse, GetScheduleResponse, TimeSlotResponse,
        UpdateScheduleRequest, UpdateScheduleResponse, VerifyPasswordRequest, VerifyPasswordResponse,
    },
};
use uuid::Uuid;

use crate::{middleware::{auth, error_handling::AppError}, ApiState};

#[axum::debug_handler]
pub async fn create_schedule(
    State(state): State<Arc<ApiState>>,
    Json(payload): Json<CreateScheduleRequest>,
) -> Result<Json<CreateScheduleResponse>, AppError> {
    // Hash password if provided
    let password_hash = match &payload.password {
        Some(password) => Some(auth::hash_password(password)?),
        None => None,
    };

    // Create schedule in database
    let db_schedule = timesync_db::repositories::schedule::create_schedule(
        &state.db_pool,
        &payload.name,
        password_hash.as_deref(),
        &payload.timezone,
    )
    .await
    .map_err(TimeError::Database)?;

    // Create time slots if provided
    for slot in &payload.slots {
        timesync_db::repositories::time_slot::create_time_slot(
            &state.db_pool,
            db_schedule.id,
            slot.start,
            slot.end,
            slot.is_recurring,
        )
        .await
        .map_err(TimeError::Database)?;
    }

    // If discord_id is provided, associate schedule with Discord user
    if let Some(discord_id) = &payload.discord_id {
        timesync_db::repositories::discord::create_discord_user(
            &state.db_pool,
            discord_id,
            Some(db_schedule.id),
        )
        .await
        .map_err(TimeError::Database)?;
    }

    let response = CreateScheduleResponse {
        id: db_schedule.id,
        name: db_schedule.name,
        created_at: db_schedule.created_at,
        is_editable: db_schedule.password_hash.is_some(),
        timezone: db_schedule.timezone,
    };

    Ok(Json(response))
}

#[axum::debug_handler]
pub async fn get_schedule(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<GetScheduleResponse>, AppError> {
    // Get schedule from database
    let db_schedule = timesync_db::repositories::schedule::get_schedule_by_id(&state.db_pool, id)
        .await
        .map_err(TimeError::Database)?
        .ok_or_else(|| TimeError::NotFound(format!("Schedule with ID {} not found", id)))?;

    // Get time slots for schedule
    let time_slots = timesync_db::repositories::time_slot::get_time_slots_by_schedule_id(
        &state.db_pool,
        id,
    )
    .await
    .map_err(TimeError::Database)?;

    let response = GetScheduleResponse {
        id: db_schedule.id,
        name: db_schedule.name,
        created_at: db_schedule.created_at,
        is_editable: db_schedule.password_hash.is_some(),
        timezone: db_schedule.timezone,
        slots: time_slots
            .into_iter()
            .map(|slot| TimeSlotResponse {
                start: slot.start_time,
                end: slot.end_time,
                is_recurring: slot.is_recurring,
            })
            .collect(),
    };

    Ok(Json(response))
}

#[axum::debug_handler]
pub async fn update_schedule(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateScheduleRequest>,
) -> Result<Json<UpdateScheduleResponse>, AppError> {
    // Verify password if provided
    if let Some(password) = &payload.password {
        let is_valid = auth::verify_schedule_password(&state.db_pool, id, password)
            .await
            .map_err(TimeError::Database)?;

        if !is_valid {
            return Err(AppError(TimeError::Authentication("Invalid password".to_string())));
        }
    } else {
        // Check if schedule is password-protected
        let db_schedule =
            timesync_db::repositories::schedule::get_schedule_by_id(&state.db_pool, id)
                .await
                .map_err(TimeError::Database)?
                .ok_or_else(|| {
                    TimeError::NotFound(format!("Schedule with ID {} not found", id))
                })?;

        if db_schedule.password_hash.is_some() {
            return Err(AppError(TimeError::Authentication(
                "Password required to update this schedule".to_string(),
            )));
        }
    }

    // Update schedule name and timezone if provided
    if let Some(name) = &payload.name {
        timesync_db::repositories::schedule::update_schedule(
            &state.db_pool, 
            id, 
            Some(name),
            payload.timezone.as_deref(),
        )
        .await
        .map_err(TimeError::Database)?;
    } else if payload.timezone.is_some() {
        // Update just the timezone if name is not provided
        timesync_db::repositories::schedule::update_schedule(
            &state.db_pool, 
            id, 
            None,
            payload.timezone.as_deref(),
        )
        .await
        .map_err(TimeError::Database)?;
    }

    // Update time slots
    // First, delete existing time slots
    timesync_db::repositories::time_slot::delete_time_slots_by_schedule_id(&state.db_pool, id)
        .await
        .map_err(TimeError::Database)?;

    // Then, create new time slots
    for slot in &payload.slots {
        timesync_db::repositories::time_slot::create_time_slot(
            &state.db_pool,
            id,
            slot.start,
            slot.end,
            slot.is_recurring,
        )
        .await
        .map_err(TimeError::Database)?;
    }

    let response = UpdateScheduleResponse {
        id,
        updated_at: Utc::now(),
    };

    Ok(Json(response))
}

#[axum::debug_handler]
pub async fn verify_password(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<Uuid>,
    Json(payload): Json<VerifyPasswordRequest>,
) -> Result<Json<VerifyPasswordResponse>, AppError> {
    let is_valid = auth::verify_schedule_password(&state.db_pool, id, &payload.password)
        .await
        .map_err(TimeError::Database)?;

    let response = VerifyPasswordResponse { valid: is_valid };

    Ok(Json(response))
}