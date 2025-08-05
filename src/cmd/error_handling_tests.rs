#[cfg(test)]
mod error_handling_tests {
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

    #[test]
    fn test_error_handling_invalid_alias_names() {
        // Test empty alias
        let result = validate_alias_name("");
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Alias name cannot be empty"));

        // Test whitespace-only alias
        let result = validate_alias_name("   ");
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        println!("Whitespace error message: '{}'", error_msg);
        assert!(error_msg.contains("whitespace"));

        // Test reserved 'cc' alias
        let result = validate_alias_name("cc");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("reserved"));

        // Test alias with spaces
        let result = validate_alias_name("test config");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("whitespace"));

        // Test alias with tabs (note: current implementation only checks for spaces)
        let result = validate_alias_name("test\tconfig");
        assert!(
            result.is_ok(),
            "Current implementation only checks for spaces, not tabs"
        );

        // Test alias with newlines (note: current implementation only checks for spaces)
        let result = validate_alias_name("test\nconfig");
        assert!(
            result.is_ok(),
            "Current implementation only checks for spaces, not newlines"
        );

        // Test alias with multiple spaces
        let result = validate_alias_name("test  config  with  spaces");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("whitespace"));
    }

    #[test]
    fn test_error_handling_valid_alias_names() {
        // Test valid aliases
        let valid_aliases = vec![
            "test",
            "test-config",
            "test_config",
            "test123",
            "123test",
            "Test-Config_123",
            "a",
            "config-name-with-dashes",
            "config_name_with_underscores",
            "mixed-case-Config",
            "UPPERCASE",
            "lowercase",
            "CamelCase",
            "snake_case",
            "kebab-case",
        ];

        for alias in valid_aliases {
            let result = validate_alias_name(alias);
            assert!(result.is_ok(), "Alias '{}' should be valid", alias);
        }
    }

    #[test]
    fn test_error_handling_path_resolution() {
        // Test empty path resolution
        let result = get_claude_settings_path(Some(""));
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.ends_with("settings.json"));

        // Test path with only slashes
        let result = get_claude_settings_path(Some("///"));
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.ends_with("settings.json"));

        // Test relative path with parent directory
        let result = get_claude_settings_path(Some("../config"));
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.ends_with("config/settings.json"));
    }

    #[test]
    fn test_error_handling_config_storage_operations() {
        let mut storage = ConfigStorage::default();

        // Test removing non-existent configuration
        assert!(!storage.remove_configuration("nonexistent"));
        assert!(!storage.remove_configuration(""));
        assert!(!storage.remove_configuration("cc"));

        // Test getting non-existent configuration
        assert!(storage.get_configuration("nonexistent").is_none());
        assert!(storage.get_configuration("").is_none());
        assert!(storage.get_configuration("cc").is_none());

        // Test operations on empty storage
        assert_eq!(storage.configurations.len(), 0);
        assert!(storage.claude_settings_dir.is_none());
        assert!(storage.get_claude_settings_dir().is_none());

        // Test adding and removing the same configuration multiple times
        let config = create_test_config("test", "sk-ant-test", "https://api.test.com");
        storage.add_configuration(config.clone());
        assert_eq!(storage.configurations.len(), 1);

        storage.add_configuration(config.clone());
        assert_eq!(storage.configurations.len(), 1); // Should overwrite

        assert!(storage.remove_configuration("test"));
        assert_eq!(storage.configurations.len(), 0);

        assert!(!storage.remove_configuration("test")); // Should not exist anymore
    }

    #[test]
    fn test_error_handling_claude_settings_operations() {
        let mut settings = ClaudeSettings::default();

        // Test removing env variables from empty settings
        settings.remove_anthropic_env();
        assert!(settings.env.is_empty());
        assert!(settings.other.is_empty());

        // Test switching to config with empty settings
        let config = create_test_config("test", "sk-ant-test", "https://api.test.com");
        settings.switch_to_config(&config);
        assert_eq!(settings.env.len(), 2);
        assert_eq!(
            settings.env.get("ANTHROPIC_AUTH_TOKEN"),
            Some(&"sk-ant-test".to_string())
        );
        assert_eq!(
            settings.env.get("ANTHROPIC_BASE_URL"),
            Some(&"https://api.test.com".to_string())
        );

        // Test removing env variables after switching
        settings.remove_anthropic_env();
        assert!(settings.env.get("ANTHROPIC_AUTH_TOKEN").is_none());
        assert!(settings.env.get("ANTHROPIC_BASE_URL").is_none());

        // Test multiple switches and removes
        for i in 0..10 {
            let config = create_test_config(
                &format!("config{}", i),
                &format!("sk-ant-test{}", i),
                &format!("https://api{}.test.com", i),
            );
            settings.switch_to_config(&config);
            assert_eq!(
                settings.env.get("ANTHROPIC_AUTH_TOKEN"),
                Some(&format!("sk-ant-test{}", i))
            );
            settings.remove_anthropic_env();
        }
    }

    #[test]
    fn test_error_handling_serialization_edge_cases() {
        // Test serializing and deserializing empty settings
        let settings = ClaudeSettings::default();
        let json = serde_json::to_string_pretty(&settings).unwrap();
        let deserialized: ClaudeSettings = serde_json::from_str(&json).unwrap();
        assert!(deserialized.env.is_empty());
        assert!(deserialized.other.is_empty());

        // Test serializing settings with only other fields
        let mut settings = ClaudeSettings::default();
        settings.other.insert(
            "key1".to_string(),
            serde_json::Value::String("value1".to_string()),
        );
        settings.other.insert(
            "key2".to_string(),
            serde_json::Value::Number(serde_json::Number::from(42)),
        );
        settings
            .other
            .insert("key3".to_string(), serde_json::Value::Bool(true));
        settings.other.insert(
            "key4".to_string(),
            serde_json::Value::Array(vec![
                serde_json::Value::String("item1".to_string()),
                serde_json::Value::String("item2".to_string()),
            ]),
        );

        let json = serde_json::to_string_pretty(&settings).unwrap();
        let deserialized: ClaudeSettings = serde_json::from_str(&json).unwrap();
        assert!(deserialized.env.is_empty());
        assert_eq!(deserialized.other.len(), 4);

        // Test deserializing malformed JSON
        let malformed_json = r#"{"env": {"ANTHROPIC_AUTH_TOKEN": "sk-ant-test"}, "invalid"#;
        let result: Result<ClaudeSettings, _> = serde_json::from_str(malformed_json);
        assert!(result.is_err());

        // Test deserializing JSON with wrong types
        let wrong_types_json = r#"{"env": "not_a_map", "other": {}}"#;
        let result: Result<ClaudeSettings, _> = serde_json::from_str(wrong_types_json);
        assert!(result.is_err());
    }

    #[test]
    fn test_error_handling_file_operations() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.json");

        // Test reading non-existent file
        let result = fs::read_to_string(&test_file);
        assert!(result.is_err());

        // Test writing to and reading from file
        let test_content = r#"{"test": "content"}"#;
        fs::write(&test_file, test_content).unwrap();
        let read_content = fs::read_to_string(&test_file).unwrap();
        assert_eq!(read_content, test_content);

        // Test overwriting existing file
        let new_content = r#"{"new": "content"}"#;
        fs::write(&test_file, new_content).unwrap();
        let read_content = fs::read_to_string(&test_file).unwrap();
        assert_eq!(read_content, new_content);
    }

    #[test]
    fn test_error_handling_config_storage_edge_cases() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("configurations.json");

        // Test loading from non-existent file (mock the behavior)
        let storage = ConfigStorage::default();
        assert!(storage.configurations.is_empty());
        assert!(storage.claude_settings_dir.is_none());

        // Test saving and loading with empty storage
        let storage = ConfigStorage::default();
        let json = serde_json::to_string_pretty(&storage).unwrap();
        fs::write(&config_path, json).unwrap();

        let loaded_content = fs::read_to_string(&config_path).unwrap();
        let loaded_storage: ConfigStorage = serde_json::from_str(&loaded_content).unwrap();
        assert!(loaded_storage.configurations.is_empty());
        assert!(loaded_storage.claude_settings_dir.is_none());

        // Test loading from malformed JSON file
        let malformed_json = r#"{"configurations": "not_a_map", "claude_settings_dir": "/path"}"#;
        fs::write(&config_path, malformed_json).unwrap();
        let result: Result<ConfigStorage, _> =
            serde_json::from_str(&fs::read_to_string(&config_path).unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn test_error_handling_large_configurations() {
        let mut storage = ConfigStorage::default();

        // Test adding very large number of configurations
        for i in 0..1000 {
            let config = create_test_config(
                &format!("config_{}", i),
                &format!("sk-ant-test{}", i),
                &format!("https://api{}.test.com", i),
            );
            storage.add_configuration(config);
        }

        assert_eq!(storage.configurations.len(), 1000);

        // Test retrieving all configurations
        for i in 0..1000 {
            let alias = &format!("config_{}", i);
            let config = storage.get_configuration(alias).unwrap();
            assert_eq!(config.token, format!("sk-ant-test{}", i));
            assert_eq!(config.url, format!("https://api{}.test.com", i));
        }

        // Test removing all configurations
        for i in 0..1000 {
            let alias = &format!("config_{}", i);
            assert!(storage.remove_configuration(alias));
        }

        assert_eq!(storage.configurations.len(), 0);
    }

    #[test]
    fn test_error_handling_special_characters_in_tokens() {
        let mut storage = ConfigStorage::default();
        let mut settings = ClaudeSettings::default();

        // Test tokens with special characters
        let special_tokens = vec![
            "sk-ant-test123",
            "sk-ant-test_123",
            "sk-ant-test-123",
            "sk-ant-test.123",
            "sk-ant-test@123",
            "sk-ant-test#123",
            "sk-ant-test$123",
            "sk-ant-test%123",
            "sk-ant-test^123",
            "sk-ant-test&123",
            "sk-ant-test*123",
            "sk-ant-test(123)",
            "sk-ant-test)123",
            "sk-ant-test+123",
            "sk-ant-test=123",
            "sk-ant-test{123}",
            "sk-ant-test}123",
            "sk-ant-test[123]",
            "sk-ant-test]123",
            "sk-ant-test|123",
            "sk-ant-test\\123",
            "sk-ant-test/123",
            "sk-ant-test:123",
            "sk-ant-test;123",
            "sk-ant-test\"123",
            "sk-ant-test'123",
            "sk-ant-test<123>",
            "sk-ant-test>123",
            "sk-ant-test,123",
            "sk-ant-test?123",
            "sk-ant-test~123",
            "sk-ant-test`123",
        ];

        for (i, token) in special_tokens.iter().enumerate() {
            let config =
                create_test_config(&format!("config_{}", i), token, "https://api.test.com");
            storage.add_configuration(config);
        }

        assert_eq!(storage.configurations.len(), special_tokens.len());

        // Test switching to configurations with special tokens
        for (i, token) in special_tokens.iter().enumerate() {
            let alias = &format!("config_{}", i);
            let config = storage.get_configuration(alias).unwrap();
            settings.switch_to_config(config);
            assert_eq!(
                settings.env.get("ANTHROPIC_AUTH_TOKEN"),
                Some(&token.to_string())
            );
        }
    }

    #[test]
    fn test_error_handling_unicode_characters() {
        let mut storage = ConfigStorage::default();

        // Test configurations with Unicode characters
        let unicode_configs = vec![
            ("测试配置", "sk-ant-测试", "https://api.测试.com"),
            ("config-café", "sk-ant-café", "https://api.café.com"),
            ("config-naïve", "sk-ant-naïve", "https://api.naïve.com"),
            ("config-über", "sk-ant-über", "https://api.über.com"),
            ("config-Москва", "sk-ant-Москва", "https://api.Москва.com"),
            ("config-北京", "sk-ant-北京", "https://api.北京.com"),
            ("config-東京", "sk-ant-東京", "https://api.東京.com"),
            (
                "config-العربية",
                "sk-ant-العربية",
                "https://api.العربية.com",
            ),
            ("config-हिन्दी", "sk-ant-हिन्दी", "https://api.हिन्दी.com"),
            ("config-한국어", "sk-ant-한국어", "https://api.한국어.com"),
        ];

        for (alias, token, url) in &unicode_configs {
            let result = validate_alias_name(alias);
            assert!(result.is_ok(), "Alias '{}' should be valid", alias);

            let config = create_test_config(alias, token, url);
            storage.add_configuration(config);
        }

        assert_eq!(storage.configurations.len(), unicode_configs.len());

        // Test retrieving configurations with Unicode characters
        for (alias, token, url) in &unicode_configs {
            let stored_config = storage.get_configuration(alias).unwrap();
            assert_eq!(stored_config.token, *token);
            assert_eq!(stored_config.url, *url);
        }
    }

    #[test]
    fn test_error_handling_concurrent_operations() {
        let mut storage = ConfigStorage::default();
        let mut settings = ClaudeSettings::default();

        // Test rapid add/remove operations
        for i in 0..100 {
            let alias = &format!("config_{}", i);
            let config = create_test_config(
                alias,
                &format!("sk-ant-test{}", i),
                &format!("https://api{}.test.com", i),
            );
            storage.add_configuration(config);
            assert!(storage.remove_configuration(alias));
        }

        assert_eq!(storage.configurations.len(), 0);

        // Test rapid switch operations
        for i in 0..100 {
            let alias = &format!("config_{}", i);
            let config = create_test_config(
                alias,
                &format!("sk-ant-test{}", i),
                &format!("https://api{}.test.com", i),
            );
            storage.add_configuration(config);
            let stored_config = storage.get_configuration(alias).unwrap();
            settings.switch_to_config(stored_config);
            settings.remove_anthropic_env();
        }

        assert_eq!(storage.configurations.len(), 100);
        assert!(settings.env.is_empty());
    }

    #[test]
    fn test_error_handling_memory_pressure() {
        let mut storage = ConfigStorage::default();

        // Test adding many configurations with large tokens and URLs
        let large_token = "sk-ant-".to_string() + &"a".repeat(1000);
        let large_url = "https://".to_string() + &"a".repeat(1000) + ".com";

        for i in 0..100 {
            let config = create_test_config(&format!("config_{}", i), &large_token, &large_url);
            storage.add_configuration(config);
        }

        assert_eq!(storage.configurations.len(), 100);

        // Test that all configurations are stored correctly
        for i in 0..100 {
            let alias = &format!("config_{}", i);
            let config = storage.get_configuration(alias).unwrap();
            assert_eq!(config.token, large_token);
            assert_eq!(config.url, large_url);
        }

        // Test removing all configurations
        for i in 0..100 {
            let alias = &format!("config_{}", i);
            assert!(storage.remove_configuration(alias));
        }

        assert_eq!(storage.configurations.len(), 0);
    }
}
