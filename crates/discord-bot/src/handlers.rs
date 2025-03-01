use serenity::{
    async_trait,
    model::{
        application::interaction::{
            Interaction, 
            InteractionResponseType,
        },
        gateway::Ready,
        id::MessageId,
    },
    prelude::*,
};
use sqlx::PgPool;
use tracing::{error, info};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use timesync_core::models::discord::MatchResult;

pub mod schedule;

use crate::config::BotConfig;

/// Main Discord handler that processes all events.
///
/// This handler is responsible for responding to Discord events like
/// ready events and commands/interactions. It maintains access to the 
/// bot configuration and database connection.
pub struct Handler {
    config: BotConfig,
    db_pool: PgPool,
    active_polls: Arc<RwLock<HashMap<MessageId, ActivePoll>>>,
}

impl Handler {
    /// Create a new handler
    pub fn new(config: BotConfig, db_pool: PgPool) -> Self {
        Self { 
            config, 
            db_pool,
            active_polls: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    /// Handle ready events (when bot connects to Discord)
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);

        // Register commands globally (visible in all servers)
        if let Err(why) = serenity::model::application::command::Command::set_global_application_commands(&ctx.http, |commands| {
            // Register all commands
            crate::commands::register_commands(commands)
        }).await {
            error!("Error registering global commands: {:?}", why);
        } else {
            info!("Global commands registered successfully!");
        }
    }

    /// Handle interactions (slash commands, buttons, etc.)
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::ApplicationCommand(command) => {
                info!("Received command: {}", command.data.name);
                
                // Create a context with shared data
                let handler_ctx = HandlerContext {
                    ctx: ctx.clone(),
                    config: self.config.clone(),
                    db_pool: self.db_pool.clone(),
                    active_polls: self.active_polls.clone(),
                };

                let result = match command.data.name.as_str() {
                    "schedule" => schedule::handle_schedule_command(handler_ctx, &command).await,
                    "group" => schedule::handle_group_command(handler_ctx, &command).await,
                    "match" => schedule::handle_match_command(handler_ctx, &command).await,
                    _ => {
                        error!("Unknown command: {}", command.data.name);
                        Err(eyre::eyre!("Unknown command"))
                    }
                };

                if let Err(e) = result {
                    error!("Error handling command: {:?}", e);
                    
                    // Try to respond with error
                    if let Err(why) = command
                        .create_interaction_response(&ctx.http, |r| {
                            r.kind(InteractionResponseType::ChannelMessageWithSource)
                                .interaction_response_data(|m| {
                                    m.content(format!("Error: {}", e))
                                        .ephemeral(true)
                                })
                        })
                        .await
                    {
                        error!("Failed to send error response: {:?}", why);
                    }
                }
            },
            Interaction::MessageComponent(mut component) => {
                let handler_ctx = HandlerContext {
                    ctx: ctx.clone(),
                    config: self.config.clone(),
                    db_pool: self.db_pool.clone(),
                    active_polls: self.active_polls.clone(),
                };
                
                if let Err(e) = schedule::handle_component_interaction(handler_ctx, &mut component).await {
                    error!("Error handling component interaction: {:?}", e);
                    
                    if let Err(why) = component
                        .create_interaction_response(&ctx.http, |r| {
                            r.kind(InteractionResponseType::ChannelMessageWithSource)
                                .interaction_response_data(|m| {
                                    m.content(format!("Error: {}", e))
                                        .ephemeral(true)
                                })
                        })
                        .await
                    {
                        error!("Failed to send error response: {:?}", why);
                    }
                }
            },
            _ => {}
        }
    }
}

/// Represents an active poll for a scheduling match
#[derive(Debug, Clone)]
pub struct ActivePoll {
    pub matches: Vec<MatchResult>,
    pub current_index: usize,
    pub group_names: Vec<String>,
    pub min_per_group: i64,
    pub required_yes_count: usize,
    pub responses: HashMap<String, bool>, // user_id -> yes/no
}

/// Shared context for command handlers.
///
/// This struct provides a convenient way to pass the Discord context,
/// bot configuration, and database connection to command handlers.
pub struct HandlerContext {
    pub ctx: Context,
    pub config: BotConfig,
    pub db_pool: PgPool,
    pub active_polls: Arc<RwLock<HashMap<MessageId, ActivePoll>>>,
}