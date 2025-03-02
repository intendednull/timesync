use chrono::{Utc, Offset, Datelike, Timelike};
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
use timesync_core::models::discord::{CreateDiscordGroupRequest, CreateDiscordGroupResponse, MatchResult};
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
    // Get the user's Discord ID and username
    let discord_id = command.user.id.to_string();
    let username = &command.user.name;
    
    // Generate a unique URL for schedule creation with discord_id and default name
    let schedule_url = format!("{}/create?discord_id={}&name={}", 
        ctx.config.web_base_url, 
        discord_id, 
        urlencoding::encode(username));
    
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
        
    // Get slot duration (default to 120 minutes / 2 hours)
    let slot_duration = command.data.options.get(2)
        .and_then(|opt| opt.value.as_ref())
        .and_then(|val| val.as_i64())
        .unwrap_or(120);
        
    // Get max days to display (default to 7, clamp between 1-7)
    let display_days = command.data.options.get(3)
        .and_then(|opt| opt.value.as_ref())
        .and_then(|val| val.as_i64())
        .unwrap_or(7)
        .max(1)
        .min(7);
    
    // Request a high number of matches to avoid running out
    let count = 50; // Set to a high value instead of using a parameter
    
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
    
    // Collect all eligible voters (members of the chosen groups)
    let mut eligible_voter_ids = std::collections::HashSet::new();
    let mut group_members = HashMap::new();
    
    // Get all members of all groups
    for group_id in &match_request.group_ids {
        let members = sqlx::query!(
            "SELECT discord_id FROM group_members WHERE group_id = $1",
            group_id
        )
        .fetch_all(&ctx.db_pool)
        .await?;
        
        // Create a vector of member IDs for this group
        let mut member_ids = Vec::new();
        
        for member in members {
            eligible_voter_ids.insert(member.discord_id.clone());
            member_ids.push(member.discord_id);
        }
        
        // Store the member list for this group
        group_members.insert(*group_id, member_ids);
    }
    
    // Set the required "Yes" votes (6 per group or all members if less than 6)
    let mut required_yes_count = 0;
    for members in group_members.values() {
        required_yes_count += std::cmp::min(6, members.len());
    }
    
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
                Ok(Some(row)) => row.try_get::<String, _>("timezone").ok(),
                _ => None
            }
            .unwrap_or_else(|| "UTC".to_string())
        } else {
            "UTC".to_string()
        }
    }.await;
    
    // Store eligible voters as a comma-separated list of IDs
    let eligible_voters_str = eligible_voter_ids.into_iter().collect::<Vec<_>>().join(",");
    
    // Organize time slots by day
    let day_slots = organize_slots_by_day(&match_response.matches, &timezone, slot_duration);
    
    // Create initial poll with the new time slot voting system
    let active_poll = super::ActivePoll {
        matches: match_response.matches.clone(),
        current_index: 0,
        group_names: group_names.clone(),
        min_per_group,
        required_yes_count,
        responses: HashMap::new(), // Keep for backward compatibility
        slot_responses: HashMap::new(), // New response format
        db_pool: ctx.db_pool.clone(),
        timezone: timezone.clone(),
        eligible_voters: eligible_voters_str.clone(),
        group_members,
        slot_duration,
        display_days,
        current_day: 0, // Start with first day
        day_slots,
    };
    
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
            if let Ok(Some(id)) = row.try_get::<Option<String>, _>("role_id") {
                role_mentions.push(format!("<@&{}>", id));
            }
        }
    }
    
    let role_ping = if !role_mentions.is_empty() {
        // More direct role ping format to ensure mentions trigger notifications
        format!("{} 🗣️ A meeting time has been proposed! Please vote on your availability!", role_mentions.join(" "))
    } else {
        String::new()
    };
    
    // Format the time slot buttons message
    let time_slot_message = format_time_slots(&active_poll);
    
    // Create a new message with role pings instead of editing to ensure mentions trigger properly
    // Send the role pings in a separate message to ensure they trigger notifications
    if !role_ping.is_empty() {
        command.create_followup_message(&ctx.ctx.http, |m| {
            m.content(&role_ping)
        }).await?;
    }
    
    // Then edit the original response with the voting UI
    let message = command.edit_original_interaction_response(&ctx.ctx.http, |m| {
        m.content("");
        
        m.embed(|e| {
            e.title(format!("Vote on Your Availability - Day {} of {}", 
                          active_poll.current_day + 1, 
                          active_poll.day_slots.len()))
                .description(&time_slot_message)
                .color(Color::GOLD)
                .footer(|f| f.text(format!(
                    "Min members per group: {} • Slot duration: {} min • Timezone: {}",
                    min_per_group,
                    slot_duration,
                    timezone
                )))
        })
        .components(|c| {
            // Add time slot selection buttons
            if let Some(slots) = active_poll.day_slots.get(&active_poll.current_day) {
                for chunk in slots.chunks(5) {
                    c.create_action_row(|row| {
                        for slot in chunk {
                            row.create_button(|b| {
                                b.custom_id(format!("slot_{}", slot.id))
                                    .label(format!("{} [0]", &slot.formatted_time)) // Start with zero votes
                                    .style(serenity::model::application::component::ButtonStyle::Success) // Start as selected
                            });
                        }
                        row
                    });
                }
            }
            
            // Add navigation and utility buttons
            c.create_action_row(|row| {
                // Previous day button
                row.create_button(|b| {
                    b.custom_id("prev_day")
                        .label("◀️ Previous Day")
                        .style(serenity::model::application::component::ButtonStyle::Primary)
                        .disabled(active_poll.current_day == 0)
                });
                
                // Next day button
                row.create_button(|b| {
                    b.custom_id("next_day")
                        .label("Next Day ▶️")
                        .style(serenity::model::application::component::ButtonStyle::Primary)
                        .disabled(active_poll.current_day == active_poll.day_slots.len() - 1)
                });
                
                row
            });
            
            // Add select all / clear buttons
            c.create_action_row(|row| {
                row.create_button(|b| {
                    b.custom_id("select_all")
                        .label("Select All")
                        .style(serenity::model::application::component::ButtonStyle::Success)
                })
                .create_button(|b| {
                    b.custom_id("clear_all")
                        .label("Clear All Days")
                        .style(serenity::model::application::component::ButtonStyle::Danger)
                })
                .create_button(|b| {
                    b.custom_id("submit_votes")
                        .label("Submit Votes")
                        .style(serenity::model::application::component::ButtonStyle::Primary)
                })
            })
        })
    }).await?;
    
    // Pre-select all time slots for all eligible voters before storing the poll
    // This makes the default state "available" for all time slots
    {
        let mut modified_poll = active_poll.clone();
        
        // Get all slot IDs from all days
        let all_slot_ids: Vec<String> = modified_poll.day_slots.values()
            .flat_map(|slots| slots.iter().map(|slot| slot.id.clone()))
            .collect();
        
        // For each eligible voter, pre-select all slots
        for voter_id in modified_poll.eligible_voters.split(',').filter(|s| !s.is_empty()) {
            modified_poll.slot_responses.insert(voter_id.to_string(), all_slot_ids.clone());
        }
        
        // Store the poll state with pre-selected slots
        let mut polls = ctx.active_polls.write().await;
        polls.insert(message.id, modified_poll);
    }
    
    Ok(())
}

/// Organize time slots by day
fn organize_slots_by_day(
    matches: &[MatchResult], 
    timezone_str: &str, 
    slot_duration: i64
) -> HashMap<usize, Vec<super::SlotInfo>> {
    let mut day_slots = HashMap::new();
    let tz = chrono_tz::Tz::from_str(timezone_str).unwrap_or(chrono_tz::UTC);
    
    // Convert all match times to slots
    for (match_idx, match_result) in matches.iter().enumerate() {
        // Convert to local timezone for display
        let start_local = match_result.start.with_timezone(&tz);
        let _end_local = match_result.end.with_timezone(&tz);
        
        // Calculate day index (days since epoch for the start date)
        let day_idx = start_local.date_naive().num_days_from_ce() as usize;
        
        // Create time chunks for this match based on slot_duration
        let mut current_time = match_result.start;
        let match_end = match_result.end;
        
        let duration_chunk = chrono::Duration::minutes(slot_duration);
        
        while current_time < match_end {
            let chunk_end = std::cmp::min(current_time + duration_chunk, match_end);
            
            // Format the time for display
            let start_local = current_time.with_timezone(&tz);
            let end_local = chunk_end.with_timezone(&tz);
            
            // Format using shorthand - no minutes if on the hour
            let formatted_time = if start_local.minute() == 0 && end_local.minute() == 0 {
                format!(
                    "{}-{}", 
                    start_local.format("%-I%p"), 
                    end_local.format("%-I%p")
                )
            } else {
                format!(
                    "{}-{}", 
                    start_local.format("%-I:%M%p"), 
                    end_local.format("%-I:%M%p")
                )
            }.to_lowercase().replace(" ", "");
            
            // Create a unique ID for this slot
            let slot_id = format!("{}_{}", match_idx, current_time.timestamp());
            
            // Create the slot info
            let slot = super::SlotInfo {
                id: slot_id,
                start: current_time,
                end: chunk_end,
                formatted_time,
                available_users: match_result.groups.iter()
                    .flat_map(|g| g.available_users.clone())
                    .collect(),
            };
            
            // Add to the day's slots
            day_slots.entry(day_idx).or_insert_with(Vec::new).push(slot);
            
            // Move to next chunk
            current_time = chunk_end;
        }
    }
    
    // Sort slots within each day by start time
    for slots in day_slots.values_mut() {
        slots.sort_by_key(|slot| slot.start);
    }
    
    // Remap the days to be 0-indexed in chronological order
    let day_indices: Vec<usize> = day_slots.keys().cloned().collect();
    if !day_indices.is_empty() {
        let mut remapped_slots = HashMap::new();
        let mut sorted_days: Vec<usize> = day_indices.clone();
        sorted_days.sort();
        
        for (i, day) in sorted_days.iter().enumerate() {
            if let Some(slots) = day_slots.remove(day) {
                remapped_slots.insert(i, slots);
            }
        }
        
        return remapped_slots;
    }
    
    day_slots
}

/// Format time slots for display in the message
fn format_time_slots(poll: &super::ActivePoll) -> String {
    // Get the current day's date
    let current_day_slots = match poll.day_slots.get(&poll.current_day) {
        Some(slots) => slots,
        None => return "No time slots available for this day.".to_string(),
    };
    
    if current_day_slots.is_empty() {
        return "No time slots available for this day.".to_string();
    }
    
    // Get the date from the first slot's start time
    let tz = chrono_tz::Tz::from_str(&poll.timezone).unwrap_or(chrono_tz::UTC);
    let day_date = current_day_slots[0].start.with_timezone(&tz).format("%A, %B %d, %Y").to_string();
    
    let mut message = format!("**{}**\n\nSelect all time slots when you are available:\n\n", day_date);
    
    // Get all users who have submitted votes
    let voted_users: std::collections::HashSet<&String> = poll.slot_responses.keys().collect();
    let total_eligible = poll.eligible_voters.split(',').filter(|s| !s.is_empty()).count();
    
    // Create a voting progress report by group
    message.push_str("**Voting Progress:**\n");
    
    // Calculate votes by group to show progress
    for (idx, (_group_id, members)) in poll.group_members.iter().enumerate() {
        // Try to get the group name if available (using the index as a best effort)
        let group_name = poll.group_names.get(idx)
            .unwrap_or(&format!("Group {}", idx + 1))
            .clone();
            
        let min_required = poll.min_per_group as usize;
        let voted_count = members.iter().filter(|m| voted_users.contains(m)).count();
        let total_count = members.len();
        
        message.push_str(&format!(
            "• **{}**: {}/{} members voted (min required: {})\n",
            group_name,
            voted_count,
            total_count,
            min_required
        ));
    }
    
    message.push_str(&format!("\n**Total: {} of {} members have voted**\n\n", 
        voted_users.len(), 
        total_eligible
    ));
    
    // Time slots are now shown directly in the buttons
    
    // Add instructions
    message.push_str("Click on a time to toggle your availability. All times are selected by default. Green buttons indicate times you're available for. The number in brackets [0] shows how many people have selected that time.\n");
    message.push_str("Use the navigation buttons to switch between days. 'Clear All Days' will mark you as unavailable for all days.\n");
    message.push_str("When you're done, click 'Submit Votes' to lock in your selections. If you've cleared all times, you'll be marked as unavailable.\n\n");
    
    message
}

/// Handle button interactions for scheduling matches
pub async fn handle_component_interaction(
    ctx: HandlerContext,
    component: &mut MessageComponentInteraction
) -> Result<()> {
    let custom_id = component.data.custom_id.clone();
    
    match custom_id.as_str() {
        // Legacy handlers
        "match_yes" | "match_no" => handle_match_vote(ctx, component, custom_id == "match_yes").await,
        "match_confirm" => handle_match_confirm(ctx, component).await,
        
        // New time slot voting handlers
        "prev_day" => handle_prev_day(ctx, component).await,
        "next_day" => handle_next_day(ctx, component).await,
        "select_all" => handle_select_all_slots(ctx, component).await,
        "clear_all" => handle_clear_all_slots(ctx, component).await,
        "submit_votes" => handle_submit_votes(ctx, component).await,
        _ if custom_id.starts_with("slot_") => {
            let slot_id = custom_id.strip_prefix("slot_").unwrap_or("");
            handle_slot_toggle(ctx, component, slot_id).await
        },
        _ => Ok(()),
    }
}

/// Handle voting interactions (Yes/No)
async fn handle_match_vote(
    ctx: HandlerContext,
    component: &mut MessageComponentInteraction,
    is_yes: bool,
) -> Result<()> {
    // Get the poll associated with this message
    let mut polls = ctx.active_polls.write().await;
    let poll = polls.get_mut(&component.message.id)
        .ok_or_else(|| eyre::eyre!("No active poll found for this message"))?;
    
    // Get the voter ID
    let voter_id = component.user.id.to_string();
    
    // Check if the user is an eligible voter (member of one of the selected groups)
    let eligible_voters: Vec<String> = if poll.eligible_voters.is_empty() {
        Vec::new()
    } else {
        poll.eligible_voters.split(',').map(|s| s.to_string()).collect()
    };
    
    if !eligible_voters.contains(&voter_id) {
        // Acknowledge the interaction
        component.create_interaction_response(&ctx.ctx.http, |r| {
            r.kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|m| {
                    m.content("You are not a member of any of the groups in this poll, so you cannot vote.")
                        .ephemeral(true)
                })
        }).await?;
        
        return Ok(());
    }
    
    // Acknowledge the interaction - ignore errors in case already acknowledged
    let _ = component.create_interaction_response(&ctx.ctx.http, |r| {
        r.kind(InteractionResponseType::DeferredUpdateMessage)
    }).await;
    
    // Record the user's response
    poll.responses.insert(voter_id, is_yes);
    
    // Count yes votes for each group
    let mut group_yes_votes = HashMap::new();
    for (group_id, members) in &poll.group_members {
        let group_yes = members.iter()
            .filter(|&member_id| poll.responses.get(member_id).is_some_and(|&vote| vote))
            .count();
        group_yes_votes.insert(*group_id, group_yes);
    }
    
    // Check if each group has at least min(6, total_members) yes votes
    let mut all_groups_have_enough = true;
    for (group_id, members) in &poll.group_members {
        let required_for_group = std::cmp::min(6, members.len());
        let actual_yes = *group_yes_votes.get(group_id).unwrap_or(&0);
        
        if actual_yes < required_for_group {
            all_groups_have_enough = false;
            break;
        }
    }
    
    // Count total yes and no votes for UI display
    let yes_votes = poll.responses.values().filter(|&&v| v).count();
    let _no_votes = poll.responses.values().filter(|&&v| !v).count();
    let _total_voters = eligible_voters.len();
    
    // Check if we have reached the minimum threshold for yes votes from each group
    if all_groups_have_enough {
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
                if let Ok(Some(id)) = row.try_get::<Option<String>, _>("role_id") {
                    role_ids.push(format!("<@&{}>", id));
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
        // Check if it's impossible to get enough yes votes for any group
        let mut impossible_to_get_enough = false;
        
        for members in poll.group_members.values() {
            let required_for_group = std::cmp::min(6, members.len());
            let current_yes = members.iter()
                .filter(|&member_id| poll.responses.get(member_id).is_some_and(|&vote| vote))
                .count();
            
            let remaining_votes = members.iter()
                .filter(|&member_id| !poll.responses.contains_key(member_id))
                .count();
            
            // If current yes + remaining potential votes is less than required, 
            // it's impossible to reach the threshold
            if current_yes + remaining_votes < required_for_group {
                impossible_to_get_enough = true;
                break;
            }
        }
        
        if impossible_to_get_enough {
            // Too many "No" votes, making it impossible to get the required votes per group
            // Move to the next option if available
            if poll.matches.len() > 1 {
                // There are more time options, move to the next one
                let current_index = poll.current_index;
                poll.current_index = (current_index + 1) % poll.matches.len();
                
                // Clear votes when moving to a new option
                poll.responses.clear();
                
                // Update the message to show the next option
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
                        if let Ok(Some(id)) = row.try_get::<Option<String>, _>("role_id") {
                            role_mentions.push(format!("<@&{}>", id));
                        }
                    }
                }
                
                let auto_advance_message = format!(
                    "🔄 Not enough people were available for the previous time slot. Moving to option {} of {}. Please vote again!",
                    poll.current_index + 1,
                    poll.matches.len()
                );
                
                component.message.edit(&ctx.ctx.http, |m| {
                    m.content(&auto_advance_message);
                    
                    m.embed(|e| {
                        e.title(format!("Proposed Meeting Time ({} of {})", 
                                      poll.current_index + 1, 
                                      poll.matches.len()))
                            .description(&match_message)
                            .color(Color::GOLD)
                            .footer(|f| f.text(format!(
                                "Min members per group: {} • {}/{} yes votes needed • Generated at: {}",
                                poll.min_per_group,
                                0, // Reset counter
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
                    })
                }).await?;
            } else {
                // We've tried all options and none worked
                let no_solution_message = "❌ We've gone through all available time options and none received enough votes.";
                
                component.message.edit(&ctx.ctx.http, |m| {
                    m.content(no_solution_message)
                     .embed(|e| {
                        e.title("No Suitable Time Found")
                            .description("We've tried all possible meeting times, but none received enough confirmations. You may want to try again with different groups or ask members to update their availability schedules.")
                            .color(Color::RED)
                     })
                     .components(|c| c) // Clear components
                }).await?;
                
                // Remove the poll from active polls
                polls.remove(&component.message.id);
            }
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
                    if let Ok(Some(id)) = row.try_get::<Option<String>, _>("role_id") {
                        role_mentions.push(format!("<@&{}>", id));
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
                })
            }).await?;
        }
    }
    
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

/// Handle "Previous Day" button click
async fn handle_prev_day(
    ctx: HandlerContext,
    component: &mut MessageComponentInteraction,
) -> Result<()> {
    // Acknowledge the interaction first - this should be done before any processing
    // to avoid "interaction has already been acknowledged" errors
    let _ = component.create_interaction_response(&ctx.ctx.http, |r| {
        r.kind(InteractionResponseType::DeferredUpdateMessage)
    }).await;
    // We use let _ to ignore errors, in case the interaction was already acknowledged
    
    // Get the poll associated with this message
    let mut polls = ctx.active_polls.write().await;
    let poll = polls.get_mut(&component.message.id)
        .ok_or_else(|| eyre::eyre!("No active poll found for this message"))?;
    
    // Verify we're not already on the first day
    if poll.current_day == 0 {
        // Already on first day, nothing to do
        return Ok(());
    }
    
    // Go to previous day
    poll.current_day -= 1;
    
    // Get a clone of the poll before releasing the lock
    let poll_clone = poll.clone();
    drop(polls);
    
    // Update the message with the cloned data
    update_time_slot_message(ctx, component, &poll_clone).await?;
    
    Ok(())
}

/// Handle "Next Day" button click
async fn handle_next_day(
    ctx: HandlerContext,
    component: &mut MessageComponentInteraction,
) -> Result<()> {
    // Acknowledge the interaction first - this should be done before any processing
    // to avoid "interaction has already been acknowledged" errors
    let _ = component.create_interaction_response(&ctx.ctx.http, |r| {
        r.kind(InteractionResponseType::DeferredUpdateMessage)
    }).await;
    // We use let _ to ignore errors, in case the interaction was already acknowledged
    
    // Get the poll associated with this message
    let mut polls = ctx.active_polls.write().await;
    let poll = polls.get_mut(&component.message.id)
        .ok_or_else(|| eyre::eyre!("No active poll found for this message"))?;
    
    // Verify we're not already on the last day
    if poll.current_day >= poll.day_slots.len() - 1 {
        // Already on last day, nothing to do
        return Ok(());
    }
    
    // Go to next day
    poll.current_day += 1;
    
    // Get a clone of the poll before releasing the lock
    let poll_clone = poll.clone();
    drop(polls);
    
    // Update the message with the cloned data
    update_time_slot_message(ctx, component, &poll_clone).await?;
    
    Ok(())
}

/// Handle time slot toggle button click
async fn handle_slot_toggle(
    ctx: HandlerContext,
    component: &mut MessageComponentInteraction,
    slot_id: &str,
) -> Result<()> {
    // Get the voter ID
    let voter_id = component.user.id.to_string();
    
    // Acknowledge the interaction first - this should be done before any processing
    // to avoid "interaction has already been acknowledged" errors
    let _ = component.create_interaction_response(&ctx.ctx.http, |r| {
        r.kind(InteractionResponseType::DeferredUpdateMessage)
    }).await;
    // We use let _ to ignore errors, in case the interaction was already acknowledged
    
    // Get the poll associated with this message
    let mut polls = ctx.active_polls.write().await;
    let poll = polls.get_mut(&component.message.id)
        .ok_or_else(|| eyre::eyre!("No active poll found for this message"))?;
    
    // Check if the user is an eligible voter (member of one of the selected groups)
    let eligible_voters: Vec<String> = if poll.eligible_voters.is_empty() {
        Vec::new()
    } else {
        poll.eligible_voters.split(',').map(|s| s.to_string()).collect()
    };
    
    if !eligible_voters.contains(&voter_id) {
        // Since we've already acknowledged the interaction above, use a follow-up message
        let _ = component.create_followup_message(&ctx.ctx.http, |m| {
            m.content("You are not a member of any of the groups in this poll, so you cannot vote.")
                .ephemeral(true)
        }).await;
        
        return Ok(());
    }
    
    // Toggle this slot for the user
    let user_slots = poll.slot_responses.entry(voter_id.clone()).or_insert_with(Vec::new);
    
    // Toggle - if slot is already selected, remove it; otherwise add it
    if let Some(index) = user_slots.iter().position(|s| s == slot_id) {
        // Remove the slot (deselect)
        user_slots.remove(index);
    } else {
        // Add the slot (select)
        user_slots.push(slot_id.to_string());
    }
    
    // Get a clone of the poll before releasing the lock
    let poll_clone = poll.clone();
    drop(polls);
    
    // Update the message with the cloned data
    update_time_slot_message(ctx, component, &poll_clone).await?;
    
    Ok(())
}

/// Handle "Select All" button click
async fn handle_select_all_slots(
    ctx: HandlerContext,
    component: &mut MessageComponentInteraction,
) -> Result<()> {
    // Get the voter ID
    let voter_id = component.user.id.to_string();
    
    // Acknowledge the interaction first - this should be done before any processing
    // to avoid "interaction has already been acknowledged" errors
    let _ = component.create_interaction_response(&ctx.ctx.http, |r| {
        r.kind(InteractionResponseType::DeferredUpdateMessage)
    }).await;
    // We use let _ to ignore errors, in case the interaction was already acknowledged
    
    // Get the poll associated with this message
    let mut polls = ctx.active_polls.write().await;
    let poll = polls.get_mut(&component.message.id)
        .ok_or_else(|| eyre::eyre!("No active poll found for this message"))?;
    
    // Check if the user is an eligible voter (member of one of the selected groups)
    let eligible_voters: Vec<String> = if poll.eligible_voters.is_empty() {
        Vec::new()
    } else {
        poll.eligible_voters.split(',').map(|s| s.to_string()).collect()
    };
    
    if !eligible_voters.contains(&voter_id) {
        // Since we've already acknowledged the interaction above, use a follow-up message
        let _ = component.create_followup_message(&ctx.ctx.http, |m| {
            m.content("You are not a member of any of the groups in this poll, so you cannot vote.")
                .ephemeral(true)
        }).await;
        
        return Ok(());
    }
    
    // Get all slot IDs for the current day
    let current_day_slots = match poll.day_slots.get(&poll.current_day) {
        Some(slots) => slots,
        None => {
            return Ok(());
        }
    };
    
    // Select all slots for this day
    let user_slots = poll.slot_responses.entry(voter_id.clone()).or_insert_with(Vec::new);
    
    // Add all slots from the current day that aren't already selected
    for slot in current_day_slots {
        if !user_slots.contains(&slot.id) {
            user_slots.push(slot.id.clone());
        }
    }
    
    // Get a clone of the poll before releasing the lock
    let poll_clone = poll.clone();
    drop(polls);
    
    // Update the message with the cloned data
    update_time_slot_message(ctx, component, &poll_clone).await?;
    
    Ok(())
}

/// Handle "Clear All" button click
async fn handle_clear_all_slots(
    ctx: HandlerContext,
    component: &mut MessageComponentInteraction,
) -> Result<()> {
    // Get the voter ID
    let voter_id = component.user.id.to_string();
    
    // Acknowledge the interaction first - this should be done before any processing
    // to avoid "interaction has already been acknowledged" errors
    let _ = component.create_interaction_response(&ctx.ctx.http, |r| {
        r.kind(InteractionResponseType::DeferredUpdateMessage)
    }).await;
    // We use let _ to ignore errors, in case the interaction was already acknowledged
    
    // Get the poll associated with this message
    let mut polls = ctx.active_polls.write().await;
    let poll = polls.get_mut(&component.message.id)
        .ok_or_else(|| eyre::eyre!("No active poll found for this message"))?;
    
    // Check if the user is an eligible voter (member of one of the selected groups)
    let eligible_voters: Vec<String> = if poll.eligible_voters.is_empty() {
        Vec::new()
    } else {
        poll.eligible_voters.split(',').map(|s| s.to_string()).collect()
    };
    
    if !eligible_voters.contains(&voter_id) {
        // Since we've already acknowledged the interaction above, use a follow-up message
        let _ = component.create_followup_message(&ctx.ctx.http, |m| {
            m.content("You are not a member of any of the groups in this poll, so you cannot vote.")
                .ephemeral(true)
        }).await;
        
        return Ok(());
    }
    
    // Clear ALL slots for ALL days
    // Just empty the user's selected slots completely
    poll.slot_responses.insert(voter_id.clone(), Vec::new());
    
    // Get a clone of the poll before releasing the lock
    let poll_clone = poll.clone();
    drop(polls);
    
    // Update the message with the cloned data
    update_time_slot_message(ctx, component, &poll_clone).await?;
    
    Ok(())
}

/// Handle "Submit Votes" button click
/// This locks in the member's selected time slots.
/// If a member submits with no selections, they are marked as unavailable.
async fn handle_submit_votes(
    ctx: HandlerContext,
    component: &mut MessageComponentInteraction,
) -> Result<()> {
    // Get the voter ID
    let voter_id = component.user.id.to_string();
    
    // Get the poll associated with this message
    let mut polls = ctx.active_polls.write().await;
    let poll = polls.get_mut(&component.message.id)
        .ok_or_else(|| eyre::eyre!("No active poll found for this message"))?;
    
    // Check if the user is an eligible voter (member of one of the selected groups)
    let eligible_voters: Vec<String> = if poll.eligible_voters.is_empty() {
        Vec::new()
    } else {
        poll.eligible_voters.split(',').map(|s| s.to_string()).collect()
    };
    
    if !eligible_voters.contains(&voter_id) {
        // Acknowledge the interaction
        component.create_interaction_response(&ctx.ctx.http, |r| {
            r.kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|m| {
                    m.content("You are not a member of any of the groups in this poll, so you cannot vote.")
                        .ephemeral(true)
                })
        }).await?;
        
        return Ok(());
    }
    
    // Acknowledge the interaction first - this should be done before any processing
    let _ = component.create_interaction_response(&ctx.ctx.http, |r| {
        r.kind(InteractionResponseType::DeferredUpdateMessage)
    }).await;
    // Note: We use let _ to ignore errors, in case the interaction was already acknowledged
    
    // Mark this user as having voted - an empty selection means they're unavailable
    // If they didn't select any slots, they'll have an empty vector in slot_responses
    if !poll.slot_responses.contains_key(&voter_id) {
        poll.slot_responses.insert(voter_id.clone(), Vec::new());
    }

    // Confirm to the user that their votes have been submitted
    let _ = component.create_followup_message(&ctx.ctx.http, |m| {
        let selected_count = poll.slot_responses.get(&voter_id)
            .map_or(0, |slots| slots.len());
        
        if selected_count > 0 {
            m.content(format!("Your votes have been submitted! You selected {} time slots.", selected_count))
        } else {
            m.content("Your response has been recorded. You didn't select any time slots, so you're marked as unavailable.")
        }
        .ephemeral(true)
    }).await;

    // Get users who have voted
    let voted_users: std::collections::HashSet<String> = poll.slot_responses.keys().cloned().collect();
    
    // Check if minimum requirements can be met with current votes
    let mut min_requirements_possible = true;
    let mut all_groups_have_enough = true;
    let mut groups_status = Vec::new();
    
    for (idx, (_group_id, members)) in poll.group_members.iter().enumerate() {
        let min_required = poll.min_per_group as usize;
        
        // Get members who selected at least one time slot (not just voted)
        let available_count = members.iter()
            .filter(|m| {
                poll.slot_responses.get(*m)
                    .map_or(false, |slots| !slots.is_empty())
            })
            .count();
        
        let remaining_count = members.len() - voted_users.intersection(&members.iter().cloned().collect()).count();
        
        // Keep track of the group's status for the message
        let group_name = poll.group_names.get(idx)
            .unwrap_or(&format!("Group {}", idx + 1))
            .clone();
        
        // Is it still possible to reach the minimum required members?
        let max_possible_available = available_count + remaining_count;
        
        // If we can't reach the minimum requirement even if all remaining members vote as available,
        // then voting is effectively complete and we know the result will be negative
        if max_possible_available < min_required {
            min_requirements_possible = false;
            groups_status.push((group_name, available_count, min_required, false));
        } else {
            // Otherwise, check if we've met the minimum already
            let min_met = available_count >= min_required;
            groups_status.push((group_name, available_count, min_required, min_met));
            
            if !min_met {
                all_groups_have_enough = false;
            }
        }
    }
    
    // If minimum requirements can't be met, or if all groups have enough votes and they've found an overlapping time slot,
    // we can finish the voting process
    if !min_requirements_possible || all_groups_have_enough {
        // If requirements can't be met, there's no point in finding an optimal slot
        if !min_requirements_possible {
            // Show a message explaining why voting failed
            let mut description = "**Voting Results**\n\n".to_string();
            description.push_str("The meeting cannot be scheduled because not enough members are available.\n\n");
            
            // Add details about each group
            description.push_str("**Group Status:**\n");
            for (group_name, voted_count, min_required, _) in groups_status {
                description.push_str(&format!(
                    "• **{}**: {}/{} members voted (minimum required: {})\n",
                    group_name, voted_count, min_required, min_required
                ));
            }
            
            // Get role mentions for status notification
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
                    if let Ok(Some(id)) = row.try_get::<Option<String>, _>("role_id") {
                        role_mentions.push(format!("<@&{}>", id));
                    }
                }
            }
            
            // Create notification message
            let notification = if !role_mentions.is_empty() {
                format!("❌ {} Meeting scheduling failed due to insufficient availability.", 
                       role_mentions.join(" "))
            } else {
                "❌ Meeting scheduling failed due to insufficient availability.".to_string()
            };
            
            // Update the message to show the failure
            component.message.edit(&ctx.ctx.http, |m| {
                m.content(&notification)
                    .embed(|e| {
                        e.title("Not Enough Members Available")
                            .description(description)
                            .color(Color::RED)
                            .footer(|f| f.text("Members who submitted without selecting any slots are considered unavailable"))
                    })
                    .components(|c| c) // Clear components
            }).await?;
            
            // Remove the poll from active polls
            polls.remove(&component.message.id);
            return Ok(());
        }
        
        // Otherwise, try to find an optimal meeting time
        let optimal_slot = find_optimal_meeting_slot(poll);
        
        if let Some((_day_idx, slot_info, attending_users)) = optimal_slot {
            // We found a good meeting time
            // Format date for display
            let tz = chrono_tz::Tz::from_str(&poll.timezone).unwrap_or(chrono_tz::UTC);
            let day_date = slot_info.start.with_timezone(&tz).format("%A, %B %d, %Y").to_string();
            
            // Create description for the message
            let mut description = format!(
                "**{}**\n**{}**\n\n",
                day_date,
                slot_info.formatted_time
            );
            
            // Add details about each group
            description.push_str("**Group Attendance:**\n");
            for (group_name, voted_count, min_required, min_met) in groups_status {
                description.push_str(&format!(
                    "• **{}**: {}/{} members (minimum required: {}) {}\n",
                    group_name, 
                    voted_count, 
                    min_required,
                    min_required,
                    if min_met { "✅" } else { "❓" }
                ));
            }
            
            description.push_str("\n**Attendees:**\n");
            for user_id in &attending_users {
                description.push_str(&format!("• <@{}>\n", user_id));
            }
            
            // Get role IDs for all groups to ping during announcement
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
                    if let Ok(Some(id)) = row.try_get::<Option<String>, _>("role_id") {
                        role_mentions.push(format!("<@&{}>", id));
                    }
                }
            }
            
            // Create ping message for attendees and roles
            let ping_message = {
                let mut message = "🔔 Meeting confirmed! ".to_string();
                
                // Add role pings
                if !role_mentions.is_empty() {
                    message.push_str(&format!("{} ", role_mentions.join(" ")));
                }
                
                // Add individual attendee pings
                if !attending_users.is_empty() {
                    message.push_str("- Please mark your calendars!");
                } else {
                    message.push_str("Please mark your calendars!");
                }
                
                message
            };
            
            // Update the message to show the confirmation
            component.message.edit(&ctx.ctx.http, |m| {
                m.content(&ping_message)
                    .embed(|e| {
                        e.title("Meeting Time Confirmed!")
                            .description(description)
                            .color(Color::DARK_GREEN)
                            .footer(|f| f.text(format!(
                                "Min members per group: {} • {} attendees • Timezone: {} • Only members who selected this slot are listed as attendees",
                                poll.min_per_group,
                                attending_users.len(),
                                poll.timezone
                            )))
                    })
                    .components(|c| c) // Clear components
            }).await?;
            
            // Remove the poll from active polls
            polls.remove(&component.message.id);
        } else {
            // No suitable meeting time found
            // Get role mentions for notification
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
                    if let Ok(Some(id)) = row.try_get::<Option<String>, _>("role_id") {
                        role_mentions.push(format!("<@&{}>", id));
                    }
                }
            }
            
            // Create notification message
            let notification = if !role_mentions.is_empty() {
                format!("❌ {} No suitable meeting time could be found.", 
                       role_mentions.join(" "))
            } else {
                "❌ No suitable meeting time could be found.".to_string()
            };
            
            component.message.edit(&ctx.ctx.http, |m| {
                m.content(&notification)
                    .embed(|e| {
                        e.title("No Suitable Meeting Time Found")
                            .description("We could not find a time slot where enough members from each group are available. You may want to try again with different parameters or ask members to update their availability.")
                            .color(Color::RED)
                    })
                    .components(|c| c) // Clear components
            }).await?;
            
            // Remove the poll from active polls
            polls.remove(&component.message.id);
        }
    } else {
        // Voting is still in progress, update the UI and let people continue voting
        // Close the polls lock before calling update_time_slot_message
        let poll_clone = poll.clone();
        // Drop the write lock to avoid borrow issues
        drop(polls);
        
        // Now update the message with the cloned poll data
        update_time_slot_message(ctx, component, &poll_clone).await?;
    }
    
    Ok(())
}

/// Update the time slot message UI
async fn update_time_slot_message(
    ctx: HandlerContext,
    component: &mut MessageComponentInteraction,
    poll: &super::ActivePoll,
) -> Result<()> {
    // Format the time slot message
    let time_slot_message = format_time_slots(poll);
    
    // Get the user's selected slots
    let voter_id = component.user.id.to_string();
    let user_selected_slots = poll.slot_responses.get(&voter_id).cloned().unwrap_or_default();
    
    // Instead of using the interaction response, directly edit the message
    // This avoids the "interaction already acknowledged" error
    component.message.edit(&ctx.ctx.http, |m| {
        m.embed(|e| {
            e.title(format!("Vote on Your Availability - Day {} of {}", 
                          poll.current_day + 1, 
                          poll.day_slots.len()))
                .description(&time_slot_message)
                .color(Color::GOLD)
                .footer(|f| f.text(format!(
                    "Min members per group: {} • Slot duration: {} min • Timezone: {}",
                    poll.min_per_group,
                    poll.slot_duration,
                    poll.timezone
                )))
        })
        .components(|c| {
            // Add time slot selection buttons
            if let Some(slots) = poll.day_slots.get(&poll.current_day) {
                for chunk in slots.chunks(5) {
                    c.create_action_row(|row| {
                        for slot in chunk {
                            // Check if this slot is selected by the current user
                            let is_selected = user_selected_slots.contains(&slot.id);
                            
                            // Count votes for this slot
                            let vote_count = poll.slot_responses.values()
                                .filter(|selected_slots| selected_slots.contains(&slot.id))
                                .count();
                                
                            row.create_button(|b| {
                                b.custom_id(format!("slot_{}", slot.id))
                                    .label(format!("{} [{}]", &slot.formatted_time, vote_count))
                                    .style(if is_selected {
                                        serenity::model::application::component::ButtonStyle::Success // Green for selected
                                    } else {
                                        serenity::model::application::component::ButtonStyle::Secondary // Neutral/gray for not selected
                                    })
                            });
                        }
                        row
                    });
                }
            }
            
            // Add navigation and utility buttons
            c.create_action_row(|row| {
                // Previous day button
                row.create_button(|b| {
                    b.custom_id("prev_day")
                        .label("◀️ Previous Day")
                        .style(serenity::model::application::component::ButtonStyle::Primary)
                        .disabled(poll.current_day == 0)
                });
                
                // Next day button
                row.create_button(|b| {
                    b.custom_id("next_day")
                        .label("Next Day ▶️")
                        .style(serenity::model::application::component::ButtonStyle::Primary)
                        .disabled(poll.current_day >= poll.day_slots.len() - 1)
                });
                
                row
            });
            
            // Add select all / clear buttons
            c.create_action_row(|row| {
                row.create_button(|b| {
                    b.custom_id("select_all")
                        .label("Select All")
                        .style(serenity::model::application::component::ButtonStyle::Success)
                })
                .create_button(|b| {
                    b.custom_id("clear_all")
                        .label("Clear All Days")
                        .style(serenity::model::application::component::ButtonStyle::Danger)
                })
                .create_button(|b| {
                    b.custom_id("submit_votes")
                        .label("Submit Votes")
                        .style(serenity::model::application::component::ButtonStyle::Primary)
                })
            })
        })
    }).await?;
    
    Ok(())
}

/// Find the optimal meeting slot based on votes
fn find_optimal_meeting_slot(poll: &super::ActivePoll) -> Option<(usize, super::SlotInfo, Vec<String>)> {
    // A map to track votes for each slot
    let mut slot_votes: HashMap<String, Vec<String>> = HashMap::new();
    
    // Get the set of users who submitted votes but selected nothing (marked as unavailable)
    let _unavailable_users: std::collections::HashSet<String> = poll.slot_responses.iter()
        .filter(|(_, slots)| slots.is_empty())
        .map(|(user_id, _)| user_id.clone())
        .collect();
    
    // Collect all votes from users who selected at least one slot
    for (user_id, selected_slots) in &poll.slot_responses {
        // Skip users who submitted with no selections (they're marked as unavailable)
        if selected_slots.is_empty() {
            continue;
        }
        
        for slot_id in selected_slots {
            slot_votes.entry(slot_id.clone())
                .or_insert_with(Vec::new)
                .push(user_id.clone());
        }
    }
    
    // Now find the best slot by day
    let mut best_slot: Option<(usize, super::SlotInfo, Vec<String>)> = None;
    let mut max_votes = 0;
    
    // Check each day
    for (day_idx, slots) in &poll.day_slots {
        for slot in slots {
            let voters = slot_votes.get(&slot.id).cloned().unwrap_or_default();
            let vote_count = voters.len();
            
            // Check if this slot meets the minimum requirements for each group
            let mut group_counts: HashMap<uuid::Uuid, usize> = HashMap::new();
            
            for (group_id, members) in &poll.group_members {
                // Count members who explicitly selected this slot
                let group_vote_count = members.iter()
                    .filter(|member_id| voters.contains(member_id))
                    .count();
                
                group_counts.insert(*group_id, group_vote_count);
            }
            
            // Check if all groups meet minimum requirement
            let all_groups_meet_min = group_counts.iter()
                .all(|(_, &count)| count >= poll.min_per_group as usize);
            
            // Update best slot if this one is better
            if all_groups_meet_min && vote_count > max_votes {
                max_votes = vote_count;
                best_slot = Some((*day_idx, slot.clone(), voters));
            }
        }
    }
    
    best_slot
}

/// Format a single match option for display
fn format_match_option(poll: &super::ActivePoll, index: usize, _required_yes: usize) -> String {
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
        // Find the group in the poll's group_members
        let group_members = poll.group_members.get(&group.id);
        let required_for_group = if let Some(members) = group_members {
            std::cmp::min(6, members.len())
        } else {
            6 // Default if not found
        };
        
        // Count yes votes for this group
        let group_yes_votes = group.available_users.iter()
            .filter(|&user_id| poll.responses.get(user_id).is_some_and(|&vote| vote))
            .count();
        
        description.push_str(&format!(
            "• **{}**: {} {} ({}/{} yes votes needed)\n",
            group.name,
            group.count,
            if group.count == 1 { "member" } else { "members" },
            group_yes_votes,
            required_for_group
        ));
        
        // List the available users for each group
        if !group.available_users.is_empty() {
            for user_id in &group.available_users {
                // Get vote status if they've voted
                let vote_status = match poll.responses.get(user_id) {
                    Some(true) => " ✅",
                    Some(false) => " ❌",
                    None => ""
                };
                
                description.push_str(&format!("  - <@{}>{}\n", user_id, vote_status));
            }
        }
    }
    
    // Add responses if there are any from users not in the available lists
    let yes_users: Vec<&String> = poll.responses.iter()
        .filter(|&(user_id, &v)| v && !match_result.groups.iter().any(|g| g.available_users.contains(user_id)))
        .map(|(user_id, _)| user_id)
        .collect();
    
    let no_users: Vec<&String> = poll.responses.iter()
        .filter(|&(user_id, &v)| !v && !match_result.groups.iter().any(|g| g.available_users.contains(user_id)))
        .map(|(user_id, _)| user_id)
        .collect();
    
    if !yes_users.is_empty() || !no_users.is_empty() {
        description.push_str("\n**Additional Responses:**\n");
        
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
    
    // Calculate progress for each group
    let mut vote_requirements = String::new();
    let mut all_groups_have_enough = true;
    
    for (group_id, group_name) in match_result.groups.iter().map(|g| (g.id, g.name.clone())) {
        if let Some(members) = poll.group_members.get(&group_id) {
            let required_for_group = std::cmp::min(6, members.len());
            let group_yes_votes = members.iter()
                .filter(|&user_id| poll.responses.get(user_id).is_some_and(|&vote| vote))
                .count();
            
            vote_requirements.push_str(&format!(
                "• Group {}: {}/{} yes votes\n",
                group_name,
                group_yes_votes,
                required_for_group
            ));
            
            if group_yes_votes < required_for_group {
                all_groups_have_enough = false;
            }
        }
    }
    
    if !vote_requirements.is_empty() {
        description.push_str("\n**Voting Progress:**\n");
        description.push_str(&vote_requirements);
        
        if all_groups_have_enough {
            description.push_str("\n✅ **All groups have enough votes!**");
        } else {
            description.push_str("\n⏳ **Waiting for more votes...**");
        }
    }
    
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
            Some(row) => row.try_get::<String, _>("timezone").ok(),
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
        description.push('\n');
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