use chrono::{DateTime, Utc};
use mockall::mock;
use uuid::Uuid;

use crate::models::{DbDiscordGroup, DbDiscordUser, DbGroupMember, DbSchedule, DbTimeSlot};

// Mock repositories for testing
mock! {
    pub ScheduleRepo {
        pub async fn create_schedule(
            &self,
            name: &'static str,
            password_hash: Option<&'static str>,
        ) -> eyre::Result<DbSchedule>;

        pub async fn get_schedule_by_id(
            &self,
            id: Uuid,
        ) -> eyre::Result<Option<DbSchedule>>;

        pub async fn update_schedule(
            &self,
            id: Uuid,
            name: Option<&'static str>,
        ) -> eyre::Result<DbSchedule>;

        pub async fn verify_password(
            &self,
            id: Uuid,
            password: &'static str,
        ) -> eyre::Result<bool>;
    }
}

mock! {
    pub TimeSlotRepo {
        pub async fn create_time_slot(
            &self,
            schedule_id: Uuid,
            start_time: DateTime<Utc>,
            end_time: DateTime<Utc>,
        ) -> eyre::Result<DbTimeSlot>;

        pub async fn get_time_slots_by_schedule_id(
            &self,
            schedule_id: Uuid,
        ) -> eyre::Result<Vec<DbTimeSlot>>;

        pub async fn delete_time_slots_by_schedule_id(
            &self,
            schedule_id: Uuid,
        ) -> eyre::Result<()>;
    }
}

mock! {
    pub DiscordUserRepo {
        pub async fn create_discord_user(
            &self,
            discord_id: &'static str,
            schedule_id: Option<Uuid>,
        ) -> eyre::Result<DbDiscordUser>;

        pub async fn get_discord_user_by_id(
            &self,
            discord_id: &'static str,
        ) -> eyre::Result<Option<DbDiscordUser>>;
    }
}

mock! {
    pub DiscordGroupRepo {
        pub async fn create_discord_group(
            &self,
            name: &'static str,
            server_id: &'static str,
        ) -> eyre::Result<DbDiscordGroup>;

        pub async fn get_discord_group_by_id(
            &self,
            id: Uuid,
        ) -> eyre::Result<Option<DbDiscordGroup>>;

        pub async fn update_discord_group(
            &self,
            id: Uuid,
            name: Option<&'static str>,
        ) -> eyre::Result<DbDiscordGroup>;

        pub async fn add_member_to_group(
            &self,
            group_id: Uuid,
            discord_id: &'static str,
        ) -> eyre::Result<DbGroupMember>;

        pub async fn remove_member_from_group(
            &self,
            group_id: Uuid,
            discord_id: &'static str,
        ) -> eyre::Result<()>;

        pub async fn get_group_members(
            &self,
            group_id: Uuid,
        ) -> eyre::Result<Vec<DbGroupMember>>;

        pub async fn get_user_groups(
            &self,
            discord_id: &'static str,
        ) -> eyre::Result<Vec<DbDiscordGroup>>;
    }
}