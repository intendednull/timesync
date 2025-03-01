use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    pub id: Uuid,
    pub name: String,
    pub password_hash: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateScheduleRequest {
    pub name: String,
    pub password: Option<String>,
    #[serde(default)]
    pub slots: Vec<CreateTimeSlotRequest>,
    pub discord_id: Option<String>,
    #[serde(default = "default_timezone")]
    pub timezone: String,
}

fn default_timezone() -> String {
    "UTC".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTimeSlotRequest {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    #[serde(default)]
    pub is_recurring: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateScheduleResponse {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub is_editable: bool,
    pub timezone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetScheduleResponse {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub is_editable: bool,
    pub timezone: String,
    pub slots: Vec<TimeSlotResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSlotResponse {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub is_recurring: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateScheduleRequest {
    pub name: Option<String>,
    pub slots: Vec<CreateTimeSlotRequest>,
    pub password: Option<String>,
    pub timezone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateScheduleResponse {
    pub id: Uuid,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyPasswordRequest {
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyPasswordResponse {
    pub valid: bool,
}