#[cfg(test)]
#[allow(clippy::module_inception)]
mod error_handling_tests {
    use crate::cmd::config::*;

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

        // Test alias with tabs
        let result = validate_alias_name("test\tconfig");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("whitespace"));

        // Test alias with newlines
        let result = validate_alias_name("test\nconfig");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("whitespace"));
    }

    #[test]
    fn test_error_handling_configuration_storage_edge_cases() {
        let mut storage = ConfigStorage::default();

        // Test adding configuration with very long names
        let long_alias = "a".repeat(1000);
        let config = create_test_config(&long_alias, "sk-ant-test", "https://api.test.com");
        storage.add_configuration(config);
        assert!(storage.configurations.contains_key(&long_alias));

        // Test removing non-existent configuration
        assert!(!storage.remove_configuration("non-existent"));

        // Test getting non-existent configuration
        assert!(storage.get_configuration("non-existent").is_none());
    }

    #[test]
    fn test_error_handling_environment_config_edge_cases() {
        // Test with empty model fields
        let mut config = create_test_config("test", "sk-ant-test", "https://api.test.com");
        config.model = Some("".to_string());
        config.small_fast_model = Some("".to_string());

        let env_config = EnvironmentConfig::from_config(&config);

        // Empty model fields should not be included in environment variables
        assert_eq!(env_config.env_vars.len(), 2); // Only token and URL
        assert!(!env_config.env_vars.contains_key("ANTHROPIC_MODEL"));
        assert!(
            !env_config
                .env_vars
                .contains_key("ANTHROPIC_SMALL_FAST_MODEL")
        );
    }

    #[test]
    fn test_error_handling_json_serialization() {
        let mut storage = ConfigStorage::default();
        let config1 = create_test_config("config1", "sk-ant-test1", "https://api1.test.com");
        let config2 = create_test_config("config2", "sk-ant-test2", "https://api2.test.com");

        storage.add_configuration(config1);
        storage.add_configuration(config2);

        // Test serialization
        let json_result = serde_json::to_string_pretty(&storage);
        assert!(json_result.is_ok());

        // Test deserialization
        let json = json_result.unwrap();
        let deserialization_result: Result<ConfigStorage, _> = serde_json::from_str(&json);
        assert!(deserialization_result.is_ok());

        let deserialized = deserialization_result.unwrap();
        assert_eq!(deserialized.configurations.len(), 2);
    }
}
