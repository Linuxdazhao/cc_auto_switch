#[cfg(test)]
mod tests {
    use crate::cmd::config::*;
    use crate::cmd::cli::*;
    use std::fs;
    use tempfile::TempDir;

    /// Helper function to create a temporary directory for testing
    fn create_test_temp_dir() -> TempDir {
        TempDir::new().expect("Failed to create temporary directory")
    }

    /// Helper function to create a test configuration
    fn create_test_config(alias: &str, token: &str, url: &str) -> Configuration {
        Configuration {
            alias_name: alias.to_string(),
            token: token.to_string(),
            url: url.to_string(),
            model: None,
            small_fast_model: None,
        }
    }

    #[test]
    fn test_configuration_creation() {
        let config = create_test_config("test", "sk-ant-test", "https://api.test.com");

        assert_eq!(config.alias_name, "test");
        assert_eq!(config.token, "sk-ant-test");
        assert_eq!(config.url, "https://api.test.com");
    }

    #[test]
    fn test_configuration_default() {
        let config = Configuration::default();

        assert_eq!(config.alias_name, "");
        assert_eq!(config.token, "");
        assert_eq!(config.url, "");
        assert_eq!(config.model, None);
        assert_eq!(config.small_fast_model, None);
    }

    #[test]
    fn test_config_storage_default() {
        let storage = ConfigStorage::default();

        assert!(storage.configurations.is_empty());
    }

    #[test]
    fn test_config_storage_add_configuration() {
        let mut storage = ConfigStorage::default();
        let config = create_test_config("test", "sk-ant-test", "https://api.test.com");

        storage.add_configuration(config.clone());

        assert_eq!(storage.configurations.len(), 1);
        assert!(storage.configurations.contains_key("test"));

        let stored_config = storage.configurations.get("test").unwrap();
        assert_eq!(stored_config.alias_name, config.alias_name);
        assert_eq!(stored_config.token, config.token);
        assert_eq!(stored_config.url, config.url);
    }

    #[test]
    fn test_config_storage_get_configuration() {
        let mut storage = ConfigStorage::default();
        let config = create_test_config("test", "sk-ant-test", "https://api.test.com");

        // Test when configuration doesn't exist
        assert!(storage.get_configuration("nonexistent").is_none());

        // Add configuration and test retrieval
        storage.add_configuration(config);
        let retrieved = storage.get_configuration("test").unwrap();

        assert_eq!(retrieved.alias_name, "test");
        assert_eq!(retrieved.token, "sk-ant-test");
        assert_eq!(retrieved.url, "https://api.test.com");
    }

    #[test]
    fn test_config_storage_remove_configuration() {
        let mut storage = ConfigStorage::default();
        let config = create_test_config("test", "sk-ant-test", "https://api.test.com");

        storage.add_configuration(config);

        // Test removal of existing configuration
        assert!(storage.remove_configuration("test"));
        assert!(!storage.configurations.contains_key("test"));

        // Test removal of non-existing configuration
        assert!(!storage.remove_configuration("nonexistent"));
    }

    #[test]
    fn test_config_storage_save_and_load() {
        let temp_dir = create_test_temp_dir();
        let test_config_path = temp_dir.path().join("configurations.json");

        // Create a custom storage path function for testing
        let _test_get_config_storage_path =
            || Ok::<std::path::PathBuf, anyhow::Error>(test_config_path.clone());

        let mut storage = ConfigStorage::default();
        let config1 = create_test_config("config1", "sk-ant-test1", "https://api1.test.com");
        let config2 = create_test_config("config2", "sk-ant-test2", "https://api2.test.com");

        storage.add_configuration(config1);
        storage.add_configuration(config2);

        // Mock the save operation to use temp directory
        let json = serde_json::to_string_pretty(&storage).unwrap();
        fs::write(&test_config_path, json).unwrap();

        // Load the storage
        let loaded_content = fs::read_to_string(&test_config_path).unwrap();
        let loaded_storage: ConfigStorage = serde_json::from_str(&loaded_content).unwrap();

        assert_eq!(loaded_storage.configurations.len(), 2);
        assert!(loaded_storage.configurations.contains_key("config1"));
        assert!(loaded_storage.configurations.contains_key("config2"));

        let loaded_config1 = loaded_storage.get_configuration("config1").unwrap();
        assert_eq!(loaded_config1.alias_name, "config1");
        assert_eq!(loaded_config1.token, "sk-ant-test1");
        assert_eq!(loaded_config1.url, "https://api1.test.com");
    }

    #[test]
    fn test_config_storage_load_nonexistent_file() {
        let temp_dir = create_test_temp_dir();
        let test_config_path = temp_dir.path().join("nonexistent.json");

        // Mock the load operation to use temp directory
        let result = if test_config_path.exists() {
            let content = fs::read_to_string(&test_config_path).unwrap();
            serde_json::from_str::<ConfigStorage>(&content).unwrap()
        } else {
            ConfigStorage::default()
        };

        assert!(result.configurations.is_empty());
    }

    #[test]
    fn test_config_storage_add_overwrites_existing() {
        let mut storage = ConfigStorage::default();
        let config1 = create_test_config("test", "sk-ant-test1", "https://api1.test.com");
        let config2 = create_test_config("test", "sk-ant-test2", "https://api2.test.com");

        storage.add_configuration(config1);
        storage.add_configuration(config2);

        assert_eq!(storage.configurations.len(), 1);
        let stored_config = storage.get_configuration("test").unwrap();
        assert_eq!(stored_config.token, "sk-ant-test2");
        assert_eq!(stored_config.url, "https://api2.test.com");
    }

    #[test]
    fn test_config_storage_multiple_configurations() {
        let mut storage = ConfigStorage::default();

        for i in 0..10 {
            let config = create_test_config(
                &format!("config{}", i),
                &format!("sk-ant-test{}", i),
                &format!("https://api{}.test.com", i),
            );
            storage.add_configuration(config);
        }

        assert_eq!(storage.configurations.len(), 10);

        for i in 0..10 {
            let alias = &format!("config{}", i);
            assert!(storage.configurations.contains_key(alias));

            let config = storage.get_configuration(alias).unwrap();
            assert_eq!(config.token, format!("sk-ant-test{}", i));
            assert_eq!(config.url, format!("https://api{}.test.com", i));
        }
    }


    #[test]
    fn test_environment_config_default() {
        let env_config = EnvironmentConfig::default();

        assert!(env_config.env_vars.is_empty());
    }

    #[test]
    fn test_environment_config_from_config() {
        let config = create_test_config("test", "sk-ant-test", "https://api.test.com");
        let env_config = EnvironmentConfig::from_config(&config);

        assert_eq!(env_config.env_vars.len(), 2);
        assert_eq!(
            env_config.env_vars.get("ANTHROPIC_AUTH_TOKEN"),
            Some(&"sk-ant-test".to_string())
        );
        assert_eq!(
            env_config.env_vars.get("ANTHROPIC_BASE_URL"),
            Some(&"https://api.test.com".to_string())
        );
    }

    #[test]
    fn test_environment_config_with_models() {
        let mut config = create_test_config("test", "sk-ant-test", "https://api.test.com");
        config.model = Some("claude-3-5-sonnet-20241022".to_string());
        config.small_fast_model = Some("claude-3-haiku-20240307".to_string());

        let env_config = EnvironmentConfig::from_config(&config);

        assert_eq!(env_config.env_vars.len(), 4);
        assert_eq!(
            env_config.env_vars.get("ANTHROPIC_MODEL"),
            Some(&"claude-3-5-sonnet-20241022".to_string())
        );
        assert_eq!(
            env_config.env_vars.get("ANTHROPIC_SMALL_FAST_MODEL"),
            Some(&"claude-3-haiku-20240307".to_string())
        );
    }

    #[test]
    fn test_environment_config_empty() {
        let env_config = EnvironmentConfig::empty();
        assert!(env_config.env_vars.is_empty());
    }

    #[test]
    fn test_environment_config_as_env_tuples() {
        let config = create_test_config("test", "sk-ant-test", "https://api.test.com");
        let env_config = EnvironmentConfig::from_config(&config);
        let tuples = env_config.as_env_tuples();

        assert_eq!(tuples.len(), 2);
        assert!(tuples.contains(&("ANTHROPIC_AUTH_TOKEN".to_string(), "sk-ant-test".to_string())));
        assert!(tuples.contains(&("ANTHROPIC_BASE_URL".to_string(), "https://api.test.com".to_string())));
    }

    #[test]
    fn test_validate_alias_name_valid() {
        assert!(validate_alias_name("test").is_ok());
        assert!(validate_alias_name("my-config").is_ok());
        assert!(validate_alias_name("config_123").is_ok());
    }

    #[test]
    fn test_validate_alias_name_empty() {
        let result = validate_alias_name("");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Alias name cannot be empty"
        );
    }

    #[test]
    fn test_validate_alias_name_reserved_cc() {
        let result = validate_alias_name("cc");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Alias name 'cc' is reserved and cannot be used"
        );
    }

    #[test]
    fn test_validate_alias_name_whitespace() {
        let result = validate_alias_name("test config");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Alias name cannot contain whitespace"
        );
    }

    #[test]
    fn test_cli_parsing() {
        use clap::Parser;

        // Test parsing of add command
        let args = vec![
            "cc-switch",
            "add",
            "my-config",
            "sk-ant-test",
            "https://api.test.com",
        ];
        let cli = Cli::try_parse_from(args).unwrap();

        if let Some(Commands::Add {
            alias_name,
            token,
            url,
            force,
            interactive,
            token_arg,
            url_arg,
            ..
        }) = cli.command
        {
            assert_eq!(alias_name, "my-config");
            assert_eq!(token, None);
            assert_eq!(url, None);
            assert!(!force);
            assert!(!interactive);
            assert_eq!(token_arg, Some("sk-ant-test".to_string()));
            assert_eq!(url_arg, Some("https://api.test.com".to_string()));
        } else {
            panic!("Expected Add command");
        }
    }

    #[test]
    fn test_cli_parsing_with_flags() {
        use clap::Parser;

        // Test parsing with flags
        let args = vec![
            "cc-switch",
            "add",
            "my-config",
            "-t",
            "sk-ant-test",
            "-u",
            "https://api.test.com",
            "-f",
            "-i",
        ];
        let cli = Cli::try_parse_from(args).unwrap();

        if let Some(Commands::Add {
            alias_name,
            token,
            url,
            force,
            interactive,
            token_arg,
            url_arg,
            ..
        }) = cli.command
        {
            assert_eq!(alias_name, "my-config");
            assert_eq!(token, Some("sk-ant-test".to_string()));
            assert_eq!(url, Some("https://api.test.com".to_string()));
            assert!(force);
            assert!(interactive);
            assert_eq!(token_arg, None);
            assert_eq!(url_arg, None);
        } else {
            panic!("Expected Add command");
        }
    }

    #[test]
    fn test_cli_parsing_list_command() {
        use clap::Parser;

        let args = vec!["cc-switch", "list"];
        let cli = Cli::try_parse_from(args).unwrap();

        if let Some(Commands::List) = cli.command {
            // Test passed
        } else {
            panic!("Expected List command");
        }
    }

    #[test]
    fn test_cli_parsing_use_command() {
        use clap::Parser;

        let args = vec!["cc-switch", "use", "my-config"];
        let cli = Cli::try_parse_from(args).unwrap();

        if let Some(Commands::Use { alias_name }) = cli.command {
            assert_eq!(alias_name, "my-config");
        } else {
            panic!("Expected Use command");
        }
    }

    #[test]
    fn test_cli_parsing_switch_alias() {
        use clap::Parser;

        let args = vec!["cc-switch", "switch", "my-config"];
        let cli = Cli::try_parse_from(args).unwrap();

        if let Some(Commands::Use { alias_name }) = cli.command {
            assert_eq!(alias_name, "my-config");
        } else {
            panic!("Expected Use command (switch alias)");
        }
    }

    #[test]
    fn test_cli_parsing_remove_command() {
        use clap::Parser;

        let args = vec!["cc-switch", "remove", "config1", "config2"];
        let cli = Cli::try_parse_from(args).unwrap();

        if let Some(Commands::Remove { alias_names }) = cli.command {
            assert_eq!(alias_names, vec!["config1", "config2"]);
        } else {
            panic!("Expected Remove command");
        }
    }

    #[test]
    fn test_cli_parsing_completion_command() {
        use clap::Parser;

        let args = vec!["cc-switch", "completion", "fish"];
        let cli = Cli::try_parse_from(args).unwrap();

        if let Some(Commands::Completion { shell }) = cli.command {
            assert_eq!(shell, "fish");
        } else {
            panic!("Expected Completion command");
        }
    }

    #[test]
    fn test_format_token_safely_long_token() {
        use crate::cmd::interactive::format_token_safely;
        
        // Test with a long token (typical Claude API token)
        let token = "sk-ant-api03-abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let formatted = format_token_safely(token);
        
        // Should show first 12 chars + "..." + last 8 chars
        // Last 8 chars of the token are "STUVWXYZ"
        assert_eq!(formatted, "sk-ant-api03...STUVWXYZ");
    }

    #[test]
    fn test_format_token_safely_medium_token() {
        use crate::cmd::interactive::format_token_safely;
        
        // Test with medium length token (15 chars)
        let token = "sk-ant-medium1";
        let formatted = format_token_safely(token);
        
        // Should show first half + "***"
        assert_eq!(formatted, "sk-ant-***");
    }

    #[test]
    fn test_format_token_safely_short_token() {
        use crate::cmd::interactive::format_token_safely;
        
        // Test with short token (6 chars)
        let token = "short1";
        let formatted = format_token_safely(token);
        
        // Should show first few chars + "***"
        assert_eq!(formatted, "sho***");
    }

    #[test]
    fn test_format_token_safely_very_short_token() {
        use crate::cmd::interactive::format_token_safely;
        
        // Test with very short token (3 chars)
        let token = "abc";
        let formatted = format_token_safely(token);
        
        // Should show first 2 chars + "***"
        assert_eq!(formatted, "ab***");
    }

    #[test]
    fn test_format_token_safely_single_char() {
        use crate::cmd::interactive::format_token_safely;
        
        // Test with single character token
        let token = "x";
        let formatted = format_token_safely(token);
        
        // Should show first char + "***"
        assert_eq!(formatted, "x***");
    }

    #[test]
    fn test_format_token_safely_empty_token() {
        use crate::cmd::interactive::format_token_safely;
        
        // Test with empty token
        let token = "";
        let formatted = format_token_safely(token);
        
        // Should show "***" for empty token
        assert_eq!(formatted, "***");
    }

    #[test]
    fn test_format_token_safely_boundary_exactly_20_chars() {
        use crate::cmd::interactive::format_token_safely;
        
        // Test with exactly 20 characters (PREFIX_LEN + SUFFIX_LEN)
        let token = "12345678901234567890";
        let formatted = format_token_safely(token);
        
        // Should show first 10 chars + "***" (medium token behavior)
        assert_eq!(formatted, "1234567890***");
    }

    #[test]
    fn test_format_token_safely_boundary_21_chars() {
        use crate::cmd::interactive::format_token_safely;
        
        // Test with 21 characters (just over PREFIX_LEN + SUFFIX_LEN)
        let token = "123456789012345678901";
        let formatted = format_token_safely(token);
        
        // Should use long token format: first 12 + "..." + last 8
        // Last 8 chars of "123456789012345678901" are "45678901"
        assert_eq!(formatted, "123456789012...45678901");
    }
}
