#[cfg(test)]
mod integration_tests {
    use crate::cmd::main::*;
    use std::fs;
    use tempfile::TempDir;

    /// Helper function to create a test configuration
    fn create_test_config(alias: &str, token: &str, url: &str) -> Configuration {
        Configuration {
            alias_name: alias.to_string(),
            token: token.to_string(),
            url: url.to_string(),
        }
    }

    /// Helper function to create a temporary directory for testing
    fn create_test_temp_dir() -> TempDir {
        TempDir::new().expect("Failed to create temporary directory")
    }

    #[test]
    fn test_integration_full_workflow() {
        let _temp_dir = create_test_temp_dir();

        // Create a test storage
        let mut storage = ConfigStorage::default();

        // 1. Add multiple configurations
        let config1 = create_test_config("prod", "sk-ant-prod", "https://api.anthropic.com");
        let config2 = create_test_config("dev", "sk-ant-dev", "https://api.dev.anthropic.com");
        let config3 = create_test_config("test", "sk-ant-test", "https://api.test.anthropic.com");

        storage.add_configuration(config1);
        storage.add_configuration(config2);
        storage.add_configuration(config3);

        // 2. Verify all configurations are stored
        assert_eq!(storage.configurations.len(), 3);
        assert!(storage.get_configuration("prod").is_some());
        assert!(storage.get_configuration("dev").is_some());
        assert!(storage.get_configuration("test").is_some());

        // 3. Test configuration switching
        let mut settings = ClaudeSettings::default();
        let prod_config = storage.get_configuration("prod").unwrap();
        settings.switch_to_config(prod_config);

        assert_eq!(
            settings.env.get("ANTHROPIC_AUTH_TOKEN"),
            Some(&"sk-ant-prod".to_string())
        );
        assert_eq!(
            settings.env.get("ANTHROPIC_BASE_URL"),
            Some(&"https://api.anthropic.com".to_string())
        );

        // 4. Switch to dev configuration
        let dev_config = storage.get_configuration("dev").unwrap();
        settings.switch_to_config(dev_config);

        assert_eq!(
            settings.env.get("ANTHROPIC_AUTH_TOKEN"),
            Some(&"sk-ant-dev".to_string())
        );
        assert_eq!(
            settings.env.get("ANTHROPIC_BASE_URL"),
            Some(&"https://api.dev.anthropic.com".to_string())
        );

        // 5. Remove test configuration
        assert!(storage.remove_configuration("test"));
        assert_eq!(storage.configurations.len(), 2);
        assert!(storage.get_configuration("test").is_none());

        // 6. Verify remaining configurations
        assert!(storage.get_configuration("prod").is_some());
        assert!(storage.get_configuration("dev").is_some());

        // 7. Test reset to default
        settings.remove_anthropic_env();
        assert!(settings.env.get("ANTHROPIC_AUTH_TOKEN").is_none());
        assert!(settings.env.get("ANTHROPIC_BASE_URL").is_none());
    }

    #[test]
    fn test_integration_error_handling() {
        let mut storage = ConfigStorage::default();

        // Test adding configuration with invalid alias
        let result = validate_alias_name("");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Alias name cannot be empty"
        );

        let result = validate_alias_name("cc");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Alias name 'cc' is reserved and cannot be used"
        );

        let result = validate_alias_name("test config");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Alias name cannot contain whitespace"
        );

        // Test removing non-existent configuration
        assert!(!storage.remove_configuration("nonexistent"));

        // Test getting non-existent configuration
        assert!(storage.get_configuration("nonexistent").is_none());

        // Test with valid alias
        let result = validate_alias_name("valid-config");
        assert!(result.is_ok());
    }

    #[test]
    fn test_integration_settings_persistence() {
        let temp_dir = create_test_temp_dir();
        let config_path = temp_dir.path().join("configurations.json");
        let settings_path = temp_dir.path().join("settings.json");

        // 1. Create and save storage

        let mut storage = ConfigStorage::default();
        let config =
            create_test_config("persist-test", "sk-ant-persist", "https://api.persist.com");
        storage.add_configuration(config);

        // Mock save operation
        let json = serde_json::to_string_pretty(&storage).unwrap();
        fs::write(&config_path, json).unwrap();

        // 2. Load and verify storage
        let loaded_content = fs::read_to_string(&config_path).unwrap();
        let loaded_storage: ConfigStorage = serde_json::from_str(&loaded_content).unwrap();

        assert_eq!(loaded_storage.configurations.len(), 1);
        let loaded_config = loaded_storage.get_configuration("persist-test").unwrap();
        assert_eq!(loaded_config.token, "sk-ant-persist");
        assert_eq!(loaded_config.url, "https://api.persist.com");

        // 3. Test settings persistence
        let mut settings = ClaudeSettings::default();
        settings.switch_to_config(loaded_config);
        settings.other.insert(
            "test_key".to_string(),
            serde_json::Value::String("test_value".to_string()),
        );

        // Save settings
        let settings_json = serde_json::to_string_pretty(&settings).unwrap();
        fs::write(&settings_path, settings_json).unwrap();

        // Load and verify settings
        let loaded_settings_content = fs::read_to_string(&settings_path).unwrap();
        let loaded_settings: ClaudeSettings =
            serde_json::from_str(&loaded_settings_content).unwrap();

        assert_eq!(
            loaded_settings.env.get("ANTHROPIC_AUTH_TOKEN"),
            Some(&"sk-ant-persist".to_string())
        );
        assert_eq!(
            loaded_settings.env.get("ANTHROPIC_BASE_URL"),
            Some(&"https://api.persist.com".to_string())
        );
        assert_eq!(
            loaded_settings.other.get("test_key"),
            Some(&serde_json::Value::String("test_value".to_string()))
        );
    }

    #[test]
    fn test_integration_path_resolution() {
        // Test absolute path
        let absolute_path = get_claude_settings_path(Some("/absolute/path/to/claude")).unwrap();
        assert_eq!(
            absolute_path,
            std::path::PathBuf::from("/absolute/path/to/claude/settings.json")
        );

        // Test relative path
        let relative_path = get_claude_settings_path(Some("relative/path")).unwrap();
        assert!(relative_path.ends_with("relative/path/settings.json"));

        // Test default path
        let default_path = get_claude_settings_path(None).unwrap();
        assert!(default_path.ends_with(".claude/settings.json"));

        // Test config storage path
        let config_path = get_config_storage_path().unwrap();
        assert!(config_path.ends_with(".cc-switch/configurations.json"));
    }

    #[test]
    fn test_integration_configuration_management() {
        let mut storage = ConfigStorage::default();

        // Test adding configurations with various tokens and URLs
        let test_configs = vec![
            ("config1", "sk-ant-api1", "https://api1.example.com"),
            ("config2", "sk-ant-api2", "https://api2.example.com"),
            ("config3", "sk-ant-api3", "https://api3.example.com"),
        ];

        for (alias, token, url) in &test_configs {
            let config = create_test_config(alias, token, url);
            storage.add_configuration(config);
        }

        // Verify all configurations are stored
        assert_eq!(storage.configurations.len(), 3);

        // Test retrieving each configuration
        for (alias, token, url) in &test_configs {
            let stored_config = storage.get_configuration(alias).unwrap();
            assert_eq!(stored_config.token, *token);
            assert_eq!(stored_config.url, *url);
        }

        // Test removing configurations in different order
        assert!(storage.remove_configuration("config2"));
        assert_eq!(storage.configurations.len(), 2);
        assert!(storage.get_configuration("config2").is_none());

        assert!(storage.remove_configuration("config1"));
        assert_eq!(storage.configurations.len(), 1);
        assert!(storage.get_configuration("config1").is_none());

        assert!(storage.remove_configuration("config3"));
        assert_eq!(storage.configurations.len(), 0);
        assert!(storage.get_configuration("config3").is_none());
    }

    #[test]
    fn test_integration_settings_serialization_edge_cases() {
        // Test settings with empty env
        let mut settings = ClaudeSettings::default();
        settings.other.insert(
            "key1".to_string(),
            serde_json::Value::String("value1".to_string()),
        );
        settings.other.insert(
            "key2".to_string(),
            serde_json::Value::Number(serde_json::Number::from(42)),
        );

        let json = serde_json::to_string_pretty(&settings).unwrap();
        assert!(!json.contains("\"env\""));

        let deserialized: ClaudeSettings = serde_json::from_str(&json).unwrap();
        assert!(deserialized.env.is_empty());
        assert_eq!(deserialized.other.len(), 2);

        // Test settings with only env
        let mut settings = ClaudeSettings::default();
        settings.env.insert(
            "ANTHROPIC_AUTH_TOKEN".to_string(),
            "sk-ant-test".to_string(),
        );
        settings.env.insert(
            "ANTHROPIC_BASE_URL".to_string(),
            "https://api.test.com".to_string(),
        );

        let json = serde_json::to_string_pretty(&settings).unwrap();
        assert!(json.contains("\"env\""));

        let deserialized: ClaudeSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.env.len(), 2);
        assert!(deserialized.other.is_empty());

        // Test settings with both env and other fields
        let mut settings = ClaudeSettings::default();
        settings.env.insert(
            "ANTHROPIC_AUTH_TOKEN".to_string(),
            "sk-ant-test".to_string(),
        );
        settings.other.insert(
            "other_key".to_string(),
            serde_json::Value::String("other_value".to_string()),
        );

        let json = serde_json::to_string_pretty(&settings).unwrap();
        assert!(json.contains("\"env\""));
        assert!(json.contains("other_key"));

        let deserialized: ClaudeSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.env.len(), 1);
        assert_eq!(deserialized.other.len(), 1);
    }

    #[test]
    fn test_integration_configuration_switching_scenarios() {
        let mut storage = ConfigStorage::default();
        let mut settings = ClaudeSettings::default();

        // Add test configurations
        let config1 = create_test_config("config1", "sk-ant-test1", "https://api1.test.com");
        let config2 = create_test_config("config2", "sk-ant-test2", "https://api2.test.com");

        storage.add_configuration(config1);
        storage.add_configuration(config2);

        // Test switching to first configuration
        let target_config = storage.get_configuration("config1").unwrap();
        settings.switch_to_config(target_config);

        assert_eq!(
            settings.env.get("ANTHROPIC_AUTH_TOKEN"),
            Some(&"sk-ant-test1".to_string())
        );
        assert_eq!(
            settings.env.get("ANTHROPIC_BASE_URL"),
            Some(&"https://api1.test.com".to_string())
        );

        // Test switching to second configuration
        let target_config = storage.get_configuration("config2").unwrap();
        settings.switch_to_config(target_config);

        assert_eq!(
            settings.env.get("ANTHROPIC_AUTH_TOKEN"),
            Some(&"sk-ant-test2".to_string())
        );
        assert_eq!(
            settings.env.get("ANTHROPIC_BASE_URL"),
            Some(&"https://api2.test.com".to_string())
        );

        // Test removing environment variables
        settings.remove_anthropic_env();

        assert!(settings.env.get("ANTHROPIC_AUTH_TOKEN").is_none());
        assert!(settings.env.get("ANTHROPIC_BASE_URL").is_none());

        // Test switching after removing env variables
        let target_config = storage.get_configuration("config1").unwrap();
        settings.switch_to_config(target_config);

        assert_eq!(
            settings.env.get("ANTHROPIC_AUTH_TOKEN"),
            Some(&"sk-ant-test1".to_string())
        );
        assert_eq!(
            settings.env.get("ANTHROPIC_BASE_URL"),
            Some(&"https://api1.test.com".to_string())
        );
    }

    #[test]
    fn test_integration_custom_directory_management() {
        let mut storage = ConfigStorage::default();

        // Test setting custom directory
        storage.set_claude_settings_dir("/custom/claude/path".to_string());

        assert_eq!(
            storage.get_claude_settings_dir(),
            Some(&"/custom/claude/path".to_string())
        );

        // Test path resolution with custom directory
        let custom_path = get_claude_settings_path(Some("/custom/claude/path")).unwrap();
        assert_eq!(
            custom_path,
            std::path::PathBuf::from("/custom/claude/path/settings.json")
        );

        // Test changing custom directory
        storage.set_claude_settings_dir("/another/custom/path".to_string());

        assert_eq!(
            storage.get_claude_settings_dir(),
            Some(&"/another/custom/path".to_string())
        );

        let another_path = get_claude_settings_path(Some("/another/custom/path")).unwrap();
        assert_eq!(
            another_path,
            std::path::PathBuf::from("/another/custom/path/settings.json")
        );
    }

    #[test]
    fn test_integration_large_dataset_handling() {
        let mut storage = ConfigStorage::default();

        // Add many configurations
        for i in 0..100 {
            let config = create_test_config(
                &format!("config_{}", i),
                &format!("sk-ant-test{}", i),
                &format!("https://api{}.test.com", i),
            );
            storage.add_configuration(config);
        }

        // Verify all configurations are stored
        assert_eq!(storage.configurations.len(), 100);

        // Test retrieving configurations
        for i in 0..100 {
            let alias = &format!("config_{}", i);
            let config = storage.get_configuration(alias).unwrap();
            assert_eq!(config.token, format!("sk-ant-test{}", i));
            assert_eq!(config.url, format!("https://api{}.test.com", i));
        }

        // Test removing every other configuration
        for i in (0..100).step_by(2) {
            let alias = &format!("config_{}", i);
            assert!(storage.remove_configuration(alias));
        }

        // Verify remaining configurations
        assert_eq!(storage.configurations.len(), 50);

        for i in (1..100).step_by(2) {
            let alias = &format!("config_{}", i);
            assert!(storage.get_configuration(alias).is_some());
        }

        for i in (0..100).step_by(2) {
            let alias = &format!("config_{}", i);
            assert!(storage.get_configuration(alias).is_none());
        }
    }
}
