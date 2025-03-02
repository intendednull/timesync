//! # Availability Handlers
//!
//! This module contains handlers for working with user availability and matching
//! optimal meeting times. It includes functionality for finding overlapping
//! availability across multiple Discord groups.
//!
//! ## Availability Matching Algorithm
//!
//! The core algorithm in this module finds optimal meeting times where multiple groups
//! of users can meet simultaneously. It works by:
//!
//! 1. Collecting all unique time slots from all users' schedules
//! 2. For each time slot, checking how many users from each group are available
//! 3. Keeping only time slots where at least a minimum number of users from each group are available
//! 4. Ranking and returning the top matches based on specified criteria
//!
//! The algorithm is optimized to minimize database queries by:
//! - Caching all time slots by schedule ID to avoid duplicate queries
//! - Performing efficient set operations for availability calculations
//! - Early filtering of time slots that cannot possibly match all criteria

use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use std::{collections::HashMap, sync::Arc};
use timesync_core::{
    errors::TimeError,
    models::discord::{MatchGroupResult, MatchResponse, MatchResult},
};
use uuid::Uuid;

use crate::{ApiState, middleware::error_handling::AppError};

/// Query parameters for the match availability endpoint
///
/// This struct defines the parameters that can be provided when searching
/// for matching availability across multiple Discord groups.
///
/// # Fields
///
/// * `group_ids` - Comma-separated list of Discord group UUIDs
/// * `min_per_group` - Minimum number of available users per group (default: 1)
/// * `count` - Maximum number of matching time slots to return (default: 5)
#[derive(Debug, Deserialize)]
pub struct MatchQuery {
    /// Comma-separated list of Discord group UUIDs to match
    pub group_ids: String,
    
    /// Minimum number of available users required from each group
    pub min_per_group: Option<usize>,
    
    /// Maximum number of matching time slots to return
    pub count: Option<usize>,
}

/// Finds optimal meeting times across multiple Discord groups
///
/// This handler analyzes the availability of users across multiple Discord groups
/// and identifies time slots where at least a minimum number of users from each
/// group are available.
///
/// # Endpoint
///
/// ```
/// GET /availability/match?group_ids=uuid1,uuid2&min_per_group=2&count=5
/// ```
///
/// # Algorithm
///
/// The matching algorithm follows these steps:
/// 
/// 1. Input Validation & Preparation:
///    - Validate group IDs and convert to UUIDs
///    - Set default parameters (min_per_group=1, count=5)
///    - Retrieve all group information from database
///
/// 2. Data Collection:
///    - Collect all users from each group
///    - Get schedule IDs for each user
///    - Fetch all time slots for each schedule (with caching)
/// 
/// 3. Time Slot Collection & De-duplication:
///    - Collect all unique time slots from all users' schedules
///    - Sort by start time and remove duplicates
/// 
/// 4. Availability Analysis:
///    - For each time slot:
///      - For each group:
///        - Count users whose availability contains the time slot
///        - Include group if it meets minimum user requirement
///      - Include time slot if all groups meet requirements
/// 
/// 5. Result Preparation:
///    - Sort matches by start time
///    - Limit to requested count
///    - Format and return response
///
/// # Time Complexity:
/// - O(U × S) where U is the number of users and S is the average number of slots per user
/// - Space complexity is O(S_total) where S_total is the total number of unique time slots
///
/// # Parameters
///
/// * `state` - Application state containing the database connection
/// * `query` - Query parameters specifying groups and matching criteria
///
/// # Returns
///
/// * `Result<Json<MatchResponse>, AppError>` - JSON response with matching time slots,
///   or an error if the operation fails
///
/// # Errors
///
/// * `TimeError::Validation` - Invalid group IDs or empty group list
/// * `TimeError::NotFound` - Group or user not found
/// * `TimeError::Database` - Database error
#[axum::debug_handler]
pub async fn match_availability(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<MatchQuery>,
) -> Result<Json<MatchResponse>, AppError> {
    // STEP 1: Input Validation & Preparation
    
    // Parse comma-separated group IDs into UUIDs
    let group_ids: Result<Vec<Uuid>, _> = query
        .group_ids
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(Uuid::parse_str)
        .collect();

    // Return validation error if any group ID is invalid
    let group_ids = group_ids.map_err(|_| {
        AppError(TimeError::Validation("Invalid group ID format. Must be comma-separated UUIDs".to_string()))
    })?;

    // Return validation error if no group IDs are provided
    if group_ids.is_empty() {
        return Err(AppError(TimeError::Validation(
            "At least one group ID must be provided".to_string(),
        )));
    }

    // Set default values for optional parameters
    let min_per_group = query.min_per_group.unwrap_or(1);
    let count = query.count.unwrap_or(5);

    // STEP 2: Data Collection
    
    // Retrieve and validate all groups
    let mut groups = HashMap::new();
    for group_id in &group_ids {
        let group = timesync_db::repositories::discord::get_discord_group_by_id(&state.db_pool, *group_id)
            .await
            .map_err(TimeError::Database)?
            .ok_or_else(|| {
                TimeError::NotFound(format!("Discord group with ID {} not found", group_id))
            })?;
        groups.insert(*group_id, group);
    }

    // Retrieve all members and their schedules for each group
    let mut group_schedules = HashMap::new();
    for group_id in &group_ids {
        let members = timesync_db::repositories::discord::get_group_members(&state.db_pool, *group_id)
            .await
            .map_err(TimeError::Database)?;

        let mut schedule_ids = Vec::new();
        for member in members {
            let user = timesync_db::repositories::discord::get_discord_user_by_id(
                &state.db_pool,
                &member.discord_id,
            )
            .await
            .map_err(|e| AppError(TimeError::Database(e)))?
            .ok_or_else(|| {
                TimeError::NotFound(format!("Discord user with ID {} not found", member.discord_id))
            })?;

            // Only include users who have associated schedules
            if let Some(schedule_id) = user.schedule_id {
                schedule_ids.push((member.discord_id, schedule_id));
            }
        }
        group_schedules.insert(*group_id, schedule_ids);
    }

    // Fetch all time slots for each schedule (caching to avoid duplicate queries)
    let mut schedule_time_slots = HashMap::new();
    for schedules in group_schedules.values() {
        for (_discord_id, schedule_id) in schedules {
            if !schedule_time_slots.contains_key(schedule_id) {
                let time_slots = timesync_db::repositories::time_slot::get_time_slots_by_schedule_id(
                    &state.db_pool,
                    *schedule_id,
                )
                .await
                .map_err(TimeError::Database)?;
                schedule_time_slots.insert(*schedule_id, time_slots);
            }
        }
    }

    // STEP 3: Time Slot Collection & De-duplication
    
    // Extract all unique time slot boundaries
    let mut time_boundaries = Vec::new();
    for slots in schedule_time_slots.values() {
        for slot in slots {
            time_boundaries.push(slot.start_time);
            time_boundaries.push(slot.end_time);
        }
    }
    
    // Sort and deduplicate time boundaries
    time_boundaries.sort();
    time_boundaries.dedup();
    
    // If we have fewer than 2 time boundaries, no valid time slots exist
    if time_boundaries.len() < 2 {
        return Ok(Json(MatchResponse { matches: Vec::new() }));
    }
    
    // Create potential time slots from adjacent boundaries
    let mut potential_time_slots = Vec::new();
    for window in time_boundaries.windows(2) {
        let start = window[0];
        let end = window[1];
        
        // Only consider non-zero duration slots
        if start < end {
            potential_time_slots.push((start, end));
        }
    }
    
    // STEP 4: Availability Analysis
    
    // Find matching time slots
    let mut matches = Vec::new();
    
    for (start, end) in potential_time_slots {
        // Skip if this slot is zero duration or invalid
        if start >= end {
            continue;
        }
        
        let mut all_groups_match = true;
        let mut match_groups = Vec::new();
        
        // For each group, find users available during this time slot
        for &group_id in &group_ids {
            let schedules = group_schedules.get(&group_id).unwrap();
            let group = groups.get(&group_id).unwrap();
            
            let mut available_users = Vec::new();
            
            // Check each user's availability
            for (discord_id, schedule_id) in schedules {
                let slots = schedule_time_slots.get(schedule_id).unwrap();
                
                // User is available if any of their slots contain this time period
                // A time slot contains another if: slot.start <= target.start && slot.end >= target.end
                let is_available = slots.iter().any(|slot| {
                    slot.start_time <= start && slot.end_time >= end
                });
                
                if is_available {
                    available_users.push(discord_id.clone());
                }
            }
            
            // Check if this group meets the minimum requirement
            if available_users.len() >= min_per_group {
                let count = available_users.len();
                match_groups.push(MatchGroupResult {
                    id: group_id,
                    name: group.name.clone(),
                    available_users,
                    count,
                });
            } else {
                // If any group doesn't have enough users, this time slot is invalid
                all_groups_match = false;
                break;
            }
        }
        
        // Only include match if all groups have at least min_per_group users available
        if all_groups_match {
            matches.push(MatchResult {
                start,
                end,
                groups: match_groups,
            });
        }
    }
    
    // STEP 5: Result Preparation
    
    // Sort matches by start time
    matches.sort_by(|a, b| a.start.cmp(&b.start));
    
    // Limit to requested count
    if matches.len() > count {
        matches.truncate(count);
    }
    
    // Build and return response
    let response = MatchResponse { matches };
    Ok(Json(response))
}

