use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordUser {
    pub discord_id: String,
    pub schedule_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDiscordUserRequest {
    pub discord_id: String,
    pub schedule_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDiscordUserResponse {
    pub discord_id: String,
    pub schedule_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetDiscordUserResponse {
    pub discord_id: String,
    pub schedule_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordGroup {
    pub id: Uuid,
    pub name: String,
    pub server_id: String,
    pub role_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDiscordGroupRequest {
    pub name: String,
    pub server_id: String,
    pub member_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDiscordGroupResponse {
    pub id: Uuid,
    pub name: String,
    pub server_id: String,
    pub role_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetDiscordGroupResponse {
    pub id: Uuid,
    pub name: String,
    pub server_id: String,
    pub role_id: Option<String>,
    pub members: Vec<DiscordGroupMember>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordGroupMember {
    pub discord_id: String,
    pub schedule_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDiscordGroupRequest {
    pub name: Option<String>,
    pub add_member_ids: Option<Vec<String>>,
    pub remove_member_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDiscordGroupResponse {
    pub id: Uuid,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDiscordGroupRoleRequest {
    pub role_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDiscordGroupRoleResponse {
    pub id: Uuid,
    pub role_id: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchRequest {
    pub group_ids: Vec<Uuid>,
    pub min_per_group: Option<usize>,
    pub count: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResponse {
    pub matches: Vec<MatchResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResult {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub groups: Vec<MatchGroupResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchGroupResult {
    pub id: Uuid,
    pub name: String,
    pub available_users: Vec<String>,
    pub count: usize,
}