#[cfg(test)]
mod integration_tests {
    use crate::cmd::config::*;
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
    fn test_integration_configuration_workflow() {
        let mut storage = ConfigStorage::default();

        // Test full workflow: add -> get -> remove
        let config = create_test_config(
            "integration-test",
            "sk-ant-integration",
            "https://api.integration.com",
        );

        // Add configuration
        storage.add_configuration(config.clone());
        assert!(storage.configurations.contains_key("integration-test"));

        // Get configuration
        let retrieved = storage.get_configuration("integration-test").unwrap();
        assert_eq!(retrieved.alias_name, config.alias_name);
        assert_eq!(retrieved.token, config.token);
        assert_eq!(retrieved.url, config.url);

        // Remove configuration
        assert!(storage.remove_configuration("integration-test"));
        assert!(!storage.configurations.contains_key("integration-test"));
    }

    #[test]
    fn test_integration_environment_config_workflow() {
        // Test configuration to environment config conversion
        let mut config = create_test_config("test", "sk-ant-test", "https://api.test.com");
        config.model = Some("claude-3-5-sonnet-20241022".to_string());
        config.small_fast_model = Some("claude-3-haiku-20240307".to_string());

        let env_config = EnvironmentConfig::from_config(&config);

        // Verify all environment variables are set
        assert_eq!(env_config.env_vars.len(), 4);
        assert!(env_config.env_vars.contains_key("ANTHROPIC_AUTH_TOKEN"));
        assert!(env_config.env_vars.contains_key("ANTHROPIC_BASE_URL"));
        assert!(env_config.env_vars.contains_key("ANTHROPIC_MODEL"));
        assert!(
            env_config
                .env_vars
                .contains_key("ANTHROPIC_SMALL_FAST_MODEL")
        );

        // Test conversion to tuples for command execution
        let tuples = env_config.as_env_tuples();
        assert_eq!(tuples.len(), 4);
    }

    #[test]
    fn test_integration_storage_persistence() {
        let temp_dir = create_test_temp_dir();
        let test_config_path = temp_dir.path().join("configurations.json");

        // Create storage with multiple configurations
        let mut storage = ConfigStorage::default();
        for i in 0..5 {
            let config = create_test_config(
                &format!("config{}", i),
                &format!("sk-ant-test{}", i),
                &format!("https://api{}.test.com", i),
            );
            storage.add_configuration(config);
        }

        // Save to file
        let json = serde_json::to_string_pretty(&storage).unwrap();
        fs::write(&test_config_path, json).unwrap();

        // Load from file
        let loaded_content = fs::read_to_string(&test_config_path).unwrap();
        let loaded_storage: ConfigStorage = serde_json::from_str(&loaded_content).unwrap();

        // Verify all configurations are preserved
        assert_eq!(loaded_storage.configurations.len(), 5);
        for i in 0..5 {
            let alias = format!("config{}", i);
            assert!(loaded_storage.configurations.contains_key(&alias));

            let config = loaded_storage.get_configuration(&alias).unwrap();
            assert_eq!(config.token, format!("sk-ant-test{}", i));
            assert_eq!(config.url, format!("https://api{}.test.com", i));
        }
    }

    #[test]
    fn test_integration_alias_validation_workflow() {
        // Test various alias validation scenarios
        let test_cases = vec![
            ("valid-alias", true),
            ("another_valid_alias", true),
            ("ValidAlias123", true),
            ("", false),            // empty
            ("cc", false),          // reserved
            ("test config", false), // contains space
        ];

        for (alias, should_be_valid) in test_cases {
            let result = validate_alias_name(alias);
            if should_be_valid {
                assert!(result.is_ok(), "Expected '{}' to be valid", alias);
            } else {
                assert!(result.is_err(), "Expected '{}' to be invalid", alias);
            }
        }
    }

    #[test]
    fn test_integration_config_path_resolution() {
        let result = get_config_storage_path();
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(path.to_string_lossy().contains(".cc-switch"));
        assert!(path.to_string_lossy().ends_with("configurations.json"));
    }
}
