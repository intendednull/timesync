use chrono::{DateTime, Utc};
use pretty_assertions::assert_eq;
use rstest::rstest;
use serde_json::{from_str, to_string};
use timesync_core::models::{
    discord::{
        CreateDiscordGroupRequest, CreateDiscordUserRequest, DiscordGroup, DiscordUser,
        GetDiscordGroupResponse, MatchResponse, UpdateDiscordGroupRequest,
    },
    schedule::{
        CreateScheduleRequest, CreateTimeSlotRequest, Schedule, TimeSlotResponse,
        UpdateScheduleRequest, VerifyPasswordRequest,
    },
    time_slot::TimeSlot,
};
use uuid::Uuid;

#[test]
fn test_schedule_serialization() {
    let id = Uuid::new_v4();
    let created_at = Utc::now();
    
    let schedule = Schedule {
        id,
        name: "Test Schedule".to_string(),
        password_hash: Some("hashed_password".to_string()),
        created_at,
    };
    
    let json = to_string(&schedule).expect("Failed to serialize schedule");
    let deserialized: Schedule = from_str(&json).expect("Failed to deserialize schedule");
    
    assert_eq!(deserialized.id, schedule.id);
    assert_eq!(deserialized.name, schedule.name);
    assert_eq!(deserialized.password_hash, schedule.password_hash);
    assert_eq!(deserialized.created_at, schedule.created_at);
}

#[test]
fn test_time_slot_serialization() {
    let id = Uuid::new_v4();
    let schedule_id = Uuid::new_v4();
    let created_at = Utc::now();
    let start_time = Utc::now();
    let end_time = start_time + chrono::Duration::hours(1);
    
    let time_slot = TimeSlot {
        id,
        schedule_id,
        start_time,
        end_time,
        created_at,
    };
    
    let json = to_string(&time_slot).expect("Failed to serialize time slot");
    let deserialized: TimeSlot = from_str(&json).expect("Failed to deserialize time slot");
    
    assert_eq!(deserialized.id, time_slot.id);
    assert_eq!(deserialized.schedule_id, time_slot.schedule_id);
    assert_eq!(deserialized.start_time, time_slot.start_time);
    assert_eq!(deserialized.end_time, time_slot.end_time);
    assert_eq!(deserialized.created_at, time_slot.created_at);
}

#[test]
fn test_discord_user_serialization() {
    let schedule_id = Uuid::new_v4();
    let created_at = Utc::now();
    
    let discord_user = DiscordUser {
        discord_id: "123456789".to_string(),
        schedule_id: Some(schedule_id),
        created_at,
    };
    
    let json = to_string(&discord_user).expect("Failed to serialize discord user");
    let deserialized: DiscordUser = from_str(&json).expect("Failed to deserialize discord user");
    
    assert_eq!(deserialized.discord_id, discord_user.discord_id);
    assert_eq!(deserialized.schedule_id, discord_user.schedule_id);
    assert_eq!(deserialized.created_at, discord_user.created_at);
}

#[test]
fn test_discord_group_serialization() {
    let id = Uuid::new_v4();
    let created_at = Utc::now();
    
    let discord_group = DiscordGroup {
        id,
        name: "Test Group".to_string(),
        server_id: "server123".to_string(),
        role_id: None,
        created_at,
    };
    
    let json = to_string(&discord_group).expect("Failed to serialize discord group");
    let deserialized: DiscordGroup = from_str(&json).expect("Failed to deserialize discord group");
    
    assert_eq!(deserialized.id, discord_group.id);
    assert_eq!(deserialized.name, discord_group.name);
    assert_eq!(deserialized.server_id, discord_group.server_id);
    assert_eq!(deserialized.created_at, discord_group.created_at);
}

#[rstest]
#[case("Test Schedule", None, vec![], None)]
#[case("Work Schedule", Some("password123"), vec![], None)]
#[case(
    "Team Meeting", 
    None, 
    vec![
        CreateTimeSlotRequest {
            start: Utc::now(),
            end: Utc::now() + chrono::Duration::hours(1),
            is_recurring: false,
        }
    ], 
    Some("discord123")
)]
fn test_create_schedule_request(
    #[case] name: &str,
    #[case] password: Option<&str>,
    #[case] slots: Vec<CreateTimeSlotRequest>,
    #[case] discord_id: Option<&str>,
) {
    let request = CreateScheduleRequest {
        name: name.to_string(),
        password: password.map(|p| p.to_string()),
        slots,
        discord_id: discord_id.map(|d| d.to_string()),
        timezone: "UTC".to_string(),
    };
    
    let json = to_string(&request).expect("Failed to serialize create schedule request");
    let deserialized: CreateScheduleRequest = from_str(&json).expect("Failed to deserialize create schedule request");
    
    assert_eq!(deserialized.name, request.name);
    assert_eq!(deserialized.password, request.password);
    assert_eq!(deserialized.slots.len(), request.slots.len());
    assert_eq!(deserialized.discord_id, request.discord_id);
}

#[rstest]
#[case("Discord User 1", Uuid::new_v4())]
#[case("123456789", Uuid::new_v4())]
fn test_create_discord_user_request(#[case] discord_id: &str, #[case] schedule_id: Uuid) {
    let request = CreateDiscordUserRequest {
        discord_id: discord_id.to_string(),
        schedule_id,
    };
    
    let json = to_string(&request).expect("Failed to serialize create discord user request");
    let deserialized: CreateDiscordUserRequest = from_str(&json).expect("Failed to deserialize create discord user request");
    
    assert_eq!(deserialized.discord_id, request.discord_id);
    assert_eq!(deserialized.schedule_id, request.schedule_id);
}

#[test]
fn test_create_discord_group_request() {
    let request = CreateDiscordGroupRequest {
        name: "Test Group".to_string(),
        server_id: "server123".to_string(),
        member_ids: vec!["user1".to_string(), "user2".to_string()],
    };
    
    let json = to_string(&request).expect("Failed to serialize create discord group request");
    let deserialized: CreateDiscordGroupRequest = from_str(&json).expect("Failed to deserialize create discord group request");
    
    assert_eq!(deserialized.name, request.name);
    assert_eq!(deserialized.server_id, request.server_id);
    assert_eq!(deserialized.member_ids, request.member_ids);
}

#[test]
fn test_update_discord_group_request() {
    let request = UpdateDiscordGroupRequest {
        name: Some("Updated Group".to_string()),
        add_member_ids: Some(vec!["user3".to_string()]),
        remove_member_ids: Some(vec!["user1".to_string()]),
    };
    
    let json = to_string(&request).expect("Failed to serialize update discord group request");
    let deserialized: UpdateDiscordGroupRequest = from_str(&json).expect("Failed to deserialize update discord group request");
    
    assert_eq!(deserialized.name, request.name);
    assert_eq!(deserialized.add_member_ids, request.add_member_ids);
    assert_eq!(deserialized.remove_member_ids, request.remove_member_ids);
}

#[test]
fn test_update_schedule_request() {
    let start_time = Utc::now();
    let end_time = start_time + chrono::Duration::hours(1);
    
    let request = UpdateScheduleRequest {
        name: Some("Updated Schedule".to_string()),
        slots: vec![CreateTimeSlotRequest {
            start: start_time,
            end: end_time,
            is_recurring: false,
        }],
        password: Some("password123".to_string()),
        timezone: Some("UTC".to_string()),
    };
    
    let json = to_string(&request).expect("Failed to serialize update schedule request");
    let deserialized: UpdateScheduleRequest = from_str(&json).expect("Failed to deserialize update schedule request");
    
    assert_eq!(deserialized.name, request.name);
    assert_eq!(deserialized.slots.len(), request.slots.len());
    assert_eq!(deserialized.password, request.password);
}

#[test]
fn test_verify_password_request() {
    let request = VerifyPasswordRequest {
        password: "password123".to_string(),
    };
    
    let json = to_string(&request).expect("Failed to serialize verify password request");
    let deserialized: VerifyPasswordRequest = from_str(&json).expect("Failed to deserialize verify password request");
    
    assert_eq!(deserialized.password, request.password);
}

#[test]
fn test_time_slot_response() {
    let start_time: DateTime<Utc> = Utc::now();
    let end_time = start_time + chrono::Duration::hours(1);
    
    let response = TimeSlotResponse {
        start: start_time,
        end: end_time,
        is_recurring: false,
    };
    
    let json = to_string(&response).expect("Failed to serialize time slot response");
    let deserialized: TimeSlotResponse = from_str(&json).expect("Failed to deserialize time slot response");
    
    assert_eq!(deserialized.start, response.start);
    assert_eq!(deserialized.end, response.end);
}

#[test]
fn test_get_discord_group_response() {
    let id = Uuid::new_v4();
    let schedule_id = Uuid::new_v4();
    
    let response = GetDiscordGroupResponse {
        id,
        name: "Test Group".to_string(),
        server_id: "server123".to_string(),
        role_id: None,
        members: vec![
            timesync_core::models::discord::DiscordGroupMember {
                discord_id: "user1".to_string(),
                schedule_id: Some(schedule_id),
            },
            timesync_core::models::discord::DiscordGroupMember {
                discord_id: "user2".to_string(),
                schedule_id: None,
            },
        ],
    };
    
    let json = to_string(&response).expect("Failed to serialize get discord group response");
    let deserialized: GetDiscordGroupResponse = from_str(&json).expect("Failed to deserialize get discord group response");
    
    assert_eq!(deserialized.id, response.id);
    assert_eq!(deserialized.name, response.name);
    assert_eq!(deserialized.server_id, response.server_id);
    assert_eq!(deserialized.members.len(), response.members.len());
    assert_eq!(deserialized.members[0].discord_id, response.members[0].discord_id);
    assert_eq!(deserialized.members[0].schedule_id, response.members[0].schedule_id);
    assert_eq!(deserialized.members[1].discord_id, response.members[1].discord_id);
    assert_eq!(deserialized.members[1].schedule_id, response.members[1].schedule_id);
}

#[test]
fn test_match_response() {
    let start_time = Utc::now();
    let end_time = start_time + chrono::Duration::hours(1);
    let group_id = Uuid::new_v4();
    
    let response = MatchResponse {
        matches: vec![
            timesync_core::models::discord::MatchResult {
                start: start_time,
                end: end_time,
                groups: vec![
                    timesync_core::models::discord::MatchGroupResult {
                        id: group_id,
                        name: "Test Group".to_string(),
                        available_users: vec!["user1".to_string(), "user2".to_string()],
                        count: 2,
                    },
                ],
            },
        ],
    };
    
    let json = to_string(&response).expect("Failed to serialize match response");
    let deserialized: MatchResponse = from_str(&json).expect("Failed to deserialize match response");
    
    assert_eq!(deserialized.matches.len(), response.matches.len());
    assert_eq!(deserialized.matches[0].start, response.matches[0].start);
    assert_eq!(deserialized.matches[0].end, response.matches[0].end);
    assert_eq!(deserialized.matches[0].groups.len(), response.matches[0].groups.len());
    assert_eq!(deserialized.matches[0].groups[0].id, response.matches[0].groups[0].id);
    assert_eq!(deserialized.matches[0].groups[0].name, response.matches[0].groups[0].name);
    assert_eq!(
        deserialized.matches[0].groups[0].available_users,
        response.matches[0].groups[0].available_users
    );
    assert_eq!(deserialized.matches[0].groups[0].count, response.matches[0].groups[0].count);
}