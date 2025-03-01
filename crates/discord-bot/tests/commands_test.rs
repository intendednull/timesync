use serenity::builder::CreateApplicationCommands;
use timesync_discord_bot::commands;

#[test]
fn test_register_commands() {
    // Test that the commands registration function works without panicking
    let mut commands = CreateApplicationCommands::default();
    commands::register_commands(&mut commands);
    
    // Just test that it doesn't panic
    // Actual command testing is complex due to opaque builder pattern
}