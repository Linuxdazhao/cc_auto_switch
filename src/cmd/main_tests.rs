#[cfg(test)]
mod tests {
    use crate::cmd::cli::{Cli, Commands};
    use crate::cmd::config::{ConfigStorage, Configuration, EnvironmentConfig};
    use crate::cmd::main::*;
    use clap::Parser;

    /// Helper function to create a test configuration
    fn create_test_config(alias: &str, token: &str, url: &str) -> Configuration {
        Configuration {
            alias_name: alias.to_string(),
            token: token.to_string(),
            url: url.to_string(),
            model: None,
            small_fast_model: None,
            max_thinking_tokens: None,
            api_timeout_ms: None,
            claude_code_disable_nonessential_traffic: None,
            anthropic_default_sonnet_model: None,
            anthropic_default_opus_model: None,
            anthropic_default_haiku_model: None,
        }
    }

    /// Helper function to create a full test configuration with all fields
    fn create_full_test_config(
        alias: &str,
        token: &str,
        url: &str,
        model: Option<&str>,
        small_fast_model: Option<&str>,
    ) -> Configuration {
        Configuration {
            alias_name: alias.to_string(),
            token: token.to_string(),
            url: url.to_string(),
            model: model.map(String::from),
            small_fast_model: small_fast_model.map(String::from),
            max_thinking_tokens: None,
            api_timeout_ms: None,
            claude_code_disable_nonessential_traffic: None,
            anthropic_default_sonnet_model: None,
            anthropic_default_opus_model: None,
            anthropic_default_haiku_model: None,
        }
    }

    // AddCommandParams Tests
    #[test]
    fn test_add_command_params_creation() {
        use crate::cmd::types::AddCommandParams;

        let params = AddCommandParams {
            alias_name: "test".to_string(),
            token: Some("sk-ant-test".to_string()),
            url: Some("https://api.test.com".to_string()),
            model: None,
            small_fast_model: None,
            max_thinking_tokens: None,
            api_timeout_ms: None,
            claude_code_disable_nonessential_traffic: None,
            anthropic_default_sonnet_model: None,
            anthropic_default_opus_model: None,
            anthropic_default_haiku_model: None,
            force: false,
            interactive: false,
            token_arg: None,
            url_arg: None,
        };

        assert_eq!(params.alias_name, "test");
        assert_eq!(params.token, Some("sk-ant-test".to_string()));
        assert_eq!(params.url, Some("https://api.test.com".to_string()));
        assert!(!params.force);
        assert!(!params.interactive);
    }

    // handle_switch_command Tests
    #[test]
    #[ignore] // Skip this test as it requires external command execution
    fn test_handle_switch_command_cc_default() {
        // Test switching to 'cc' (default configuration)
        let result = handle_switch_command(Some("cc"));

        // Function should complete without panicking
        // Note: This test will launch Claude CLI in a real environment
        // In a proper test, we'd want to mock the external command execution
        match result {
            Ok(_) => {
                // Success case - command executed
            }
            Err(e) => {
                // Error case might occur if claude CLI is not installed
                let error_msg = e.to_string();
                // Should be a meaningful error about the external command
                assert!(!error_msg.is_empty());
            }
        }
    }

    #[test]
    fn test_handle_switch_command_nonexistent_alias() {
        // Test switching to a non-existent alias
        let result = handle_switch_command(Some("nonexistent-alias"));

        assert!(result.is_err(), "Should fail for non-existent alias");
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("not found"));
        assert!(error_msg.contains("nonexistent-alias"));
    }

    #[test]
    fn test_handle_switch_command_empty_alias() {
        // Test switching with empty string alias
        let result = handle_switch_command(Some(""));

        assert!(result.is_err(), "Should fail for empty alias");
    }

    #[test]
    fn test_handle_switch_command_none_interactive_mode() {
        // Test switching with None (should trigger interactive mode)
        let result = handle_switch_command(None);

        // This should try to enter interactive mode, which may fail in test environment
        match result {
            Ok(_) => {
                // Success case - interactive mode worked
            }
            Err(e) => {
                // Error case - interactive mode failed (expected in test environment)
                let error_msg = e.to_string();
                assert!(!error_msg.is_empty());
            }
        }
    }

    // CLI Command Parsing Tests
    #[test]
    fn test_cli_add_command_parsing() {
        let args = vec![
            "cc-switch",
            "add",
            "my-config",
            "sk-ant-test-token",
            "https://api.test.com",
        ];

        let cli = Cli::try_parse_from(args).expect("Should parse add command");

        match cli.command {
            Some(Commands::Add {
                alias_name,
                token_arg,
                url_arg,
                force,
                interactive,
                token,
                url,
                model,
                small_fast_model,
                max_thinking_tokens,
                api_timeout_ms,
                claude_code_disable_nonessential_traffic,
                anthropic_default_sonnet_model,
                anthropic_default_opus_model,
                anthropic_default_haiku_model,
            }) => {
                assert_eq!(alias_name, "my-config");
                assert_eq!(token_arg, Some("sk-ant-test-token".to_string()));
                assert_eq!(url_arg, Some("https://api.test.com".to_string()));
                assert!(!force);
                assert!(!interactive);
                assert_eq!(token, None);
                assert_eq!(url, None);
                assert_eq!(model, None);
                assert_eq!(small_fast_model, None);
            }
            _ => panic!("Expected Add command"),
        }
    }

    #[test]
    fn test_cli_add_command_with_flags() {
        let args = vec![
            "cc-switch",
            "add",
            "my-config",
            "-t",
            "sk-ant-flag-token",
            "-u",
            "https://flag.api.com",
            "--force",
            "--interactive",
        ];

        let cli = Cli::try_parse_from(args).expect("Should parse add command with flags");

        match cli.command {
            Some(Commands::Add {
                alias_name,
                token,
                url,
                force,
                interactive,
                token_arg,
                url_arg,
                model,
                small_fast_model,
                max_thinking_tokens,
                api_timeout_ms,
                claude_code_disable_nonessential_traffic,
                anthropic_default_sonnet_model,
                anthropic_default_opus_model,
                anthropic_default_haiku_model,
            }) => {
                assert_eq!(alias_name, "my-config");
                assert_eq!(token, Some("sk-ant-flag-token".to_string()));
                assert_eq!(url, Some("https://flag.api.com".to_string()));
                assert!(force);
                assert!(interactive);
                assert_eq!(token_arg, None);
                assert_eq!(url_arg, None);
                assert_eq!(model, None);
                assert_eq!(small_fast_model, None);
            }
            _ => panic!("Expected Add command"),
        }
    }

    #[test]
    fn test_cli_add_command_with_model_flags() {
        let args = vec![
            "cc-switch",
            "add",
            "model-config",
            "-t",
            "sk-ant-model-token",
            "-u",
            "https://model.api.com",
            "-m",
            "claude-3-5-sonnet-20241022",
            "--small-fast-model",
            "claude-3-haiku-20240307",
        ];

        let cli = Cli::try_parse_from(args).expect("Should parse add command with model flags");

        match cli.command {
            Some(Commands::Add {
                alias_name,
                token,
                url,
                model,
                small_fast_model,
                max_thinking_tokens,
                ..
            }) => {
                assert_eq!(alias_name, "model-config");
                assert_eq!(token, Some("sk-ant-model-token".to_string()));
                assert_eq!(url, Some("https://model.api.com".to_string()));
                assert_eq!(model, Some("claude-3-5-sonnet-20241022".to_string()));
                assert_eq!(
                    small_fast_model,
                    Some("claude-3-haiku-20240307".to_string())
                );
            }
            _ => panic!("Expected Add command"),
        }
    }

    #[test]
    fn test_cli_remove_command_single() {
        let args = vec!["cc-switch", "remove", "config-to-remove"];

        let cli = Cli::try_parse_from(args).expect("Should parse remove command");

        match cli.command {
            Some(Commands::Remove { alias_names }) => {
                assert_eq!(alias_names, vec!["config-to-remove"]);
            }
            _ => panic!("Expected Remove command"),
        }
    }

    #[test]
    fn test_cli_remove_command_multiple() {
        let args = vec!["cc-switch", "remove", "config1", "config2", "config3"];

        let cli = Cli::try_parse_from(args).expect("Should parse remove command");

        match cli.command {
            Some(Commands::Remove { alias_names }) => {
                assert_eq!(alias_names, vec!["config1", "config2", "config3"]);
            }
            _ => panic!("Expected Remove command"),
        }
    }

    #[test]
    fn test_cli_use_command() {
        let args = vec!["cc-switch", "use", "my-config"];

        let cli = Cli::try_parse_from(args).expect("Should parse use command");

        match cli.command {
            Some(Commands::Use { alias_name }) => {
                assert_eq!(alias_name, "my-config");
            }
            _ => panic!("Expected Use command"),
        }
    }

    #[test]
    fn test_cli_switch_alias_command() {
        let args = vec!["cc-switch", "switch", "my-config"];

        let cli = Cli::try_parse_from(args).expect("Should parse switch (alias) command");

        match cli.command {
            Some(Commands::Use { alias_name }) => {
                assert_eq!(alias_name, "my-config");
            }
            _ => panic!("Expected Use command (via switch alias)"),
        }
    }

    #[test]
    fn test_cli_list_command() {
        let args = vec!["cc-switch", "list"];

        let cli = Cli::try_parse_from(args).expect("Should parse list command");

        match cli.command {
            Some(Commands::List { plain: _ }) => {
                // Test passes if we get List command
            }
            _ => panic!("Expected List command"),
        }
    }

    #[test]
    fn test_cli_current_command() {
        let args = vec!["cc-switch", "current"];

        let cli = Cli::try_parse_from(args).expect("Should parse current command");

        match cli.command {
            Some(Commands::Current) => {
                // Test passes if we get Current command
            }
            _ => panic!("Expected Current command"),
        }
    }

    #[test]
    fn test_cli_completion_command() {
        let args = vec!["cc-switch", "completion", "fish"];

        let cli = Cli::try_parse_from(args).expect("Should parse completion command");

        match cli.command {
            Some(Commands::Completion { shell }) => {
                assert_eq!(shell, "fish");
            }
            _ => panic!("Expected Completion command"),
        }
    }

    #[test]
    fn test_cli_alias_command() {
        let args = vec!["cc-switch", "alias", "bash"];

        let cli = Cli::try_parse_from(args).expect("Should parse alias command");

        match cli.command {
            Some(Commands::Alias { shell }) => {
                assert_eq!(shell, "bash".to_string());
            }
            _ => panic!("Expected Alias command"),
        }
    }

    #[test]
    fn test_cli_list_aliases_flag() {
        let args = vec!["cc-switch", "--list-aliases"];

        let cli = Cli::try_parse_from(args).expect("Should parse --list-aliases flag");

        assert!(cli.list_aliases, "Should have list_aliases flag set");
        assert!(
            cli.command.is_none(),
            "Should have no subcommand when using --list-aliases"
        );
    }

    #[test]
    fn test_cli_help_flag() {
        let result = Cli::try_parse_from(vec!["cc-switch", "--help"]);

        // Help flag causes early exit with help text
        assert!(result.is_err(), "Help flag should cause early exit");
    }

    #[test]
    fn test_cli_invalid_command() {
        let result = Cli::try_parse_from(vec!["cc-switch", "invalid-command"]);

        assert!(result.is_err(), "Invalid command should fail to parse");
    }

    // Integration Tests for Command Logic
    #[test]
    fn test_environment_config_generation() {
        let config = create_test_config("test", "sk-ant-test", "https://api.test.com");
        let env_config = EnvironmentConfig::from_config(&config);
        let env_tuples = env_config.as_env_tuples();

        assert_eq!(env_tuples.len(), 2);
        assert!(env_tuples.contains(&(
            "ANTHROPIC_AUTH_TOKEN".to_string(),
            "sk-ant-test".to_string()
        )));
        assert!(env_tuples.contains(&(
            "ANTHROPIC_BASE_URL".to_string(),
            "https://api.test.com".to_string()
        )));
    }

    #[test]
    fn test_environment_config_generation_with_models() {
        let config = create_full_test_config(
            "full-test",
            "sk-ant-full-test",
            "https://full-test.api.com",
            Some("claude-3-5-sonnet-20241022"),
            Some("claude-3-haiku-20240307"),
        );
        let env_config = EnvironmentConfig::from_config(&config);
        let env_tuples = env_config.as_env_tuples();

        assert_eq!(env_tuples.len(), 4);
        assert!(env_tuples.contains(&(
            "ANTHROPIC_AUTH_TOKEN".to_string(),
            "sk-ant-full-test".to_string()
        )));
        assert!(env_tuples.contains(&(
            "ANTHROPIC_BASE_URL".to_string(),
            "https://full-test.api.com".to_string()
        )));
        assert!(env_tuples.contains(&(
            "ANTHROPIC_MODEL".to_string(),
            "claude-3-5-sonnet-20241022".to_string()
        )));
        assert!(env_tuples.contains(&(
            "ANTHROPIC_SMALL_FAST_MODEL".to_string(),
            "claude-3-haiku-20240307".to_string()
        )));
    }

    #[test]
    fn test_empty_environment_config() {
        let env_config = EnvironmentConfig::empty();
        let env_tuples = env_config.as_env_tuples();

        assert!(env_tuples.is_empty());
    }

    // Error Handling Tests
    #[test]
    fn test_cli_add_missing_alias() {
        let result = Cli::try_parse_from(vec!["cc-switch", "add"]);

        assert!(result.is_err(), "Should fail when alias name is missing");
    }

    #[test]
    fn test_cli_remove_missing_alias() {
        let result = Cli::try_parse_from(vec!["cc-switch", "remove"]);

        assert!(
            result.is_err(),
            "Should fail when no alias names provided to remove"
        );
    }

    #[test]
    fn test_cli_use_missing_alias() {
        let result = Cli::try_parse_from(vec!["cc-switch", "use"]);

        assert!(
            result.is_err(),
            "Should fail when alias name is missing for use command"
        );
    }

    #[test]
    fn test_cli_completion_default_shell() {
        let result = Cli::try_parse_from(vec!["cc-switch", "completion"]);

        assert!(
            result.is_ok(),
            "Should succeed with default shell for completion command"
        );

        if let Ok(cli) = result {
            match cli.command {
                Some(Commands::Completion { shell }) => {
                    assert_eq!(shell, "fish", "Should default to fish shell");
                }
                _ => panic!("Expected Completion command"),
            }
        }
    }

    // Edge Cases Tests
    #[test]
    fn test_cli_empty_args() {
        let result = Cli::try_parse_from(vec!["cc-switch"]);

        // Should succeed but with no command (will trigger default behavior)
        assert!(result.is_ok(), "Empty args should parse successfully");
        let cli = result.unwrap();
        assert!(
            cli.command.is_none(),
            "Should have no command with empty args"
        );
        assert!(!cli.list_aliases, "Should not have list_aliases flag");
    }

    #[test]
    fn test_cli_with_special_characters_in_alias() {
        let args = vec![
            "cc-switch",
            "add",
            "test-config_123",
            "sk-ant-test",
            "https://api.test.com",
        ];

        let result = Cli::try_parse_from(args);
        assert!(
            result.is_ok(),
            "Should handle special characters in alias names"
        );

        if let Ok(cli) = result {
            if let Some(Commands::Add { alias_name, .. }) = cli.command {
                assert_eq!(alias_name, "test-config_123");
            }
        }
    }

    #[test]
    fn test_cli_unicode_in_arguments() {
        let args = vec![
            "cc-switch",
            "add",
            "测试-config",
            "sk-ant-测试",
            "https://αpi.测试.com",
        ];

        let result = Cli::try_parse_from(args);
        assert!(result.is_ok(), "Should handle Unicode in arguments");

        if let Ok(cli) = result {
            if let Some(Commands::Add {
                alias_name,
                token_arg,
                url_arg,
                ..
            }) = cli.command
            {
                assert_eq!(alias_name, "测试-config");
                assert_eq!(token_arg, Some("sk-ant-测试".to_string()));
                assert_eq!(url_arg, Some("https://αpi.测试.com".to_string()));
            }
        }
    }

    #[test]
    fn test_cli_very_long_arguments() {
        let long_alias = "a".repeat(1000);
        let long_token = format!("sk-ant-{}", "b".repeat(1000));
        let long_url = format!("https://{}com", "c".repeat(1000));

        let args = vec!["cc-switch", "add", &long_alias, &long_token, &long_url];

        let result = Cli::try_parse_from(args);
        assert!(result.is_ok(), "Should handle very long arguments");

        if let Ok(cli) = result {
            if let Some(Commands::Add {
                alias_name,
                token_arg,
                url_arg,
                ..
            }) = cli.command
            {
                assert_eq!(alias_name.len(), 1000);
                assert_eq!(token_arg.as_ref().unwrap().len(), 1007); // "sk-ant-" + 1000
                assert_eq!(url_arg.as_ref().unwrap().len(), 1011); // "https://" + 1000 + "com"
            }
        }
    }

    // Performance Tests (Basic)
    #[test]
    fn test_cli_parsing_performance() {
        let start = std::time::Instant::now();

        // Parse many commands to test performance
        for i in 0..1000 {
            let config_name = format!("config-{}", i);
            let token = format!("sk-ant-token-{}", i);
            let url = format!("https://api-{}.test.com", i);

            let args = vec!["cc-switch", "add", &config_name, &token, &url];

            let _ = Cli::try_parse_from(args);
        }

        let duration = start.elapsed();

        // Should complete within reasonable time (5 seconds is generous)
        assert!(
            duration.as_secs() < 5,
            "CLI parsing should be fast, took {:?}",
            duration
        );
    }

    // Configuration Storage Integration Tests
    #[test]
    fn test_storage_operations_integration() {
        let mut storage = ConfigStorage::default();

        // Add multiple configurations
        storage.add_configuration(create_test_config(
            "config1",
            "sk-ant-1",
            "https://api1.com",
        ));
        storage.add_configuration(create_test_config(
            "config2",
            "sk-ant-2",
            "https://api2.com",
        ));
        storage.add_configuration(create_test_config(
            "config3",
            "sk-ant-3",
            "https://api3.com",
        ));

        assert_eq!(storage.configurations.len(), 3);

        // Test retrieval
        let config1 = storage.get_configuration("config1").unwrap();
        assert_eq!(config1.token, "sk-ant-1");
        assert_eq!(config1.url, "https://api1.com");

        // Test removal
        assert!(storage.remove_configuration("config2"));
        assert_eq!(storage.configurations.len(), 2);
        assert!(storage.get_configuration("config2").is_none());

        // Test non-existent removal
        assert!(!storage.remove_configuration("nonexistent"));
        assert_eq!(storage.configurations.len(), 2);
    }
}
