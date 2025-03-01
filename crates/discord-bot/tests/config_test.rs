use timesync_discord_bot::config::BotConfig;

#[test]
fn test_command_prefix_default() {
    // Test the default command prefix
    let config = BotConfig {
        token: "test_token".to_string(),
        application_id: 12345,
        web_base_url: "http://localhost".to_string(),
        database_url: "postgres://localhost".to_string(),
        command_prefix: None,
    };
    
    assert_eq!(config.command_prefix(), "!");
}

#[test]
fn test_command_prefix_custom() {
    // Test a custom command prefix
    let config = BotConfig {
        token: "test_token".to_string(),
        application_id: 12345,
        web_base_url: "http://localhost".to_string(),
        database_url: "postgres://localhost".to_string(),
        command_prefix: Some("/".to_string()),
    };
    
    assert_eq!(config.command_prefix(), "/");
}