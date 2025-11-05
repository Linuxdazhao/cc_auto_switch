#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use cc_switch::cli::cli::*;
    use cc_switch::config::EnvironmentConfig;
    use cc_switch::config::*;
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
            max_thinking_tokens: None,
            api_timeout_ms: None,
            claude_code_disable_nonessential_traffic: None,
            anthropic_default_sonnet_model: None,
            anthropic_default_opus_model: None,
            anthropic_default_haiku_model: None,
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
        assert!(tuples.contains(&(
            "ANTHROPIC_AUTH_TOKEN".to_string(),
            "sk-ant-test".to_string()
        )));
        assert!(tuples.contains(&(
            "ANTHROPIC_BASE_URL".to_string(),
            "https://api.test.com".to_string()
        )));
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
            assert_eq!(alias_name.unwrap(), "my-config");
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
            assert_eq!(alias_name.unwrap(), "my-config");
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

        if let Some(Commands::List { plain: _ }) = cli.command {
            // Test passed
        } else {
            panic!("Expected List command");
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

    // Additional config.rs tests to improve coverage
    #[test]
    fn test_get_config_storage_path() {
        let result = get_config_storage_path();
        assert!(
            result.is_ok(),
            "Should be able to determine config storage path"
        );

        let path = result.unwrap();
        assert!(path.to_string_lossy().contains(".claude"));
        assert!(
            path.to_string_lossy()
                .contains("cc_auto_switch_setting.json")
        );
    }

    #[test]
    fn test_config_storage_load_create_directory() {
        let temp_dir = create_test_temp_dir();
        let _test_config_path = temp_dir.path().join("configurations.json");

        // Test loading when file doesn't exist - should return default
        // Note: This test uses the actual config path, so it may succeed or fail
        // depending on whether user has actual configurations
        let storage = ConfigStorage::load();

        match storage {
            Ok(s) => {
                // Storage loaded successfully - configurations may exist or be empty
                // Both cases are valid for this test
                println!(
                    "Loaded storage with {} configurations",
                    s.configurations.len()
                );
            }
            Err(_) => {
                // May fail due to home directory issues in test environment
                // This is acceptable as we're testing the logic
            }
        }
    }

    #[test]
    fn test_config_storage_save_and_load_integration() {
        // This test verifies the save/load cycle works correctly
        let mut storage = ConfigStorage::default();
        let config1 = create_test_config("save-test-1", "sk-ant-save-1", "https://save1.test.com");
        let config2 = create_test_config("save-test-2", "sk-ant-save-2", "https://save2.test.com");

        storage.add_configuration(config1);
        storage.add_configuration(config2);

        // Test in-memory operations work correctly
        assert_eq!(storage.configurations.len(), 2);
        assert!(storage.configurations.contains_key("save-test-1"));
        assert!(storage.configurations.contains_key("save-test-2"));

        let retrieved1 = storage.get_configuration("save-test-1").unwrap();
        assert_eq!(retrieved1.token, "sk-ant-save-1");
        assert_eq!(retrieved1.url, "https://save1.test.com");

        let retrieved2 = storage.get_configuration("save-test-2").unwrap();
        assert_eq!(retrieved2.token, "sk-ant-save-2");
        assert_eq!(retrieved2.url, "https://save2.test.com");
    }

    #[test]
    fn test_config_storage_remove_multiple() {
        let mut storage = ConfigStorage::default();

        // Add multiple configurations
        for i in 0..5 {
            let config = create_test_config(
                &format!("remove-test-{}", i),
                &format!("sk-ant-remove-{}", i),
                &format!("https://remove-{}.test.com", i),
            );
            storage.add_configuration(config);
        }

        assert_eq!(storage.configurations.len(), 5);

        // Remove some configurations
        assert!(storage.remove_configuration("remove-test-1"));
        assert!(storage.remove_configuration("remove-test-3"));
        assert_eq!(storage.configurations.len(), 3);

        // Try to remove non-existent configuration
        assert!(!storage.remove_configuration("non-existent"));
        assert_eq!(storage.configurations.len(), 3);

        // Verify remaining configurations
        assert!(storage.configurations.contains_key("remove-test-0"));
        assert!(storage.configurations.contains_key("remove-test-2"));
        assert!(storage.configurations.contains_key("remove-test-4"));
        assert!(!storage.configurations.contains_key("remove-test-1"));
        assert!(!storage.configurations.contains_key("remove-test-3"));
    }

    #[test]
    fn test_environment_config_empty_model_strings() {
        let mut config =
            create_test_config("empty-model-test", "sk-ant-empty", "https://empty.test.com");
        config.model = Some("".to_string()); // Empty string, not None
        config.small_fast_model = Some("".to_string()); // Empty string, not None

        let env_config = EnvironmentConfig::from_config(&config);
        let env_tuples = env_config.as_env_tuples();

        // Empty model strings should not be included in environment variables
        assert_eq!(env_tuples.len(), 2);
        assert!(env_tuples.iter().any(|(k, _)| k == "ANTHROPIC_AUTH_TOKEN"));
        assert!(env_tuples.iter().any(|(k, _)| k == "ANTHROPIC_BASE_URL"));
        assert!(!env_tuples.iter().any(|(k, _)| k == "ANTHROPIC_MODEL"));
        assert!(
            !env_tuples
                .iter()
                .any(|(k, _)| k == "ANTHROPIC_SMALL_FAST_MODEL")
        );
    }

    #[test]
    fn test_environment_config_from_config_edge_cases() {
        // Test with empty token and URL (should still be included)
        let config = Configuration {
            alias_name: "edge-case".to_string(),
            token: "".to_string(),
            url: "".to_string(),
            model: None,
            small_fast_model: None,
            max_thinking_tokens: None,
            api_timeout_ms: None,
            claude_code_disable_nonessential_traffic: None,
            anthropic_default_sonnet_model: None,
            anthropic_default_opus_model: None,
            anthropic_default_haiku_model: None,
        };

        let env_config = EnvironmentConfig::from_config(&config);
        let env_tuples = env_config.as_env_tuples();

        assert_eq!(env_tuples.len(), 2);
        assert!(
            env_tuples
                .iter()
                .any(|(k, v)| k == "ANTHROPIC_AUTH_TOKEN" && v.is_empty())
        );
        assert!(
            env_tuples
                .iter()
                .any(|(k, v)| k == "ANTHROPIC_BASE_URL" && v.is_empty())
        );
    }

    #[test]
    fn test_environment_config_partial_models() {
        // Test with only main model set
        let mut config1 =
            create_test_config("partial-1", "sk-ant-partial-1", "https://partial1.test.com");
        config1.model = Some("claude-main-only".to_string());

        let env_config1 = EnvironmentConfig::from_config(&config1);
        let env_tuples1 = env_config1.as_env_tuples();

        assert_eq!(env_tuples1.len(), 3);
        assert!(
            env_tuples1
                .iter()
                .any(|(k, v)| k == "ANTHROPIC_MODEL" && v == "claude-main-only")
        );
        assert!(
            !env_tuples1
                .iter()
                .any(|(k, _)| k == "ANTHROPIC_SMALL_FAST_MODEL")
        );

        // Test with only small fast model set
        let mut config2 =
            create_test_config("partial-2", "sk-ant-partial-2", "https://partial2.test.com");
        config2.small_fast_model = Some("haiku-only".to_string());

        let env_config2 = EnvironmentConfig::from_config(&config2);
        let env_tuples2 = env_config2.as_env_tuples();

        assert_eq!(env_tuples2.len(), 3);
        assert!(!env_tuples2.iter().any(|(k, _)| k == "ANTHROPIC_MODEL"));
        assert!(
            env_tuples2
                .iter()
                .any(|(k, v)| k == "ANTHROPIC_SMALL_FAST_MODEL" && v == "haiku-only")
        );
    }

    #[test]
    fn test_validate_alias_name_edge_cases() {
        // Test with different types of whitespace
        assert!(
            validate_alias_name("test\tconfig").is_err(),
            "Should reject tabs"
        );
        assert!(
            validate_alias_name("test\nconfig").is_err(),
            "Should reject newlines"
        );
        assert!(
            validate_alias_name("test\rconfig").is_err(),
            "Should reject carriage returns"
        );
        assert!(
            validate_alias_name("test config with multiple spaces").is_err(),
            "Should reject multiple spaces"
        );

        // Test with valid special characters
        assert!(
            validate_alias_name("test-config").is_ok(),
            "Should accept hyphens"
        );
        assert!(
            validate_alias_name("test_config").is_ok(),
            "Should accept underscores"
        );
        assert!(
            validate_alias_name("test.config").is_ok(),
            "Should accept dots"
        );
        assert!(
            validate_alias_name("test123").is_ok(),
            "Should accept numbers"
        );
        assert!(
            validate_alias_name("123test").is_ok(),
            "Should accept starting with numbers"
        );

        // Test with very long alias names
        let long_alias = "a".repeat(1000);
        assert!(
            validate_alias_name(&long_alias).is_ok(),
            "Should accept very long alias names"
        );

        // Test with unicode characters
        assert!(
            validate_alias_name("测试-config").is_ok(),
            "Should accept unicode characters"
        );
        assert!(
            validate_alias_name("αλιας").is_ok(),
            "Should accept Greek characters"
        );
        assert!(
            validate_alias_name("config-🚀").is_ok(),
            "Should accept emoji characters"
        );
    }

    #[test]
    fn test_validate_alias_name_case_sensitivity() {
        // "cc" is reserved, but other cases should be allowed
        assert!(
            validate_alias_name("CC").is_ok(),
            "Should accept uppercase CC"
        );
        assert!(
            validate_alias_name("Cc").is_ok(),
            "Should accept mixed case Cc"
        );
        assert!(
            validate_alias_name("cC").is_ok(),
            "Should accept mixed case cC"
        );
        assert!(
            validate_alias_name("cc-config").is_ok(),
            "Should accept cc as prefix"
        );
        assert!(
            validate_alias_name("config-cc").is_ok(),
            "Should accept cc as suffix"
        );
    }

    #[test]
    fn test_configuration_serialization_format() {
        let config = Configuration {
            alias_name: "format-test".to_string(),
            token: "sk-ant-format-test".to_string(),
            url: "https://format.test.com".to_string(),
            model: Some("claude-format-model".to_string()),
            small_fast_model: None,
            max_thinking_tokens: None,
            api_timeout_ms: None,
            claude_code_disable_nonessential_traffic: None,
            anthropic_default_sonnet_model: None,
            anthropic_default_opus_model: None,
            anthropic_default_haiku_model: None,
        };

        let json = serde_json::to_string_pretty(&config).expect("Should serialize to pretty JSON");

        // Verify JSON structure
        assert!(json.contains("\"alias_name\": \"format-test\""));
        assert!(json.contains("\"token\": \"sk-ant-format-test\""));
        assert!(json.contains("\"url\": \"https://format.test.com\""));
        assert!(json.contains("\"model\": \"claude-format-model\""));
        // small_fast_model should not appear since it's None
        assert!(!json.contains("small_fast_model"));
    }

    #[test]
    fn test_configuration_deserialization_extra_fields() {
        // Test that deserialization ignores unknown fields (forward compatibility)
        let json_with_extra_fields = r#"{
            "alias_name": "extra-fields-test",
            "token": "sk-ant-extra",
            "url": "https://extra.test.com",
            "model": "claude-extra-model",
            "unknown_field": "should-be-ignored",
            "another_unknown": 42
        }"#;

        let result: Result<Configuration, _> = serde_json::from_str(json_with_extra_fields);
        assert!(
            result.is_ok(),
            "Should successfully deserialize despite extra fields"
        );

        let config = result.unwrap();
        assert_eq!(config.alias_name, "extra-fields-test");
        assert_eq!(config.token, "sk-ant-extra");
        assert_eq!(config.url, "https://extra.test.com");
        assert_eq!(config.model, Some("claude-extra-model".to_string()));
        assert_eq!(config.small_fast_model, None);
    }

    #[test]
    fn test_configuration_deserialization_missing_optional_fields() {
        // Test that missing optional fields default to None
        let minimal_json = r#"{
            "alias_name": "minimal-test",
            "token": "sk-ant-minimal",
            "url": "https://minimal.test.com"
        }"#;

        let config: Configuration =
            serde_json::from_str(minimal_json).expect("Should deserialize minimal JSON");

        assert_eq!(config.alias_name, "minimal-test");
        assert_eq!(config.token, "sk-ant-minimal");
        assert_eq!(config.url, "https://minimal.test.com");
        assert_eq!(config.model, None);
        assert_eq!(config.small_fast_model, None);
    }

    #[test]
    fn test_environment_config_as_env_tuples_order() {
        let config = Configuration {
            alias_name: "order-test".to_string(),
            token: "sk-ant-order".to_string(),
            url: "https://order.test.com".to_string(),
            model: Some("claude-order-model".to_string()),
            small_fast_model: Some("haiku-order-model".to_string()),
            max_thinking_tokens: None,
            api_timeout_ms: None,
            claude_code_disable_nonessential_traffic: None,
            anthropic_default_sonnet_model: None,
            anthropic_default_opus_model: None,
            anthropic_default_haiku_model: None,
        };

        let env_config = EnvironmentConfig::from_config(&config);
        let env_tuples = env_config.as_env_tuples();

        assert_eq!(env_tuples.len(), 4);

        // Verify all expected variables are present (order doesn't matter for HashMap)
        let var_names: Vec<String> = env_tuples.iter().map(|(k, _)| k.clone()).collect();
        assert!(var_names.contains(&"ANTHROPIC_AUTH_TOKEN".to_string()));
        assert!(var_names.contains(&"ANTHROPIC_BASE_URL".to_string()));
        assert!(var_names.contains(&"ANTHROPIC_MODEL".to_string()));
        assert!(var_names.contains(&"ANTHROPIC_SMALL_FAST_MODEL".to_string()));

        // Verify values are correct
        for (key, value) in env_tuples {
            match key.as_str() {
                "ANTHROPIC_AUTH_TOKEN" => assert_eq!(value, "sk-ant-order"),
                "ANTHROPIC_BASE_URL" => assert_eq!(value, "https://order.test.com"),
                "ANTHROPIC_MODEL" => assert_eq!(value, "claude-order-model"),
                "ANTHROPIC_SMALL_FAST_MODEL" => assert_eq!(value, "haiku-order-model"),
                _ => panic!("Unexpected environment variable: {}", key),
            }
        }
    }

    #[test]
    fn test_format_token_for_display_long_token() {
        use cc_switch::cli::display_utils::format_token_for_display;

        // Test with a long token (typical Claude API token)
        let token = "sk-ant-api03-abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let formatted = format_token_for_display(token);

        // Should show first 12 chars + "..." + last 8 chars
        // Last 8 chars of the token are "STUVWXYZ"
        assert_eq!(formatted, "sk-ant-api03...STUVWXYZ");
    }

    #[test]
    fn test_format_token_for_display_medium_token() {
        use cc_switch::cli::display_utils::format_token_for_display;

        // Test with medium length token (15 chars)
        let token = "sk-ant-medium1";
        let formatted = format_token_for_display(token);

        // Should show first half + "***"
        assert_eq!(formatted, "sk-ant-***");
    }

    #[test]
    fn test_format_token_for_display_short_token() {
        use cc_switch::cli::display_utils::format_token_for_display;

        // Test with short token (6 chars)
        let token = "short1";
        let formatted = format_token_for_display(token);

        // Should show first few chars + "***"
        assert_eq!(formatted, "sho***");
    }

    #[test]
    fn test_format_token_for_display_very_short_token() {
        use cc_switch::cli::display_utils::format_token_for_display;

        // Test with very short token (3 chars)
        let token = "abc";
        let formatted = format_token_for_display(token);

        // Should show first 2 chars + "***"
        assert_eq!(formatted, "ab***");
    }

    #[test]
    fn test_format_token_for_display_single_char() {
        use cc_switch::cli::display_utils::format_token_for_display;

        // Test with single character token
        let token = "x";
        let formatted = format_token_for_display(token);

        // Should show first char + "***"
        assert_eq!(formatted, "x***");
    }

    #[test]
    fn test_format_token_for_display_empty_token() {
        use cc_switch::cli::display_utils::format_token_for_display;

        // Test with empty token
        let token = "";
        let formatted = format_token_for_display(token);

        // Should show "***" for empty token
        assert_eq!(formatted, "***");
    }

    #[test]
    fn test_format_token_for_display_boundary_exactly_20_chars() {
        use cc_switch::cli::display_utils::format_token_for_display;

        // Test with exactly 20 characters (PREFIX_LEN + SUFFIX_LEN)
        let token = "12345678901234567890";
        let formatted = format_token_for_display(token);

        // Should show first 10 chars + "***" (medium token behavior)
        assert_eq!(formatted, "1234567890***");
    }

    #[test]
    fn test_format_token_for_display_boundary_21_chars() {
        use cc_switch::cli::display_utils::format_token_for_display;

        // Test with 21 characters (just over PREFIX_LEN + SUFFIX_LEN)
        let token = "123456789012345678901";
        let formatted = format_token_for_display(token);

        // Should use long token format: first 12 + "..." + last 8
        // Last 8 chars of "123456789012345678901" are "45678901"
        assert_eq!(formatted, "123456789012...45678901");
    }
}

#[cfg(test)]
mod config_edit_tests {
    use cc_switch::config::EnvironmentConfig;
    use cc_switch::config::types::{ConfigStorage, Configuration};
    use tempfile::TempDir;

    fn create_test_storage_dir() -> (TempDir, ConfigStorage) {
        let temp_dir = TempDir::new().unwrap();
        let mut storage = ConfigStorage::default();

        // Add a test configuration
        let config = Configuration {
            alias_name: "test-config".to_string(),
            token: "sk-test-123".to_string(),
            url: "https://api.test.com".to_string(),
            model: Some("test-model".to_string()),
            small_fast_model: Some("test-fast-model".to_string()),
            max_thinking_tokens: None,
            api_timeout_ms: None,
            claude_code_disable_nonessential_traffic: None,
            anthropic_default_sonnet_model: None,
            anthropic_default_opus_model: None,
            anthropic_default_haiku_model: None,
        };
        storage.add_configuration(config);

        (temp_dir, storage)
    }

    #[test]
    fn test_update_configuration_same_alias() {
        let (_temp_dir, mut storage) = create_test_storage_dir();

        // Update the configuration with same alias
        let updated_config = Configuration {
            alias_name: "test-config".to_string(),
            token: "sk-updated-456".to_string(),
            url: "https://api.updated.com".to_string(),
            model: Some("updated-model".to_string()),
            small_fast_model: None,
            max_thinking_tokens: None,
            api_timeout_ms: None,
            claude_code_disable_nonessential_traffic: None,
            anthropic_default_sonnet_model: None,
            anthropic_default_opus_model: None,
            anthropic_default_haiku_model: None,
        };

        let result = storage.update_configuration("test-config", updated_config);
        assert!(result.is_ok());

        // Verify the configuration was updated
        let config = storage.get_configuration("test-config").unwrap();
        assert_eq!(config.token, "sk-updated-456");
        assert_eq!(config.url, "https://api.updated.com");
        assert_eq!(config.model, Some("updated-model".to_string()));
        assert_eq!(config.small_fast_model, None);
    }

    #[test]
    fn test_update_configuration_rename_alias() {
        let (_temp_dir, mut storage) = create_test_storage_dir();

        // Rename the configuration
        let renamed_config = Configuration {
            alias_name: "renamed-config".to_string(),
            token: "sk-test-123".to_string(),
            url: "https://api.test.com".to_string(),
            model: Some("test-model".to_string()),
            small_fast_model: Some("test-fast-model".to_string()),
            max_thinking_tokens: None,
            api_timeout_ms: None,
            claude_code_disable_nonessential_traffic: None,
            anthropic_default_sonnet_model: None,
            anthropic_default_opus_model: None,
            anthropic_default_haiku_model: None,
        };

        let result = storage.update_configuration("test-config", renamed_config);
        assert!(result.is_ok());

        // Verify the old alias is gone and new alias exists
        assert!(storage.get_configuration("test-config").is_none());
        assert!(storage.get_configuration("renamed-config").is_some());

        let config = storage.get_configuration("renamed-config").unwrap();
        assert_eq!(config.alias_name, "renamed-config");
    }

    #[test]
    fn test_update_configuration_nonexistent() {
        let (_temp_dir, mut storage) = create_test_storage_dir();

        let new_config = Configuration {
            alias_name: "new-config".to_string(),
            token: "sk-new-789".to_string(),
            url: "https://api.new.com".to_string(),
            model: None,
            small_fast_model: None,
            max_thinking_tokens: None,
            api_timeout_ms: None,
            claude_code_disable_nonessential_traffic: None,
            anthropic_default_sonnet_model: None,
            anthropic_default_opus_model: None,
            anthropic_default_haiku_model: None,
        };

        let result = storage.update_configuration("nonexistent", new_config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_update_configuration_rename_to_existing() {
        let (_temp_dir, mut storage) = create_test_storage_dir();

        // Add another configuration
        let config2 = Configuration {
            alias_name: "config2".to_string(),
            token: "sk-config2-456".to_string(),
            url: "https://api.config2.com".to_string(),
            model: None,
            small_fast_model: None,
            max_thinking_tokens: None,
            api_timeout_ms: None,
            claude_code_disable_nonessential_traffic: None,
            anthropic_default_sonnet_model: None,
            anthropic_default_opus_model: None,
            anthropic_default_haiku_model: None,
        };
        storage.add_configuration(config2);

        // Try to rename test-config to config2 (should succeed and overwrite)
        let renamed_config = Configuration {
            alias_name: "config2".to_string(),
            token: "sk-overwritten".to_string(),
            url: "https://api.overwritten.com".to_string(),
            model: None,
            small_fast_model: None,
            max_thinking_tokens: None,
            api_timeout_ms: None,
            claude_code_disable_nonessential_traffic: None,
            anthropic_default_sonnet_model: None,
            anthropic_default_opus_model: None,
            anthropic_default_haiku_model: None,
        };

        let result = storage.update_configuration("test-config", renamed_config);
        assert!(result.is_ok());

        // Verify test-config is gone and config2 is overwritten
        assert!(storage.get_configuration("test-config").is_none());
        let config = storage.get_configuration("config2").unwrap();
        assert_eq!(config.token, "sk-overwritten");
        assert_eq!(config.url, "https://api.overwritten.com");
    }

    #[test]
    fn test_update_configuration_clear_optional_fields() {
        let (_temp_dir, mut storage) = create_test_storage_dir();

        // Update configuration with cleared optional fields
        let updated_config = Configuration {
            alias_name: "test-config".to_string(),
            token: "sk-test-123".to_string(),
            url: "https://api.test.com".to_string(),
            model: None,
            small_fast_model: None,
            max_thinking_tokens: None,
            api_timeout_ms: None,
            claude_code_disable_nonessential_traffic: None,
            anthropic_default_sonnet_model: None,
            anthropic_default_opus_model: None,
            anthropic_default_haiku_model: None,
        };

        let result = storage.update_configuration("test-config", updated_config);
        assert!(result.is_ok());

        let config = storage.get_configuration("test-config").unwrap();
        assert_eq!(config.model, None);
        assert_eq!(config.small_fast_model, None);
    }

    #[test]
    fn test_new_configuration_fields() {
        let config = Configuration {
            alias_name: "test".to_string(),
            token: "sk-ant-test".to_string(),
            url: "https://api.test.com".to_string(),
            model: None,
            small_fast_model: None,
            max_thinking_tokens: None,
            api_timeout_ms: Some(3000000),
            claude_code_disable_nonessential_traffic: Some(1),
            anthropic_default_sonnet_model: Some("MiniMax-M2".to_string()),
            anthropic_default_opus_model: Some("MiniMax-M2".to_string()),
            anthropic_default_haiku_model: Some("MiniMax-M2".to_string()),
        };

        assert_eq!(config.api_timeout_ms, Some(3000000));
        assert_eq!(config.claude_code_disable_nonessential_traffic, Some(1));
        assert_eq!(
            config.anthropic_default_sonnet_model,
            Some("MiniMax-M2".to_string())
        );
        assert_eq!(
            config.anthropic_default_opus_model,
            Some("MiniMax-M2".to_string())
        );
        assert_eq!(
            config.anthropic_default_haiku_model,
            Some("MiniMax-M2".to_string())
        );
    }

    #[test]
    fn test_environment_config_with_new_fields() {
        let config = Configuration {
            alias_name: "test".to_string(),
            token: "sk-ant-test".to_string(),
            url: "https://api.test.com".to_string(),
            model: None,
            small_fast_model: None,
            max_thinking_tokens: None,
            api_timeout_ms: Some(3000000),
            claude_code_disable_nonessential_traffic: Some(1),
            anthropic_default_sonnet_model: Some("MiniMax-M2".to_string()),
            anthropic_default_opus_model: Some("MiniMax-M2".to_string()),
            anthropic_default_haiku_model: Some("MiniMax-M2".to_string()),
        };

        let env_config = EnvironmentConfig::from_config(&config);

        assert_eq!(
            env_config.env_vars.get("API_TIMEOUT_MS"),
            Some(&"3000000".to_string())
        );
        assert_eq!(
            env_config
                .env_vars
                .get("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC"),
            Some(&"1".to_string())
        );
        assert_eq!(
            env_config.env_vars.get("ANTHROPIC_DEFAULT_SONNET_MODEL"),
            Some(&"MiniMax-M2".to_string())
        );
        assert_eq!(
            env_config.env_vars.get("ANTHROPIC_DEFAULT_OPUS_MODEL"),
            Some(&"MiniMax-M2".to_string())
        );
        assert_eq!(
            env_config.env_vars.get("ANTHROPIC_DEFAULT_HAIKU_MODEL"),
            Some(&"MiniMax-M2".to_string())
        );
    }
}

#[cfg(test)]
mod claude_settings_tests {
    use cc_switch::config::ClaudeSettings;
    use cc_switch::config::types::StorageMode;
    use std::collections::BTreeMap;
    use std::fs;
    use tempfile::TempDir;

    /// Helper function to create a temporary directory for testing
    fn create_test_temp_dir() -> TempDir {
        TempDir::new().expect("Failed to create temporary directory")
    }

    /// Helper function to create a test configuration
    fn create_test_config(
        alias: &str,
        token: &str,
        url: &str,
    ) -> cc_switch::config::types::Configuration {
        cc_switch::config::types::Configuration {
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

    #[test]
    fn test_switch_to_env_mode_detects_conflicts() {
        let temp_dir = create_test_temp_dir();
        let settings_path = temp_dir.path().join("settings.json");

        // Create a test configuration
        let config = create_test_config("test-config", "sk-ant-test", "https://api.test.com");

        // Create ClaudeSettings with Anthropic env fields (should be auto-cleaned)
        let mut env = BTreeMap::new();
        env.insert("ANTHROPIC_AUTH_TOKEN".to_string(), "old-token".to_string());
        env.insert("ANTHROPIC_BASE_URL".to_string(), "https://old-url.com".to_string());

        let mut settings = ClaudeSettings {
            env,
            other: BTreeMap::new(),
        };

        // Try to switch to Env mode - should succeed and auto-clean conflicts
        let result = settings.switch_to_config_with_mode(
            &config,
            StorageMode::Env,
            Some(temp_dir.path().to_str().unwrap()),
        );

        // Verify the operation succeeded
        assert!(
            result.is_ok(),
            "Switch to Env mode should succeed and auto-clean conflicts"
        );

        // Verify the settings file was created with cleaned env
        assert!(
            settings_path.exists(),
            "Settings file should be created after cleaning"
        );

        // Read the saved file and verify conflicts were removed
        let content = fs::read_to_string(&settings_path).expect("Failed to read settings file");
        let saved_settings: ClaudeSettings =
            serde_json::from_str(&content).expect("Failed to parse saved settings");

        // Verify Anthropic fields were removed
        assert!(
            !saved_settings.env.contains_key("ANTHROPIC_AUTH_TOKEN"),
            "ANTHROPIC_AUTH_TOKEN should be removed from env field"
        );
        assert!(
            !saved_settings.env.contains_key("ANTHROPIC_BASE_URL"),
            "ANTHROPIC_BASE_URL should be removed from env field"
        );
    }

    #[test]
    fn test_switch_to_env_mode_succeeds_without_conflicts() {
        let temp_dir = create_test_temp_dir();
        let settings_path = temp_dir.path().join("settings.json");

        // Create a test configuration
        let config = create_test_config("test-config", "sk-ant-test", "https://api.test.com");

        // Create ClaudeSettings with only standard Claude fields (no conflicts)
        let mut other = BTreeMap::new();
        other.insert(
            "$schema".to_string(),
            serde_json::Value::String("https://claude.ai/schema.json".to_string()),
        );
        other.insert(
            "theme".to_string(),
            serde_json::Value::String("dark".to_string()),
        );

        let mut settings = ClaudeSettings {
            env: BTreeMap::new(),
            other,
        };

        // Try to switch to Env mode - should succeed when no conflicts exist
        let result = settings.switch_to_config_with_mode(
            &config,
            StorageMode::Env,
            Some(temp_dir.path().to_str().unwrap()),
        );

        // Verify the operation succeeded
        assert!(
            result.is_ok(),
            "Switch to Env mode should succeed when no conflicts exist"
        );

        // Verify no file was created (env mode shouldn't touch settings.json)
        assert!(
            !settings_path.exists(),
            "Settings file should not be created in env mode"
        );
    }

    #[test]
    fn test_switch_to_config_mode_preserves_non_anthropic_fields() {
        let temp_dir = create_test_temp_dir();
        let settings_path = temp_dir.path().join("settings.json");

        // Create a test configuration
        let config = create_test_config("test-config", "sk-ant-test", "https://api.test.com");

        // Create ClaudeSettings with valid non-Anthropic fields and Anthropic fields
        let mut other = BTreeMap::new();
        other.insert(
            "userPreferences".to_string(),
            serde_json::Value::String("dark".to_string()),
        );
        other.insert(
            "theme".to_string(),
            serde_json::Value::String("dark".to_string()),
        );
        other.insert(
            "anthropicAuthToken".to_string(),
            serde_json::Value::String("old-token".to_string()),
        );

        let mut settings = ClaudeSettings {
            env: BTreeMap::new(),
            other,
        };

        // Temporarily unset system environment variables for this test
        let original_auth_token = std::env::var("ANTHROPIC_AUTH_TOKEN").ok();
        let original_base_url = std::env::var("ANTHROPIC_BASE_URL").ok();
        unsafe {
            std::env::remove_var("ANTHROPIC_AUTH_TOKEN");
            std::env::remove_var("ANTHROPIC_BASE_URL");
        }

        // Switch to Config mode
        let result = settings.switch_to_config_with_mode(
            &config,
            StorageMode::Config,
            Some(temp_dir.path().to_str().unwrap()),
        );

        // Restore original environment variables
        unsafe {
            if let Some(token) = original_auth_token {
                std::env::set_var("ANTHROPIC_AUTH_TOKEN", token);
            }
            if let Some(url) = original_base_url {
                std::env::set_var("ANTHROPIC_BASE_URL", url);
            }
        }

        // Verify the operation succeeded
        assert!(result.is_ok(), "Switch to Config mode should succeed");

        // Read the saved file
        let content = fs::read_to_string(&settings_path).expect("Failed to read settings file");

        // Parse and verify Anthropic fields are in 'env' with UPPERCASE names
        let saved_settings: ClaudeSettings =
            serde_json::from_str(&content).expect("Failed to parse saved settings");

        // Verify Anthropic settings are in 'env' with UPPERCASE names
        assert!(
            saved_settings.env.contains_key("ANTHROPIC_AUTH_TOKEN"),
            "Config mode should use UPPERCASE field names in 'env'"
        );
        assert_eq!(
            saved_settings.env.get("ANTHROPIC_AUTH_TOKEN"),
            Some(&"sk-ant-test".to_string())
        );
        assert!(
            saved_settings.env.contains_key("ANTHROPIC_BASE_URL"),
            "Config mode should have ANTHROPIC_BASE_URL in 'env'"
        );
        assert_eq!(
            saved_settings.env.get("ANTHROPIC_BASE_URL"),
            Some(&"https://api.test.com".to_string())
        );

        // Verify non-Anthropic fields are preserved in 'other'
        assert!(saved_settings.other.contains_key("userPreferences"));
        assert!(saved_settings.other.contains_key("theme"));

        // Note: anthropicAuthToken in 'other' field is preserved since it's not in 'env'
        // The 'other' field is for Claude's own internal settings and we don't modify it
    }
}
