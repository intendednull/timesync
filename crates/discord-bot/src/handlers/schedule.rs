use chrono::{Utc, Offset};
use eyre::Result;
use serenity::{
    model::{
        application::interaction::{
            application_command::ApplicationCommandInteraction, 
            message_component::MessageComponentInteraction,
            InteractionResponseType
        }, 
        application::interaction::application_command::CommandDataOption,
    },
    utils::Color,
};
use timesync_core::models::discord::{CreateDiscordGroupRequest, CreateDiscordGroupResponse};
use std::collections::HashMap;
use std::str::FromStr;
use sqlx::Row;

use crate::handlers::HandlerContext;

/// Handle the /schedule command
pub async fn handle_schedule_command(
    ctx: HandlerContext, 
    command: &ApplicationCommandInteraction
) -> Result<()> {
    // Get the subcommand
    let subcommand = command.data.options.first()
        .ok_or_else(|| eyre::eyre!("Missing subcommand"))?;
    
    match subcommand.name.as_str() {
        "create" => handle_schedule_create(ctx, command).await,
        _ => {
            command.create_interaction_response(&ctx.ctx.http, |r| {
                r.kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|m| {
                        m.content("Unknown subcommand").ephemeral(true)
                    })
            }).await?;
            
            Ok(())
        }
    }
}

/// Handle the /schedule create subcommand
async fn handle_schedule_create(
    ctx: HandlerContext, 
    command: &ApplicationCommandInteraction
) -> Result<()> {
    // Get the user's Discord ID
    let discord_id = command.user.id.to_string();
    
    // Generate a unique URL for schedule creation
    let schedule_url = format!("{}/create?discord_id={}", ctx.config.web_base_url, discord_id);
    
    // Send a response with the URL
    command.create_interaction_response(&ctx.ctx.http, |r| {
        r.kind(InteractionResponseType::ChannelMessageWithSource)
            .interaction_response_data(|m| {
                m.embed(|e| {
                    e.title("Create Your Availability Schedule")
                        .description(format!(
                            "Click the link below to create your availability schedule:\n\n[Create Schedule]({})", 
                            schedule_url
                        ))
                        .color(Color::DARK_GREEN)
                        .footer(|f| f.text("Your schedule will be linked to your Discord account"))
                })
            })
    }).await?;
    
    Ok(())
}

/// Handle the /group command
pub async fn handle_group_command(
    ctx: HandlerContext, 
    command: &ApplicationCommandInteraction
) -> Result<()> {
    // Get the subcommand
    let subcommand = command.data.options.first()
        .ok_or_else(|| eyre::eyre!("Missing subcommand"))?;
    
    match subcommand.name.as_str() {
        "create" => handle_group_create(ctx, command, subcommand).await,
        "list" => handle_group_list(ctx, command).await,
        "add" => handle_group_add(ctx, command, subcommand).await,
        "remove" => handle_group_remove(ctx, command, subcommand).await,
        "info" => handle_group_info(ctx, command, subcommand).await,
        _ => {
            command.create_interaction_response(&ctx.ctx.http, |r| {
                r.kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|m| {
                        m.content("Unknown subcommand").ephemeral(true)
                    })
            }).await?;
            
            Ok(())
        }
    }
}

/// Handle the /group create subcommand
async fn handle_group_create(
    ctx: HandlerContext,
    command: &ApplicationCommandInteraction,
    subcommand: &CommandDataOption,
) -> Result<()> {
    // Get the name option
    let name = get_option_string(subcommand, "name")?;
    
    // Get the members option
    let members_str = get_option_string(subcommand, "members")?;
    
    // Process the members list
    let member_ids = if members_str.trim().to_lowercase() == "all" {
        // If "all" is specified, add all users in the current thread
        // This would require more complex logic to fetch all users from the channel
        // For simplicity, we'll just add the caller
        vec![command.user.id.to_string()]
    } else {
        // Parse the mention tags
        parse_mention_tags(&members_str)
    };
    
    if member_ids.is_empty() {
        command.create_interaction_response(&ctx.ctx.http, |r| {
            r.kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|m| {
                    m.content("No valid members specified").ephemeral(true)
                })
        }).await?;
        
        return Ok(());
    }
    
    // Get the guild (server) ID
    let guild_id = command.guild_id
        .ok_or_else(|| eyre::eyre!("Command must be used in a server"))?;
    
    // Create the request payload
    let request = CreateDiscordGroupRequest {
        name: name.clone(),
        server_id: guild_id.to_string(),
        member_ids: member_ids.clone(),
    };
    
    // Make API request to create the group
    let client = reqwest::Client::new();
    let response = client.post(format!("{}/api/discord/groups", ctx.config.web_base_url))
        .json(&request)
        .send()
        .await?;
    
    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(eyre::eyre!("Failed to create group: {}", error_text));
    }
    
    // Parse the response to get the group ID
    let group_response: CreateDiscordGroupResponse = response.json().await?;
    
    // Create a Discord role for the group
    let guild = command.guild_id.unwrap().to_guild_cached(&ctx.ctx)
        .ok_or_else(|| eyre::eyre!("Failed to get guild"))?;
    
    // Create the role (using just the group name)
    let role = guild.create_role(&ctx.ctx.http, |r| {
        r.name(name.clone())
         .colour(0x3498db) // Blue color
         .hoist(false)     // Don't display separately
         .mentionable(true)
    }).await?;
    
    // Store the role ID in the database
    client.put(format!("{}/api/discord/groups/{}/role", ctx.config.web_base_url, group_response.id))
        .json(&serde_json::json!({ "role_id": role.id.to_string() }))
        .send()
        .await?;
    
    // Assign the role to all members in the group
    for member_id in &member_ids {
        if let Ok(user_id) = member_id.parse::<u64>() {
            if let Ok(mut member) = guild.member(&ctx.ctx.http, user_id).await {
                // Ignore errors when adding roles to allow the process to continue
                let _ = member.add_role(&ctx.ctx.http, role.id).await;
            }
        }
    }
    
    // Respond to the interaction
    command.create_interaction_response(&ctx.ctx.http, |r| {
        r.kind(InteractionResponseType::ChannelMessageWithSource)
            .interaction_response_data(|m| {
                m.embed(|e| {
                    e.title("Scheduling Group Created")
                        .description(format!("Group **{}** has been created with {} members", name, member_ids.len()))
                        .field("Members", format_member_list(&member_ids), false)
                        .field("Discord Role", format!("<@&{}>", role.id), false)
                        .color(Color::DARK_GREEN)
                        .timestamp(Utc::now().to_rfc3339())
                })
            })
    }).await?;
    
    Ok(())
}

/// Handle the /group list subcommand
async fn handle_group_list(
    ctx: HandlerContext,
    command: &ApplicationCommandInteraction,
) -> Result<()> {
    // Get the guild (server) ID
    let server_id = command.guild_id
        .ok_or_else(|| eyre::eyre!("Command must be used in a server"))?
        .to_string();
    
    // Query the database for groups in this server
    let groups = sqlx::query!(
        "SELECT id, name FROM discord_groups WHERE server_id = $1 ORDER BY name",
        server_id
    )
    .fetch_all(&ctx.db_pool)
    .await?;
    
    if groups.is_empty() {
        command.create_interaction_response(&ctx.ctx.http, |r| {
            r.kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|m| {
                    m.content("No groups have been created in this server yet").ephemeral(true)
                })
        }).await?;
        
        return Ok(());
    }
    
    // Create a description of all groups
    let mut description = String::from("Here are all the scheduling groups in this server:\n\n");
    
    for group in groups {
        // Count members in each group
        let count = sqlx::query!(
            "SELECT COUNT(*) FROM group_members WHERE group_id = $1",
            group.id
        )
        .fetch_one(&ctx.db_pool)
        .await?
        .count
        .unwrap_or(0) as i64;
        
        // Get the role ID for this group
        let role_id: Option<String> = sqlx::query(
            "SELECT role_id FROM discord_groups WHERE id = $1"
        )
        .bind(group.id)
        .fetch_optional(&ctx.db_pool)
        .await?
        .and_then(|row| row.try_get("role_id").ok())
        .flatten();
        
        let role_mention = role_id.map(|id| format!(" <@&{}>", id)).unwrap_or_default();
        
        description.push_str(&format!("**{}**{} - {} {}\n", 
            group.name, 
            role_mention,
            count,
            if count == 1 { "member" } else { "members" }
        ));
    }
    
    // Respond to the interaction
    command.create_interaction_response(&ctx.ctx.http, |r| {
        r.kind(InteractionResponseType::ChannelMessageWithSource)
            .interaction_response_data(|m| {
                m.embed(|e| {
                    e.title("Scheduling Groups")
                        .description(description)
                        .color(Color::BLUE)
                        .footer(|f| f.text("Use /group info <n> to see details about a specific group"))
                })
            })
    }).await?;
    
    Ok(())
}

/// Handle the /group add subcommand
async fn handle_group_add(
    ctx: HandlerContext,
    command: &ApplicationCommandInteraction,
    subcommand: &CommandDataOption,
) -> Result<()> {
    // Get the name option
    let name = get_option_string(subcommand, "name")?;
    
    // Get the members option
    let members_str = get_option_string(subcommand, "members")?;
    
    // Get the guild
    let guild_id = command.guild_id
        .ok_or_else(|| eyre::eyre!("Command must be used in a server"))?;
    
    // Parse the mention tags
    let member_ids = parse_mention_tags(&members_str);
    
    if member_ids.is_empty() {
        command.create_interaction_response(&ctx.ctx.http, |r| {
            r.kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|m| {
                    m.content("No valid members specified").ephemeral(true)
                })
        }).await?;
        
        return Ok(());
    }
    
    // Find the group by name in this server using basic query to avoid sqlx compile-time checks
    let group_row = sqlx::query(
        "SELECT id, role_id FROM discord_groups WHERE name = $1 AND server_id = $2"
    )
    .bind(name.clone())
    .bind(guild_id.to_string())
    .fetch_optional(&ctx.db_pool)
    .await?
    .ok_or_else(|| eyre::eyre!("Group not found"))?;
    
    let group_id = group_row.get::<uuid::Uuid, _>("id");
    let group_role_id: Option<String> = group_row.try_get("role_id").ok();
    
    // Create a simple struct to hold the data
    #[derive(Clone)]
    struct GroupInfo {
        id: uuid::Uuid,
        role_id: Option<String>,
    }
    
    let group = GroupInfo {
        id: group_id,
        role_id: group_role_id,
    };
    
    // Make the request to update the group
    let update_request = timesync_core::models::discord::UpdateDiscordGroupRequest {
        name: None,
        add_member_ids: Some(member_ids.clone()),
        remove_member_ids: None,
    };
    
    // Make API request to update the group
    let client = reqwest::Client::new();
    let response = client.put(format!("{}/api/discord/groups/{}", ctx.config.web_base_url, group.id))
        .json(&update_request)
        .send()
        .await?;
    
    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(eyre::eyre!("Failed to update group: {}", error_text));
    }
    
    // Get the guild
    let guild = guild_id.to_guild_cached(&ctx.ctx)
        .ok_or_else(|| eyre::eyre!("Failed to get guild"))?;
    
    // Assign the role to new members if the group has a role
    if let Some(role_id_str) = &group.role_id {
        if let Ok(role_id) = role_id_str.parse::<u64>() {
            // Assign role to all new members
            for member_id in &member_ids {
                if let Ok(user_id) = member_id.parse::<u64>() {
                    if let Ok(mut member) = guild.member(&ctx.ctx.http, user_id).await {
                        // Ignore errors when adding roles to allow the process to continue
                        let _ = member.add_role(&ctx.ctx.http, role_id).await;
                    }
                }
            }
        }
    }
    
    // Respond to the interaction
    command.create_interaction_response(&ctx.ctx.http, |r| {
        r.kind(InteractionResponseType::ChannelMessageWithSource)
            .interaction_response_data(|m| {
                m.embed(|e| {
                    e.title("Members Added")
                        .description(format!("Added {} {} to group **{}**", 
                            member_ids.len(),
                            if member_ids.len() == 1 { "member" } else { "members" },
                            name
                        ))
                        .field("Added Members", format_member_list(&member_ids), false)
                        .color(Color::DARK_GREEN)
                        .timestamp(Utc::now().to_rfc3339())
                })
            })
    }).await?;
    
    Ok(())
}

/// Handle the /group remove subcommand
async fn handle_group_remove(
    ctx: HandlerContext,
    command: &ApplicationCommandInteraction,
    subcommand: &CommandDataOption,
) -> Result<()> {
    // Get the name option
    let name = get_option_string(subcommand, "name")?;
    
    // Get the members option
    let members_str = get_option_string(subcommand, "members")?;
    
    // Get the guild
    let guild_id = command.guild_id
        .ok_or_else(|| eyre::eyre!("Command must be used in a server"))?;
    
    // Parse the mention tags
    let member_ids = parse_mention_tags(&members_str);
    
    if member_ids.is_empty() {
        command.create_interaction_response(&ctx.ctx.http, |r| {
            r.kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|m| {
                    m.content("No valid members specified").ephemeral(true)
                })
        }).await?;
        
        return Ok(());
    }
    
    // Find the group by name in this server using basic query to avoid sqlx compile-time checks
    let group_row = sqlx::query(
        "SELECT id, role_id FROM discord_groups WHERE name = $1 AND server_id = $2"
    )
    .bind(name.clone())
    .bind(guild_id.to_string())
    .fetch_optional(&ctx.db_pool)
    .await?
    .ok_or_else(|| eyre::eyre!("Group not found"))?;
    
    let group_id = group_row.get::<uuid::Uuid, _>("id");
    let group_role_id: Option<String> = group_row.try_get("role_id").ok();
    
    // Create a simple struct to hold the data
    #[derive(Clone)]
    struct GroupInfo {
        id: uuid::Uuid,
        role_id: Option<String>,
    }
    
    let group = GroupInfo {
        id: group_id,
        role_id: group_role_id,
    };
    
    // Get the guild
    let guild = guild_id.to_guild_cached(&ctx.ctx)
        .ok_or_else(|| eyre::eyre!("Failed to get guild"))?;
    
    // Remove the role from members if the group has a role
    if let Some(role_id_str) = &group.role_id {
        if let Ok(role_id) = role_id_str.parse::<u64>() {
            // Remove role from all members being removed
            for member_id in &member_ids {
                if let Ok(user_id) = member_id.parse::<u64>() {
                    if let Ok(mut member) = guild.member(&ctx.ctx.http, user_id).await {
                        // Ignore errors when removing roles to allow the process to continue
                        let _ = member.remove_role(&ctx.ctx.http, role_id).await;
                    }
                }
            }
        }
    }
    
    // Make the request to update the group
    let update_request = timesync_core::models::discord::UpdateDiscordGroupRequest {
        name: None,
        add_member_ids: None,
        remove_member_ids: Some(member_ids.clone()),
    };
    
    // Make API request to update the group
    let client = reqwest::Client::new();
    let response = client.put(format!("{}/api/discord/groups/{}", ctx.config.web_base_url, group.id))
        .json(&update_request)
        .send()
        .await?;
    
    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(eyre::eyre!("Failed to update group: {}", error_text));
    }
    
    // Respond to the interaction
    command.create_interaction_response(&ctx.ctx.http, |r| {
        r.kind(InteractionResponseType::ChannelMessageWithSource)
            .interaction_response_data(|m| {
                m.embed(|e| {
                    e.title("Members Removed")
                        .description(format!("Removed {} {} from group **{}**", 
                            member_ids.len(),
                            if member_ids.len() == 1 { "member" } else { "members" },
                            name
                        ))
                        .field("Removed Members", format_member_list(&member_ids), false)
                        .color(Color::ORANGE)
                        .timestamp(Utc::now().to_rfc3339())
                })
            })
    }).await?;
    
    Ok(())
}

/// Handle the /group info subcommand
async fn handle_group_info(
    ctx: HandlerContext,
    command: &ApplicationCommandInteraction,
    subcommand: &CommandDataOption,
) -> Result<()> {
    // Get the name option
    let name = get_option_string(subcommand, "name")?;
    
    // Get the guild (server) ID
    let server_id = command.guild_id
        .ok_or_else(|| eyre::eyre!("Command must be used in a server"))?
        .to_string();
    
    // Find the group by name in this server
    let group = sqlx::query!(
        "SELECT id, name, created_at FROM discord_groups WHERE name = $1 AND server_id = $2",
        name,
        server_id
    )
    .fetch_optional(&ctx.db_pool)
    .await?
    .ok_or_else(|| eyre::eyre!("Group not found"))?;
    
    // Get all members of the group
    let members = sqlx::query!(
        "SELECT gm.discord_id, du.schedule_id 
         FROM group_members gm
         LEFT JOIN discord_users du ON gm.discord_id = du.discord_id
         WHERE gm.group_id = $1",
        group.id
    )
    .fetch_all(&ctx.db_pool)
    .await?;
    
    // Get the role information using basic query to avoid sqlx compile-time checks
    let role_row = sqlx::query(
        "SELECT role_id FROM discord_groups WHERE id = $1"
    )
    .bind(group.id)
    .fetch_one(&ctx.db_pool)
    .await?;
    
    let role_id: Option<String> = role_row.try_get("role_id").ok();
    
    // Create a formatted list of members
    let mut members_list = String::new();
    let mut with_schedule = 0;
    
    for member in &members {
        let has_schedule = member.schedule_id.is_some();
        if has_schedule {
            with_schedule += 1;
        }
        
        members_list.push_str(&format!("<@{}> {}\n",
            member.discord_id,
            if has_schedule { "✅" } else { "❌" }
        ));
    }
    
    if members_list.is_empty() {
        members_list = "No members in this group yet".to_string();
    }
    
    // Create description text
    let description = format!(
        "This group has {} members, {} of which have availability schedules.",
        members.len(),
        with_schedule
    );
    
    // Add role information if available
    let role_field = if let Some(role_id_str) = &role_id {
        format!("<@&{}>", role_id_str)
    } else {
        "No role assigned".to_string()
    };
    
    // Respond to the interaction
    command.create_interaction_response(&ctx.ctx.http, |r| {
        r.kind(InteractionResponseType::ChannelMessageWithSource)
            .interaction_response_data(|m| {
                m.embed(|e| {
                    e.title(format!("Group: {}", group.name))
                        .description(description)
                        .field("Discord Role", role_field, false)
                        .field(
                            "Members (✅ = has schedule, ❌ = no schedule)", 
                            members_list, 
                            false
                        )
                        .footer(|f| f.text(format!(
                            "Created: {} • Group ID: {}",
                            group.created_at.format("%Y-%m-%d"),
                            group.id
                        )))
                        .color(Color::BLUE)
                })
            })
    }).await?;
    
    Ok(())
}

/// Handle the /match command
pub async fn handle_match_command(
    ctx: HandlerContext,
    command: &ApplicationCommandInteraction,
) -> Result<()> {
    // Get the groups
    let groups_str = command.data.options.first()
        .ok_or_else(|| eyre::eyre!("Missing groups parameter"))?
        .value.as_ref()
        .and_then(|val| val.as_str())
        .ok_or_else(|| eyre::eyre!("Invalid groups parameter"))?;
    
    // Get the guild (server) ID
    let server_id = command.guild_id
        .ok_or_else(|| eyre::eyre!("Command must be used in a server"))?
        .to_string();
    
    // Get optional parameters
    let min_per_group = command.data.options.get(1)
        .and_then(|opt| opt.value.as_ref())
        .and_then(|val| val.as_i64())
        .unwrap_or(1);
    
    let count = command.data.options.get(2)
        .and_then(|opt| opt.value.as_ref())
        .and_then(|val| val.as_i64())
        .unwrap_or(5);
    
    // Parse the group names
    let group_names: Vec<String> = groups_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    
    if group_names.is_empty() {
        command.create_interaction_response(&ctx.ctx.http, |r| {
            r.kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|m| {
                    m.content("No valid groups specified").ephemeral(true)
                })
        }).await?;
        
        return Ok(());
    }
    
    // Find the group IDs for these names
    let mut group_ids = Vec::new();
    
    for name in &group_names {
        match sqlx::query!(
            "SELECT id FROM discord_groups WHERE name = $1 AND server_id = $2",
            name,
            server_id
        )
        .fetch_optional(&ctx.db_pool)
        .await? {
            Some(record) => group_ids.push(record.id),
            None => {
                command.create_interaction_response(&ctx.ctx.http, |r| {
                    r.kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|m| {
                            m.content(format!("Group not found: {}", name)).ephemeral(true)
                        })
                }).await?;
                
                return Ok(());
            }
        }
    }
    
    // Acknowledge the command first to buy time for processing
    command.create_interaction_response(&ctx.ctx.http, |r| {
        r.kind(InteractionResponseType::DeferredChannelMessageWithSource)
    }).await?;
    
    // Make API request to get matches
    let match_request = timesync_core::models::discord::MatchRequest {
        group_ids,
        min_per_group: Some(min_per_group as usize),
        count: Some(count as usize),
    };
    
    let client = reqwest::Client::new();
    let response = client.get(format!("{}/api/availability/match", ctx.config.web_base_url))
        .query(&[
            ("group_ids", match_request.group_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",")),
            ("min_per_group", min_per_group.to_string()),
            ("count", count.to_string()),
        ])
        .send()
        .await?;
    
    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(eyre::eyre!("Failed to find matches: {}", error_text));
    }
    
    let match_response: timesync_core::models::discord::MatchResponse = response.json().await?;
    
    if match_response.matches.is_empty() {
        command.edit_original_interaction_response(&ctx.ctx.http, |m| {
            m.embed(|e| {
                e.title("No Matching Times Found")
                    .description("There are no time slots where the specified groups have common availability.")
                    .color(Color::RED)
            })
        }).await?;
        
        return Ok(());
    }
    
    // Calculate the total number of unique users in all groups
    let first_match = &match_response.matches[0];
    let mut unique_user_ids = std::collections::HashSet::new();
    
    for group in &first_match.groups {
        for user_id in &group.available_users {
            unique_user_ids.insert(user_id);
        }
    }
    let total_unique_users = unique_user_ids.len();
    
    // Determine how many users need to say yes for the time to be confirmed
    // Using a simple majority (over 50%)
    let required_yes_count = (total_unique_users as f64 * 0.5).ceil() as usize;
    
    // Get the server's timezone or use UTC as default
    let timezone = async {
        // Check if the discord_servers table exists
        let table_exists = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS (
                SELECT 1 
                FROM information_schema.tables 
                WHERE table_name = 'discord_servers'
            );
            "#,
        )
        .fetch_one(&ctx.db_pool)
        .await
        .unwrap_or(false);
        
        if table_exists {
            // Get the server's timezone
            let result = sqlx::query(
                "SELECT timezone FROM discord_servers WHERE server_id = $1"
            )
            .bind(server_id.clone())
            .fetch_optional(&ctx.db_pool)
            .await;
            
            match result {
                Ok(Some(row)) => match row.try_get::<String, _>("timezone") {
                    Ok(tz) => Some(tz),
                    Err(_) => None
                },
                _ => None
            }
            .unwrap_or_else(|| "UTC".to_string())
        } else {
            "UTC".to_string()
        }
    }.await;
    
    // Create initial poll with first match
    let active_poll = super::ActivePoll {
        matches: match_response.matches.clone(),
        current_index: 0,
        group_names: group_names.clone(),
        min_per_group,
        required_yes_count,
        responses: HashMap::new(),
        db_pool: ctx.db_pool.clone(),
        timezone,
    };
    
    // Create a formatted response with just the first match
    let first_match_message = format_match_option(&active_poll, 0, required_yes_count);
    
    // Get role mentions for all groups
    let mut role_mentions = Vec::new();
    for group_name in &group_names {
        // Query to get the role ID for this group
        let role_query = sqlx::query(
            "SELECT role_id FROM discord_groups WHERE name = $1 AND server_id = $2"
        )
        .bind(group_name)
        .bind(server_id.clone())
        .fetch_optional(&ctx.db_pool)
        .await;
        
        if let Ok(Some(row)) = role_query {
            if let Ok(role_id) = row.try_get::<Option<String>, _>("role_id") {
                if let Some(id) = role_id {
                    role_mentions.push(format!("<@&{}>", id));
                }
            }
        }
    }
    
    let role_ping = if !role_mentions.is_empty() {
        format!("🗣️ {} A meeting time has been proposed! Please vote!", role_mentions.join(" "))
    } else {
        String::new()
    };
    
    // Edit the original response with the first match
    let message = command.edit_original_interaction_response(&ctx.ctx.http, |m| {
        if !role_ping.is_empty() {
            m.content(&role_ping);
        }
        
        m.embed(|e| {
            e.title(format!("Proposed Meeting Time (1 of {})", match_response.matches.len()))
                .description(&first_match_message)
                .color(Color::GOLD)
                .footer(|f| f.text(format!(
                    "Min members per group: {} • {}/{} yes votes needed • Generated at: {}",
                    min_per_group,
                    0,
                    required_yes_count,
                    Utc::now().format("%Y-%m-%d %H:%M UTC")
                )))
        })
        .components(|c| {
            c.create_action_row(|row| {
                row.create_button(|b| {
                    b.custom_id("match_yes")
                        .label("Yes")
                        .style(serenity::model::application::component::ButtonStyle::Success)
                })
                .create_button(|b| {
                    b.custom_id("match_no")
                        .label("No")
                        .style(serenity::model::application::component::ButtonStyle::Danger)
                })
            })
            .create_action_row(|row| {
                if active_poll.matches.len() > 1 {
                    row.create_button(|b| {
                        b.custom_id("match_next")
                            .label("Next Option")
                            .style(serenity::model::application::component::ButtonStyle::Secondary)
                    })
                } else {
                    row
                }
            })
        })
    }).await?;
    
    // Store the poll state
    {
        let mut polls = ctx.active_polls.write().await;
        polls.insert(message.id, active_poll);
    }
    
    Ok(())
}

/// Handle button interactions for scheduling matches
pub async fn handle_component_interaction(
    ctx: HandlerContext,
    component: &mut MessageComponentInteraction
) -> Result<()> {
    let custom_id = &component.data.custom_id;
    
    match custom_id.as_str() {
        "match_yes" | "match_no" => handle_match_vote(ctx, component, custom_id == "match_yes").await,
        "match_next" => handle_match_next(ctx, component).await,
        "match_confirm" => handle_match_confirm(ctx, component).await,
        _ => Ok(()),
    }
}

/// Handle voting interactions (Yes/No)
async fn handle_match_vote(
    ctx: HandlerContext,
    component: &mut MessageComponentInteraction,
    is_yes: bool,
) -> Result<()> {
    // Acknowledge the interaction
    component.create_interaction_response(&ctx.ctx.http, |r| {
        r.kind(InteractionResponseType::DeferredUpdateMessage)
    }).await?;

    // Get the poll associated with this message
    let mut polls = ctx.active_polls.write().await;
    let poll = polls.get_mut(&component.message.id)
        .ok_or_else(|| eyre::eyre!("No active poll found for this message"))?;
    
    // Record the user's response
    let user_id = component.user.id.to_string();
    poll.responses.insert(user_id, is_yes);
    
    // Count yes votes
    let yes_votes = poll.responses.values().filter(|&&v| v).count();
    
    // Check if we have enough yes votes
    if yes_votes >= poll.required_yes_count {
        // Enough people agreed to this time, confirm it
        let match_result = &poll.matches[poll.current_index];
        
        // Format the time using the poll's timezone if possible
        let (start_time, end_time, tz_display) = if let Ok(tz) = chrono_tz::Tz::from_str(&poll.timezone) {
            let start_in_tz = match_result.start.with_timezone(&tz);
            let end_in_tz = match_result.end.with_timezone(&tz);
            
            (
                start_in_tz.format("%a, %b %d at %H:%M").to_string(),
                end_in_tz.format("%H:%M").to_string(),
                poll.timezone.clone()
            )
        } else {
            // Fallback to UTC
            (
                match_result.start.format("%a, %b %d at %H:%M").to_string(),
                match_result.end.format("%H:%M").to_string(),
                "UTC".to_string()
            )
        };
        
        // Collect attendees and role IDs for the ping message
        let attendees: Vec<String> = poll.responses.iter()
            .filter(|&(_, is_attending)| *is_attending)
            .map(|(user_id, _)| format!("<@{}>", user_id))
            .collect();
        
        // Gather role IDs for all groups involved
        let mut role_ids = Vec::new();
        for group_name in &poll.group_names {
            // Query to get the role ID for this group
            let role_query = sqlx::query(
                "SELECT role_id FROM discord_groups WHERE name = $1 AND server_id = $2"
            )
            .bind(group_name)
            .bind(component.guild_id.unwrap().to_string())
            .fetch_optional(&ctx.db_pool)
            .await;
            
            if let Ok(Some(row)) = role_query {
                if let Ok(role_id) = row.try_get::<Option<String>, _>("role_id") {
                    if let Some(id) = role_id {
                        role_ids.push(format!("<@&{}>", id));
                    }
                }
            }
        }
            
        // Create ping message for attendees and roles
        let ping_message = {
            let mut message = "🔔 Meeting confirmed! ".to_string();
            
            // Add role pings
            if !role_ids.is_empty() {
                message.push_str(&format!("{} ", role_ids.join(" ")));
            }
            
            // Add individual attendee pings
            if !attendees.is_empty() {
                message.push_str("- Please mark your calendars!");
            } else {
                message.push_str("Please mark your calendars!");
            }
            
            message
        };
        
        let mut description = format!(
            "The meeting time has been confirmed!\n\n**{}** - **{}** ({})\n\n",
            start_time, end_time, tz_display
        );
        
        description.push_str("**Attending:**\n");
        for (user_id, response) in &poll.responses {
            if *response {
                description.push_str(&format!("• <@{}> ✅\n", user_id));
            }
        }
        
        description.push_str("\n**Not Available:**\n");
        for (user_id, response) in &poll.responses {
            if !response {
                description.push_str(&format!("• <@{}> ❌\n", user_id));
            }
        }
        
        // Update the message to show the confirmation
        component.message.edit(&ctx.ctx.http, |m| {
            m.content(&ping_message)
             .embed(|e| {
                e.title("Meeting Time Confirmed!")
                    .description(description)
                    .color(Color::DARK_GREEN)
                    .footer(|f| f.text(format!(
                        "Min members per group: {} • {}/{} yes votes received",
                        poll.min_per_group,
                        yes_votes,
                        poll.required_yes_count
                    )))
            })
            .components(|c| c) // Clear components
        }).await?;
        
        // Remove the poll from active polls
        polls.remove(&component.message.id);
    } else {
        // Update the message to show the updated vote count
        let match_message = format_match_option(poll, poll.current_index, poll.required_yes_count);
        
        // Gather role IDs for all groups to ping during voting
        let mut role_mentions = Vec::new();
        for group_name in &poll.group_names {
            // Query to get the role ID for this group
            let role_query = sqlx::query(
                "SELECT role_id FROM discord_groups WHERE name = $1 AND server_id = $2"
            )
            .bind(group_name)
            .bind(component.guild_id.unwrap().to_string())
            .fetch_optional(&ctx.db_pool)
            .await;
            
            if let Ok(Some(row)) = role_query {
                if let Ok(role_id) = row.try_get::<Option<String>, _>("role_id") {
                    if let Some(id) = role_id {
                        role_mentions.push(format!("<@&{}>", id));
                    }
                }
            }
        }
        
        let role_ping = if !role_mentions.is_empty() {
            format!("🗣️ {} Please vote on this meeting time proposal!", role_mentions.join(" "))
        } else {
            String::new()
        };
        
        component.message.edit(&ctx.ctx.http, |m| {
            if !role_ping.is_empty() {
                m.content(&role_ping);
            }
            
            m.embed(|e| {
                e.title(format!("Proposed Meeting Time ({} of {})", 
                               poll.current_index + 1, 
                               poll.matches.len()))
                    .description(&match_message)
                    .color(Color::GOLD)
                    .footer(|f| f.text(format!(
                        "Min members per group: {} • {}/{} yes votes needed • Generated at: {}",
                        poll.min_per_group,
                        yes_votes,
                        poll.required_yes_count,
                        Utc::now().format("%Y-%m-%d %H:%M UTC")
                    )))
            })
            .components(|c| {
                c.create_action_row(|row| {
                    row.create_button(|b| {
                        b.custom_id("match_yes")
                            .label("Yes")
                            .style(serenity::model::application::component::ButtonStyle::Success)
                    })
                    .create_button(|b| {
                        b.custom_id("match_no")
                            .label("No")
                            .style(serenity::model::application::component::ButtonStyle::Danger)
                    })
                })
                .create_action_row(|row| {
                    if poll.matches.len() > 1 {
                        row.create_button(|b| {
                            b.custom_id("match_next")
                                .label("Next Option")
                                .style(serenity::model::application::component::ButtonStyle::Secondary)
                        })
                    } else {
                        row
                    }
                })
            })
        }).await?;
    }
    
    Ok(())
}

/// Handle the "Next Option" button to cycle through matches
async fn handle_match_next(
    ctx: HandlerContext,
    component: &mut MessageComponentInteraction,
) -> Result<()> {
    // Acknowledge the interaction
    component.create_interaction_response(&ctx.ctx.http, |r| {
        r.kind(InteractionResponseType::DeferredUpdateMessage)
    }).await?;

    // Get the poll associated with this message
    let mut polls = ctx.active_polls.write().await;
    let poll = polls.get_mut(&component.message.id)
        .ok_or_else(|| eyre::eyre!("No active poll found for this message"))?;
    
    // Move to the next match option, or wrap around to the first
    poll.current_index = (poll.current_index + 1) % poll.matches.len();
    
    // Count yes votes
    let yes_votes = poll.responses.values().filter(|&&v| v).count();
    
    // Update the message to show the next option
    let match_message = format_match_option(poll, poll.current_index, poll.required_yes_count);
    
    component.message.edit(&ctx.ctx.http, |m| {
        m.embed(|e| {
            e.title(format!("Proposed Meeting Time ({} of {})", 
                           poll.current_index + 1, 
                           poll.matches.len()))
                .description(&match_message)
                .color(Color::GOLD)
                .footer(|f| f.text(format!(
                    "Min members per group: {} • {}/{} yes votes needed • Generated at: {}",
                    poll.min_per_group,
                    yes_votes,
                    poll.required_yes_count,
                    Utc::now().format("%Y-%m-%d %H:%M UTC")
                )))
        })
        .components(|c| {
            c.create_action_row(|row| {
                row.create_button(|b| {
                    b.custom_id("match_yes")
                        .label("Yes")
                        .style(serenity::model::application::component::ButtonStyle::Success)
                })
                .create_button(|b| {
                    b.custom_id("match_no")
                        .label("No")
                        .style(serenity::model::application::component::ButtonStyle::Danger)
                })
            })
            .create_action_row(|row| {
                row.create_button(|b| {
                    b.custom_id("match_next")
                        .label("Next Option")
                        .style(serenity::model::application::component::ButtonStyle::Secondary)
                })
            })
        })
    }).await?;
    
    Ok(())
}

/// Handle the "Confirm Meeting" button (legacy implementation)
async fn handle_match_confirm(
    ctx: HandlerContext,
    component: &mut MessageComponentInteraction,
) -> Result<()> {
    // This is the "old" confirm button, which should not be used anymore
    // However, we keep this for backward compatibility
    component.create_interaction_response(&ctx.ctx.http, |r| {
        r.kind(InteractionResponseType::ChannelMessageWithSource)
            .interaction_response_data(|m| {
                m.content("This feature has been updated. Please use the new /match command to propose meeting times.")
                    .ephemeral(true)
            })
    }).await?;
    
    Ok(())
}

/// Format a single match option for display
fn format_match_option(poll: &super::ActivePoll, index: usize, required_yes: usize) -> String {
    let match_result = &poll.matches[index];
    
    // Format the time using the poll's timezone if possible
    let (formatted_start, formatted_end, tz_display) = if let Ok(tz) = chrono_tz::Tz::from_str(&poll.timezone) {
        let start_in_tz = match_result.start.with_timezone(&tz);
        let end_in_tz = match_result.end.with_timezone(&tz);
        
        (
            start_in_tz.format("%a, %b %d at %H:%M").to_string(),
            end_in_tz.format("%H:%M").to_string(),
            poll.timezone.clone()
        )
    } else {
        // Fallback to UTC
        (
            match_result.start.format("%a, %b %d at %H:%M").to_string(),
            match_result.end.format("%H:%M").to_string(),
            "UTC".to_string()
        )
    };
    
    let mut description = format!(
        "**Proposed Time:** {} - {} ({})\n\n",
        formatted_start, formatted_end, tz_display
    );
    
    // Add group information
    description.push_str("**Available Members:**\n");
    for group in &match_result.groups {
        description.push_str(&format!(
            "• **{}**: {} {}\n",
            group.name,
            group.count,
            if group.count == 1 { "member" } else { "members" }
        ));
        
        // List the available users for each group
        if !group.available_users.is_empty() {
            for user_id in &group.available_users {
                description.push_str(&format!("  - <@{}>\n", user_id));
            }
        }
    }
    
    // Add responses if there are any
    let yes_count = poll.responses.iter().filter(|&(_, v)| *v).count();
    let yes_users: Vec<&String> = poll.responses.iter()
        .filter(|&(_, v)| *v)
        .map(|(user_id, _)| user_id)
        .collect();
    
    let no_users: Vec<&String> = poll.responses.iter()
        .filter(|&(_, v)| !(*v))
        .map(|(user_id, _)| user_id)
        .collect();
    
    if !yes_users.is_empty() || !no_users.is_empty() {
        description.push_str("\n**Current Responses:**\n");
        
        if !yes_users.is_empty() {
            description.push_str("✅ **Yes:**\n");
            for user_id in &yes_users {
                description.push_str(&format!("• <@{}>\n", user_id));
            }
        }
        
        if !no_users.is_empty() {
            description.push_str("❌ **No:**\n");
            for user_id in &no_users {
                description.push_str(&format!("• <@{}>\n", user_id));
            }
        }
    }
    
    // Add progress indicator
    description.push_str(&format!(
        "\n{}/{} yes votes needed to confirm this time",
        yes_count,
        required_yes
    ));
    
    description
}

/// Handle the /timezone command
pub async fn handle_timezone_command(
    ctx: HandlerContext, 
    command: &ApplicationCommandInteraction
) -> Result<()> {
    // Get the subcommand
    let subcommand = command.data.options.first()
        .ok_or_else(|| eyre::eyre!("Missing subcommand"))?;
    
    match subcommand.name.as_str() {
        "set" => handle_timezone_set(ctx, command, subcommand).await,
        "show" => handle_timezone_show(ctx, command).await,
        "list" => handle_timezone_list(ctx, command).await,
        _ => {
            command.create_interaction_response(&ctx.ctx.http, |r| {
                r.kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|m| {
                        m.content("Unknown subcommand").ephemeral(true)
                    })
            }).await?;
            
            Ok(())
        }
    }
}

/// Handle the /timezone set subcommand
async fn handle_timezone_set(
    ctx: HandlerContext,
    command: &ApplicationCommandInteraction,
    subcommand: &CommandDataOption,
) -> Result<()> {
    // Get the guild (server) ID
    let server_id = command.guild_id
        .ok_or_else(|| eyre::eyre!("Command must be used in a server"))?
        .to_string();
    
    // Get the timezone option
    let timezone = get_option_string(subcommand, "timezone")?;
    
    // Validate the timezone
    if !is_valid_timezone(&timezone) {
        command.create_interaction_response(&ctx.ctx.http, |r| {
            r.kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|m| {
                    m.content(format!(
                        "Invalid timezone: {}. Use `/timezone list` to see available options.",
                        timezone
                    )).ephemeral(true)
                })
        }).await?;
        
        return Ok(());
    }
    
    // First check if the discord_servers table exists
    let table_exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1 
            FROM information_schema.tables 
            WHERE table_name = 'discord_servers'
        );
        "#,
    )
    .fetch_one(&ctx.db_pool)
    .await?;
    
    if !table_exists {
        // Create the table if it doesn't exist
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS discord_servers (
                server_id VARCHAR(255) PRIMARY KEY,
                timezone VARCHAR(100) NOT NULL DEFAULT 'UTC',
                created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
            );
            "#,
        )
        .execute(&ctx.db_pool)
        .await?;
        
        // Create index
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_discord_servers_server_id ON discord_servers(server_id)"
        )
        .execute(&ctx.db_pool)
        .await?;
    }
    
    // Store the timezone for the response
    let timezone_copy = timezone.clone();
    
    // Insert or update the server's timezone
    sqlx::query(
        r#"
        INSERT INTO discord_servers (server_id, timezone) 
        VALUES ($1, $2)
        ON CONFLICT (server_id) 
        DO UPDATE SET timezone = $2
        "#
    )
    .bind(server_id)
    .bind(timezone)
    .execute(&ctx.db_pool)
    .await?;
    
    // Respond to the interaction
    command.create_interaction_response(&ctx.ctx.http, |r| {
        r.kind(InteractionResponseType::ChannelMessageWithSource)
            .interaction_response_data(|m| {
                m.embed(|e| {
                    e.title("Server Timezone Updated")
                        .description(format!("The server timezone has been set to **{}**", timezone_copy))
                        .color(Color::DARK_GREEN)
                        .footer(|f| f.text("All times in match commands will be displayed in this timezone"))
                })
            })
    }).await?;
    
    Ok(())
}

/// Handle the /timezone show subcommand
async fn handle_timezone_show(
    ctx: HandlerContext,
    command: &ApplicationCommandInteraction,
) -> Result<()> {
    // Get the guild (server) ID
    let server_id = command.guild_id
        .ok_or_else(|| eyre::eyre!("Command must be used in a server"))?
        .to_string();
    
    // First check if the discord_servers table exists
    let table_exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1 
            FROM information_schema.tables 
            WHERE table_name = 'discord_servers'
        );
        "#,
    )
    .fetch_one(&ctx.db_pool)
    .await?;
    
    let timezone_result = if table_exists {
        // Get the server's timezone
        let result = sqlx::query(
            "SELECT timezone FROM discord_servers WHERE server_id = $1"
        )
        .bind(server_id)
        .fetch_optional(&ctx.db_pool)
        .await?;
        
        match result {
            Some(row) => match row.try_get::<String, _>("timezone") {
                Ok(tz) => Some(tz),
                Err(_) => None
            },
            None => None
        }
    } else {
        None
    };
    
    let timezone = timezone_result.unwrap_or_else(|| "UTC".to_string());
    
    // Get the current time in that timezone
    let utc_now = chrono::Utc::now();
    let timezone_info = if let Ok(tz) = chrono_tz::Tz::from_str(&timezone) {
        let local_time = utc_now.with_timezone(&tz);
        
        // Calculate offset in hours
        let utc_offset = local_time.offset().fix().local_minus_utc() as f64 / 3600.0;
        let offset_str = if utc_offset >= 0.0 {
            format!("+{}", utc_offset)
        } else {
            format!("{}", utc_offset)
        };
        
        format!(
            "Current time: **{}**\nOffset from UTC: **{}**",
            local_time.format("%Y-%m-%d %H:%M:%S"),
            offset_str
        )
    } else {
        "Unable to determine current time in this timezone.".to_string()
    };
    
    // Respond to the interaction
    command.create_interaction_response(&ctx.ctx.http, |r| {
        r.kind(InteractionResponseType::ChannelMessageWithSource)
            .interaction_response_data(|m| {
                m.embed(|e| {
                    e.title("Server Timezone")
                        .description(format!("This server's timezone is set to **{}**", timezone))
                        .field("Timezone Information", timezone_info, false)
                        .color(Color::BLUE)
                })
            })
    }).await?;
    
    Ok(())
}

/// Handle the /timezone list subcommand
async fn handle_timezone_list(
    ctx: HandlerContext,
    command: &ApplicationCommandInteraction,
) -> Result<()> {
    // Common timezones by region
    let timezones = vec![
        ("**North America**", vec![
            "America/Los_Angeles (Pacific Time)",
            "America/Denver (Mountain Time)",
            "America/Chicago (Central Time)",
            "America/New_York (Eastern Time)",
        ]),
        ("**Europe**", vec![
            "Europe/London (GMT/BST)",
            "Europe/Paris (Central European Time)",
            "Europe/Helsinki (Eastern European Time)",
        ]),
        ("**Asia/Pacific**", vec![
            "Asia/Tokyo (Japan Standard Time)",
            "Asia/Shanghai (China Standard Time)",
            "Asia/Kolkata (India Standard Time)",
            "Australia/Sydney (Australian Eastern Standard Time)",
        ]),
        ("**Other**", vec![
            "UTC (Coordinated Universal Time)",
            "Etc/GMT+12 (UTC-12)",
            "Etc/GMT-12 (UTC+12)",
        ]),
    ];
    
    // Build description with all timezones
    let mut description = "Here are some common timezones you can use:\n\n".to_string();
    
    for (region, zones) in timezones {
        description.push_str(&format!("{}\n", region));
        for zone in zones {
            description.push_str(&format!("• `{}`\n", zone));
        }
        description.push_str("\n");
    }
    
    description.push_str("\nUse `/timezone set <timezone>` to set your server's timezone. For a complete list of available timezones, see the [IANA Time Zone Database](https://en.wikipedia.org/wiki/List_of_tz_database_time_zones).");
    
    // Respond to the interaction
    command.create_interaction_response(&ctx.ctx.http, |r| {
        r.kind(InteractionResponseType::ChannelMessageWithSource)
            .interaction_response_data(|m| {
                m.embed(|e| {
                    e.title("Available Timezones")
                        .description(description)
                        .color(Color::BLUE)
                        .footer(|f| f.text("Timezone data from the IANA Time Zone Database"))
                })
            })
    }).await?;
    
    Ok(())
}

/// Check if a timezone string is valid
fn is_valid_timezone(timezone: &str) -> bool {
    // We'll validate by trying to parse it
    chrono_tz::Tz::from_str(timezone).is_ok()
}

/// Extract a string option from a command
fn get_option_string(options: &CommandDataOption, name: &str) -> Result<String> {
    options.options.iter()
        .find(|opt| opt.name == name)
        .and_then(|opt| opt.value.as_ref())
        .and_then(|val| val.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| eyre::eyre!("Missing {} parameter", name))
}

/// Parse mention tags from a comma-separated string
fn parse_mention_tags(input: &str) -> Vec<String> {
    input.split(',')
        .filter_map(|part| {
            let part = part.trim();
            
            // Extract user ID from a mention tag
            if part.starts_with("<@") && part.ends_with('>') {
                let id_part = part.trim_start_matches("<@").trim_start_matches('!').trim_end_matches('>');
                if let Ok(id) = id_part.parse::<u64>() {
                    return Some(id.to_string());
                }
            }
            
            // Try to parse as a raw user ID
            if let Ok(id) = part.parse::<u64>() {
                return Some(id.to_string());
            }
            
            None
        })
        .collect()
}

/// Format a list of member IDs as mention tags
fn format_member_list(member_ids: &[String]) -> String {
    if member_ids.is_empty() {
        return "None".to_string();
    }
    
    member_ids.iter()
        .map(|id| format!("<@{}>", id))
        .collect::<Vec<_>>()
        .join(", ")
}