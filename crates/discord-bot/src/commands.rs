use serenity::builder::CreateApplicationCommands;
use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::{CommandResult, StandardFramework};
use serenity::model::prelude::*;
use serenity::prelude::*;

pub mod schedule;

#[group]
#[commands(ping)]
struct General;

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, "Pong!").await?;
    Ok(())
}

/// Register all commands for the bot.
///
/// This function creates and registers all of the slash commands that the bot
/// will respond to, including their options, descriptions, and permissions.
///
/// # Arguments
///
/// * `commands` - A mutable reference to a CreateApplicationCommands object
///                that will be modified to include our custom commands.
///
/// # Returns
///
/// The same CreateApplicationCommands object with our commands added.
pub fn register_commands(commands: &mut CreateApplicationCommands) -> &mut CreateApplicationCommands {
    // Create the schedule command
    commands.create_application_command(|command| {
        *command = schedule::create_schedule_command();
        command
    });
    
    // Create the group command
    commands.create_application_command(|command| {
        *command = schedule::group_command();
        command
    });
    
    // Create the match command 
    commands.create_application_command(|command| {
        *command = schedule::match_command();
        command
    });
    
    commands
}