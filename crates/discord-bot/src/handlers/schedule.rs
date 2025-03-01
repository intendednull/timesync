use chrono::Utc;
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
use timesync_core::models::discord::CreateDiscordGroupRequest;
use std::collections::HashMap;

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
    let server_id = command.guild_id
        .ok_or_else(|| eyre::eyre!("Command must be used in a server"))?
        .to_string();
    
    // Create the request payload
    let request = CreateDiscordGroupRequest {
        name: name.clone(),
        server_id,
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
    
    // Respond to the interaction
    command.create_interaction_response(&ctx.ctx.http, |r| {
        r.kind(InteractionResponseType::ChannelMessageWithSource)
            .interaction_response_data(|m| {
                m.embed(|e| {
                    e.title("Scheduling Group Created")
                        .description(format!("Group **{}** has been created with {} members", name, member_ids.len()))
                        .field("Members", format_member_list(&member_ids), false)
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
        
        description.push_str(&format!("**{}** - {} {}\n", 
            group.name, 
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
                        .footer(|f| f.text("Use /group info <name> to see details about a specific group"))
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
    
    // Get the guild (server) ID
    let server_id = command.guild_id
        .ok_or_else(|| eyre::eyre!("Command must be used in a server"))?
        .to_string();
    
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
    
    // Find the group by name in this server
    let group_id = sqlx::query!(
        "SELECT id FROM discord_groups WHERE name = $1 AND server_id = $2",
        name,
        server_id
    )
    .fetch_optional(&ctx.db_pool)
    .await?
    .ok_or_else(|| eyre::eyre!("Group not found"))?
    .id;
    
    // Make the request to update the group
    let update_request = timesync_core::models::discord::UpdateDiscordGroupRequest {
        name: None,
        add_member_ids: Some(member_ids.clone()),
        remove_member_ids: None,
    };
    
    // Make API request to update the group
    let client = reqwest::Client::new();
    let response = client.put(format!("{}/api/discord/groups/{}", ctx.config.web_base_url, group_id))
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
    
    // Get the guild (server) ID
    let server_id = command.guild_id
        .ok_or_else(|| eyre::eyre!("Command must be used in a server"))?
        .to_string();
    
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
    
    // Find the group by name in this server
    let group_id = sqlx::query!(
        "SELECT id FROM discord_groups WHERE name = $1 AND server_id = $2",
        name,
        server_id
    )
    .fetch_optional(&ctx.db_pool)
    .await?
    .ok_or_else(|| eyre::eyre!("Group not found"))?
    .id;
    
    // Make the request to update the group
    let update_request = timesync_core::models::discord::UpdateDiscordGroupRequest {
        name: None,
        add_member_ids: None,
        remove_member_ids: Some(member_ids.clone()),
    };
    
    // Make API request to update the group
    let client = reqwest::Client::new();
    let response = client.put(format!("{}/api/discord/groups/{}", ctx.config.web_base_url, group_id))
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
    
    // Respond to the interaction
    command.create_interaction_response(&ctx.ctx.http, |r| {
        r.kind(InteractionResponseType::ChannelMessageWithSource)
            .interaction_response_data(|m| {
                m.embed(|e| {
                    e.title(format!("Group: {}", group.name))
                        .description(format!(
                            "This group has {} members, {} of which have availability schedules.",
                            members.len(),
                            with_schedule
                        ))
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
    
    // Create initial poll with first match
    let active_poll = super::ActivePoll {
        matches: match_response.matches.clone(),
        current_index: 0,
        group_names: group_names.clone(),
        min_per_group,
        required_yes_count,
        responses: HashMap::new(),
    };
    
    // Create a formatted response with just the first match
    let first_match_message = format_match_option(&active_poll, 0, required_yes_count);
    
    // Edit the original response with the first match
    let message = command.edit_original_interaction_response(&ctx.ctx.http, |m| {
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
        
        // Format confirmed time
        let start_time = match_result.start.format("%a, %b %d at %H:%M");
        let end_time = match_result.end.format("%H:%M");
        
        // Collect attendees for the ping message
        let attendees: Vec<String> = poll.responses.iter()
            .filter(|&(_, is_attending)| *is_attending)
            .map(|(user_id, _)| format!("<@{}>", user_id))
            .collect();
            
        // Create ping message for attendees
        let ping_message = if !attendees.is_empty() {
            format!("🔔 Meeting attendees: {} - Please mark your calendars!", attendees.join(" "))
        } else {
            "No confirmed attendees yet.".to_string()
        };
        
        let mut description = format!(
            "The meeting time has been confirmed!\n\n**{}** - **{}** UTC\n\n",
            start_time, end_time
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
    
    // Format the time
    let start_time = match_result.start.format("%a, %b %d at %H:%M");
    let end_time = match_result.end.format("%H:%M");
    
    let mut description = format!(
        "**Proposed Time:** {} - {} UTC\n\n",
        start_time, end_time
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