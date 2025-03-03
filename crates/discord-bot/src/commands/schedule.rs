use serenity::{
    builder::CreateApplicationCommand,
    model::application::command::CommandOptionType,
};

/// Create command for generating a new schedule
pub fn create_schedule_command() -> CreateApplicationCommand {
    let mut command = CreateApplicationCommand::default();
    command
        .name("schedule")
        .description("Create a new availability schedule")
        .create_option(|option| {
            option
                .name("create")
                .description("Create a new availability schedule")
                .kind(CommandOptionType::SubCommand)
        })
        .dm_permission(false);
    
    command
}

/// Create command for managing groups
pub fn group_command() -> CreateApplicationCommand {
    let mut command = CreateApplicationCommand::default();
    command
        .name("group")
        .description("Manage scheduling groups")
        .dm_permission(false)
        // Create subcommand
        .create_option(|option| {
            option
                .name("create")
                .description("Create a new group of users")
                .kind(CommandOptionType::SubCommand)
                .create_sub_option(|sub_option| {
                    sub_option
                        .name("name")
                        .description("Name of the group")
                        .kind(CommandOptionType::String)
                        .required(true)
                })
                .create_sub_option(|sub_option| {
                    sub_option
                        .name("members")
                        .description("Comma-separated list of mention tags (@user1, @user2) or 'all' for everyone in the current thread")
                        .kind(CommandOptionType::String)
                        .required(true)
                })
        })
        // List subcommand
        .create_option(|option| {
            option
                .name("list")
                .description("List all groups in this server")
                .kind(CommandOptionType::SubCommand)
        })
        // Add subcommand
        .create_option(|option| {
            option
                .name("add")
                .description("Add users to an existing group")
                .kind(CommandOptionType::SubCommand)
                .create_sub_option(|sub_option| {
                    sub_option
                        .name("name")
                        .description("Name of the group")
                        .kind(CommandOptionType::String)
                        .required(true)
                })
                .create_sub_option(|sub_option| {
                    sub_option
                        .name("members")
                        .description("Comma-separated list of mention tags (@user1, @user2)")
                        .kind(CommandOptionType::String)
                        .required(true)
                })
        })
        // Remove subcommand
        .create_option(|option| {
            option
                .name("remove")
                .description("Remove users from an existing group")
                .kind(CommandOptionType::SubCommand)
                .create_sub_option(|sub_option| {
                    sub_option
                        .name("name")
                        .description("Name of the group")
                        .kind(CommandOptionType::String)
                        .required(true)
                })
                .create_sub_option(|sub_option| {
                    sub_option
                        .name("members")
                        .description("Comma-separated list of mention tags (@user1, @user2)")
                        .kind(CommandOptionType::String)
                        .required(true)
                })
        })
        // Info subcommand
        .create_option(|option| {
            option
                .name("info")
                .description("Show information about a group")
                .kind(CommandOptionType::SubCommand)
                .create_sub_option(|sub_option| {
                    sub_option
                        .name("name")
                        .description("Name of the group")
                        .kind(CommandOptionType::String)
                        .required(true)
                })
        });
    
    command
}

/// Create command for matching available times
pub fn match_command() -> CreateApplicationCommand {
    let mut command = CreateApplicationCommand::default();
    command
        .name("match")
        .description("Find common available times between groups")
        .dm_permission(false)
        .create_option(|option| {
            option
                .name("groups")
                .description("Comma-separated list of group names")
                .kind(CommandOptionType::String)
                .required(true)
        })
        .create_option(|option| {
            option
                .name("min_per_group")
                .description("Minimum number of users required from each group")
                .kind(CommandOptionType::Integer)
                .required(false)
        })
        .create_option(|option| {
            option
                .name("slot_duration")
                .description("Duration of each time slot in minutes (default: 120)")
                .kind(CommandOptionType::Integer)
                .required(false)
        })
        .create_option(|option| {
            option
                .name("max_days")
                .description("Maximum number of days to display (1-7, default: 7)")
                .kind(CommandOptionType::Integer)
                .required(false)
        })
        .create_option(|option| {
            option
                .name("time_span")
                .description("Human-friendly time span (e.g., 'next 3 days', 'this weekend')")
                .kind(CommandOptionType::String)
                .required(false)
        });
    
    command
}

/// Create command for setting the server timezone
pub fn timezone_command() -> CreateApplicationCommand {
    let mut command = CreateApplicationCommand::default();
    command
        .name("timezone")
        .description("Manage server timezone settings")
        .dm_permission(false)
        // Set subcommand
        .create_option(|option| {
            option
                .name("set")
                .description("Set the default timezone for this server")
                .kind(CommandOptionType::SubCommand)
                .create_sub_option(|sub_option| {
                    sub_option
                        .name("timezone")
                        .description("Timezone name (e.g., 'America/New_York', 'Europe/London', 'Asia/Tokyo')")
                        .kind(CommandOptionType::String)
                        .required(true)
                })
        })
        // Show subcommand
        .create_option(|option| {
            option
                .name("show")
                .description("Show the current server timezone")
                .kind(CommandOptionType::SubCommand)
        })
        // List subcommand
        .create_option(|option| {
            option
                .name("list")
                .description("List available timezone options")
                .kind(CommandOptionType::SubCommand)
        });
    
    command
}