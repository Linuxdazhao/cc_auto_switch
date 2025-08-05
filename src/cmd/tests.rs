#[cfg(test)]
mod tests {
    use crate::cmd::main::*;
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
    }

    #[test]
    fn test_config_storage_default() {
        let storage = ConfigStorage::default();

        assert!(storage.configurations.is_empty());
        assert!(storage.claude_settings_dir.is_none());
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
    fn test_config_storage_set_claude_settings_dir() {
        let mut storage = ConfigStorage::default();

        assert!(storage.claude_settings_dir.is_none());

        storage.set_claude_settings_dir("/test/path".to_string());

        assert_eq!(storage.claude_settings_dir, Some("/test/path".to_string()));
    }

    #[test]
    fn test_config_storage_get_claude_settings_dir() {
        let mut storage = ConfigStorage::default();

        // Test when no custom directory is set
        assert!(storage.get_claude_settings_dir().is_none());

        // Test when custom directory is set
        storage.set_claude_settings_dir("/test/path".to_string());
        assert_eq!(
            storage.get_claude_settings_dir(),
            Some(&"/test/path".to_string())
        );
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
        storage.set_claude_settings_dir("/custom/path".to_string());

        // Mock the save operation to use temp directory
        let json = serde_json::to_string_pretty(&storage).unwrap();
        fs::write(&test_config_path, json).unwrap();

        // Load the storage
        let loaded_content = fs::read_to_string(&test_config_path).unwrap();
        let loaded_storage: ConfigStorage = serde_json::from_str(&loaded_content).unwrap();

        assert_eq!(loaded_storage.configurations.len(), 2);
        assert!(loaded_storage.configurations.contains_key("config1"));
        assert!(loaded_storage.configurations.contains_key("config2"));
        assert_eq!(
            loaded_storage.claude_settings_dir,
            Some("/custom/path".to_string())
        );

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
        assert!(result.claude_settings_dir.is_none());
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

    fn create_test_claude_settings() -> ClaudeSettings {
        let mut settings = ClaudeSettings::default();
        settings.env.insert(
            "ANTHROPIC_AUTH_TOKEN".to_string(),
            "sk-ant-test".to_string(),
        );
        settings.env.insert(
            "ANTHROPIC_BASE_URL".to_string(),
            "https://api.test.com".to_string(),
        );
        settings.other.insert(
            "other_key".to_string(),
            serde_json::Value::String("other_value".to_string()),
        );
        settings
    }

    #[test]
    fn test_claude_settings_default() {
        let settings = ClaudeSettings::default();

        assert!(settings.env.is_empty());
        assert!(settings.other.is_empty());
    }

    #[test]
    fn test_claude_settings_switch_to_config() {
        let mut settings = ClaudeSettings::default();
        let config = create_test_config("test", "sk-ant-test", "https://api.test.com");

        settings.switch_to_config(&config);

        assert_eq!(
            settings.env.get("ANTHROPIC_AUTH_TOKEN"),
            Some(&"sk-ant-test".to_string())
        );
        assert_eq!(
            settings.env.get("ANTHROPIC_BASE_URL"),
            Some(&"https://api.test.com".to_string())
        );
    }

    #[test]
    fn test_claude_settings_remove_anthropic_env() {
        let mut settings = create_test_claude_settings();

        settings.remove_anthropic_env();

        assert!(settings.env.get("ANTHROPIC_AUTH_TOKEN").is_none());
        assert!(settings.env.get("ANTHROPIC_BASE_URL").is_none());
        // Ensure other settings are preserved
        assert!(settings.other.contains_key("other_key"));
    }

    #[test]
    fn test_claude_settings_serialization() {
        let settings = create_test_claude_settings();

        let json = serde_json::to_string_pretty(&settings).unwrap();
        let deserialized: ClaudeSettings = serde_json::from_str(&json).unwrap();

        assert_eq!(
            deserialized.env.get("ANTHROPIC_AUTH_TOKEN"),
            settings.env.get("ANTHROPIC_AUTH_TOKEN")
        );
        assert_eq!(
            deserialized.env.get("ANTHROPIC_BASE_URL"),
            settings.env.get("ANTHROPIC_BASE_URL")
        );
        assert_eq!(
            deserialized.other.get("other_key"),
            settings.other.get("other_key")
        );
    }

    #[test]
    fn test_claude_settings_serialization_empty_env() {
        let mut settings = ClaudeSettings::default();
        settings.other.insert(
            "other_key".to_string(),
            serde_json::Value::String("other_value".to_string()),
        );

        let json = serde_json::to_string_pretty(&settings).unwrap();

        // When env is empty, it should not appear in the JSON
        assert!(!json.contains("\"env\""));

        let deserialized: ClaudeSettings = serde_json::from_str(&json).unwrap();
        assert!(deserialized.env.is_empty());
        assert_eq!(
            deserialized.other.get("other_key"),
            settings.other.get("other_key")
        );
    }

    #[test]
    fn test_claude_settings_deserialization_missing_env() {
        let json = r#"{
            "other_key": "other_value",
            "another_key": 42
        }"#;

        let deserialized: ClaudeSettings = serde_json::from_str(json).unwrap();

        assert!(deserialized.env.is_empty());
        assert_eq!(
            deserialized.other.get("other_key"),
            Some(&serde_json::Value::String("other_value".to_string()))
        );
        assert_eq!(
            deserialized.other.get("another_key"),
            Some(&serde_json::Value::Number(42.into()))
        );
    }

    #[test]
    fn test_claude_settings_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let settings_path = temp_dir.path().join("settings.json");

        let settings = create_test_claude_settings();

        // Save settings
        let json = serde_json::to_string_pretty(&settings).unwrap();
        fs::write(&settings_path, json).unwrap();

        // Load settings
        let loaded_content = fs::read_to_string(&settings_path).unwrap();
        let loaded_settings: ClaudeSettings = serde_json::from_str(&loaded_content).unwrap();

        assert_eq!(
            loaded_settings.env.get("ANTHROPIC_AUTH_TOKEN"),
            settings.env.get("ANTHROPIC_AUTH_TOKEN")
        );
        assert_eq!(
            loaded_settings.env.get("ANTHROPIC_BASE_URL"),
            settings.env.get("ANTHROPIC_BASE_URL")
        );
        assert_eq!(
            loaded_settings.other.get("other_key"),
            settings.other.get("other_key")
        );
    }

    #[test]
    fn test_claude_settings_switch_to_config_preserves_other() {
        let mut settings = create_test_claude_settings();
        let config = create_test_config("test", "sk-ant-new", "https://api.new.com");

        settings.switch_to_config(&config);

        assert_eq!(
            settings.env.get("ANTHROPIC_AUTH_TOKEN"),
            Some(&"sk-ant-new".to_string())
        );
        assert_eq!(
            settings.env.get("ANTHROPIC_BASE_URL"),
            Some(&"https://api.new.com".to_string())
        );
        assert!(settings.other.contains_key("other_key"));
    }

    #[test]
    fn test_claude_settings_multiple_switches() {
        let mut settings = ClaudeSettings::default();
        let config1 = create_test_config("test1", "sk-ant-test1", "https://api1.test.com");
        let config2 = create_test_config("test2", "sk-ant-test2", "https://api2.test.com");

        settings.switch_to_config(&config1);
        assert_eq!(
            settings.env.get("ANTHROPIC_AUTH_TOKEN"),
            Some(&"sk-ant-test1".to_string())
        );

        settings.switch_to_config(&config2);
        assert_eq!(
            settings.env.get("ANTHROPIC_AUTH_TOKEN"),
            Some(&"sk-ant-test2".to_string())
        );
        assert_eq!(
            settings.env.get("ANTHROPIC_BASE_URL"),
            Some(&"https://api2.test.com".to_string())
        );
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
    fn test_get_claude_settings_path_absolute() {
        let custom_dir = if cfg!(windows) {
            // On Windows, use an absolute path that doesn't start with /
            "C:/absolute/path/to/claude"
        } else {
            "/absolute/path/to/claude"
        };
        let result = get_claude_settings_path(Some(custom_dir));

        assert!(result.is_ok());
        let expected_path = std::path::PathBuf::from(custom_dir).join("settings.json");
        assert_eq!(result.unwrap(), expected_path);
    }

    #[test]
    fn test_get_claude_settings_path_relative() {
        let custom_dir = "relative/path";
        let result = get_claude_settings_path(Some(custom_dir));

        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.ends_with("relative/path/settings.json"));
    }

    #[test]
    fn test_get_claude_settings_path_default() {
        let result = get_claude_settings_path(None);

        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.ends_with(".claude/settings.json"));
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
    fn test_cli_parsing_switch_command() {
        use clap::Parser;

        let args = vec!["cc-switch", "switch", "my-config"];
        let cli = Cli::try_parse_from(args).unwrap();

        if let Some(Commands::Switch { alias_name }) = cli.command {
            assert_eq!(alias_name, "my-config");
        } else {
            panic!("Expected Switch command");
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
}
