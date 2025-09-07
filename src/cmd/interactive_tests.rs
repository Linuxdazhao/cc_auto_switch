#[cfg(test)]
mod tests {
    use crate::cmd::interactive::*;
    use crate::cmd::config::{ConfigStorage, Configuration, EnvironmentConfig};

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

    /// Helper function to create configuration with all fields
    fn create_full_config(
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
        }
    }

    /// Helper to create test storage with configurations
    fn create_test_storage_with_configs() -> ConfigStorage {
        let mut storage = ConfigStorage::default();
        storage.add_configuration(create_test_config("test1", "sk-ant-test1", "https://api1.test.com"));
        storage.add_configuration(create_test_config("test2", "sk-ant-test2", "https://api2.test.com"));
        storage.add_configuration(create_full_config(
            "full-config", 
            "sk-ant-full", 
            "https://full.api.com",
            Some("claude-3-5-sonnet-20241022"),
            Some("claude-3-haiku-20240307")
        ));
        storage
    }

    /// Helper to create large storage for pagination testing
    fn create_large_storage() -> ConfigStorage {
        let mut storage = ConfigStorage::default();
        
        // Create 15 configurations to test pagination
        for i in 0..15 {
            let config = create_test_config(
                &format!("config{:02}", i),
                &format!("sk-ant-token{:02}", i),
                &format!("https://api{:02}.test.com", i),
            );
            storage.add_configuration(config);
        }
        
        storage
    }

    // Test Environment Configuration Functions
    #[test]
    fn test_environment_config_creation() {
        let config = create_test_config("test", "sk-ant-test", "https://api.test.com");
        let env_config = EnvironmentConfig::from_config(&config);
        
        let env_tuples = env_config.as_env_tuples();
        assert_eq!(env_tuples.len(), 2);
        assert!(env_tuples.contains(&("ANTHROPIC_AUTH_TOKEN".to_string(), "sk-ant-test".to_string())));
        assert!(env_tuples.contains(&("ANTHROPIC_BASE_URL".to_string(), "https://api.test.com".to_string())));
    }

    #[test]
    fn test_environment_config_with_models() {
        let config = create_full_config(
            "full-test",
            "sk-ant-full-test", 
            "https://full-test.api.com",
            Some("claude-3-5-sonnet-20241022"),
            Some("claude-3-haiku-20240307")
        );
        let env_config = EnvironmentConfig::from_config(&config);
        
        let env_tuples = env_config.as_env_tuples();
        assert_eq!(env_tuples.len(), 4);
        assert!(env_tuples.contains(&("ANTHROPIC_AUTH_TOKEN".to_string(), "sk-ant-full-test".to_string())));
        assert!(env_tuples.contains(&("ANTHROPIC_BASE_URL".to_string(), "https://full-test.api.com".to_string())));
        assert!(env_tuples.contains(&("ANTHROPIC_MODEL".to_string(), "claude-3-5-sonnet-20241022".to_string())));
        assert!(env_tuples.contains(&("ANTHROPIC_SMALL_FAST_MODEL".to_string(), "claude-3-haiku-20240307".to_string())));
    }

    #[test]
    fn test_empty_environment_config() {
        let env_config = EnvironmentConfig::empty();
        let env_tuples = env_config.as_env_tuples();
        
        assert!(env_tuples.is_empty());
    }

    // Test Configuration Storage Functionality
    #[test]
    fn test_storage_configuration_retrieval() {
        let storage = create_test_storage_with_configs();
        
        assert_eq!(storage.configurations.len(), 3);
        
        let test1 = storage.get_configuration("test1").unwrap();
        assert_eq!(test1.alias_name, "test1");
        assert_eq!(test1.token, "sk-ant-test1");
        assert_eq!(test1.url, "https://api1.test.com");
        
        let full_config = storage.get_configuration("full-config").unwrap();
        assert_eq!(full_config.model, Some("claude-3-5-sonnet-20241022".to_string()));
        assert_eq!(full_config.small_fast_model, Some("claude-3-haiku-20240307".to_string()));
        
        assert!(storage.get_configuration("nonexistent").is_none());
    }

    #[test]
    fn test_storage_empty_configuration_list() {
        let storage = ConfigStorage::default();
        
        assert!(storage.configurations.is_empty());
        assert!(storage.get_configuration("any").is_none());
    }

    #[test]
    fn test_storage_single_configuration() {
        let mut storage = ConfigStorage::default();
        storage.add_configuration(create_test_config("single", "sk-ant-single", "https://single.api.com"));
        
        assert_eq!(storage.configurations.len(), 1);
        
        let config = storage.get_configuration("single").unwrap();
        assert_eq!(config.alias_name, "single");
        assert_eq!(config.token, "sk-ant-single");
        assert_eq!(config.url, "https://single.api.com");
    }

    // Test Configuration Sorting and Ordering
    #[test]
    fn test_configuration_ordering() {
        let storage = create_test_storage_with_configs();
        
        // Test that configurations are stored and can be retrieved
        let mut aliases: Vec<String> = storage.configurations.keys().cloned().collect();
        aliases.sort();
        
        assert_eq!(aliases, vec!["full-config", "test1", "test2"]);
    }

    #[test]
    fn test_large_configuration_set() {
        let storage = create_large_storage();
        
        assert_eq!(storage.configurations.len(), 15);
        
        // Test that all configurations are accessible
        for i in 0..15 {
            let alias = format!("config{:02}", i);
            let config = storage.get_configuration(&alias);
            assert!(config.is_some(), "Configuration {} should exist", alias);
            
            let config = config.unwrap();
            assert_eq!(config.alias_name, alias);
            assert_eq!(config.token, format!("sk-ant-token{:02}", i));
            assert_eq!(config.url, format!("https://api{:02}.test.com", i));
        }
    }

    // Test Edge Cases and Error Conditions
    #[test]
    fn test_configuration_with_empty_fields() {
        let config = Configuration {
            alias_name: "".to_string(),
            token: "".to_string(),
            url: "".to_string(),
            model: Some("".to_string()),
            small_fast_model: Some("".to_string()),
        };
        
        let env_config = EnvironmentConfig::from_config(&config);
        let env_tuples = env_config.as_env_tuples();
        
        assert_eq!(env_tuples.len(), 4);
        assert!(env_tuples.contains(&("ANTHROPIC_AUTH_TOKEN".to_string(), "".to_string())));
        assert!(env_tuples.contains(&("ANTHROPIC_BASE_URL".to_string(), "".to_string())));
        assert!(env_tuples.contains(&("ANTHROPIC_MODEL".to_string(), "".to_string())));
        assert!(env_tuples.contains(&("ANTHROPIC_SMALL_FAST_MODEL".to_string(), "".to_string())));
    }

    #[test]
    fn test_configuration_with_unicode() {
        let config = create_test_config(
            "测试-unicode", 
            "sk-ant-测试-🔑", 
            "https://αpi.测试.com"
        );
        
        let env_config = EnvironmentConfig::from_config(&config);
        let env_tuples = env_config.as_env_tuples();
        
        assert_eq!(env_tuples.len(), 2);
        assert!(env_tuples.contains(&("ANTHROPIC_AUTH_TOKEN".to_string(), "sk-ant-测试-🔑".to_string())));
        assert!(env_tuples.contains(&("ANTHROPIC_BASE_URL".to_string(), "https://αpi.测试.com".to_string())));
    }

    #[test]
    fn test_configuration_with_very_long_values() {
        let long_alias = "a".repeat(1000);
        let long_token = format!("sk-ant-{}", "b".repeat(1000));
        let long_url = format!("https://{}.com", "c".repeat(1000));
        
        let config = create_test_config(&long_alias, &long_token, &long_url);
        
        assert_eq!(config.alias_name.len(), 1000);
        assert_eq!(config.token.len(), 1007); // "sk-ant-" + 1000
        assert_eq!(config.url.len(), 1011); // "https://" + 1000 + ".com"
        
        let env_config = EnvironmentConfig::from_config(&config);
        let env_tuples = env_config.as_env_tuples();
        
        assert_eq!(env_tuples.len(), 2);
        // Test that the values are preserved correctly
        let token_tuple = env_tuples.iter()
            .find(|(k, _)| k == "ANTHROPIC_AUTH_TOKEN")
            .unwrap();
        assert_eq!(token_tuple.1.len(), 1007);
    }

    // Test Configuration Cloning and Equality
    #[test]
    fn test_configuration_clone() {
        let original = create_full_config(
            "clone-test",
            "sk-ant-clone",
            "https://clone.api.com",
            Some("claude-model"),
            Some("haiku-model"),
        );
        
        let cloned = original.clone();
        
        assert_eq!(original.alias_name, cloned.alias_name);
        assert_eq!(original.token, cloned.token);
        assert_eq!(original.url, cloned.url);
        assert_eq!(original.model, cloned.model);
        assert_eq!(original.small_fast_model, cloned.small_fast_model);
    }

    #[test]
    fn test_storage_overwrite_configuration() {
        let mut storage = ConfigStorage::default();
        
        let config1 = create_test_config("duplicate", "sk-ant-first", "https://first.api.com");
        let config2 = create_test_config("duplicate", "sk-ant-second", "https://second.api.com");
        
        storage.add_configuration(config1);
        assert_eq!(storage.configurations.len(), 1);
        
        let first_config = storage.get_configuration("duplicate").unwrap();
        assert_eq!(first_config.token, "sk-ant-first");
        
        storage.add_configuration(config2);
        assert_eq!(storage.configurations.len(), 1); // Still only one config
        
        let second_config = storage.get_configuration("duplicate").unwrap();
        assert_eq!(second_config.token, "sk-ant-second");
        assert_eq!(second_config.url, "https://second.api.com");
    }

    // Interactive Function Behavior Tests (Testing Logic, Not Terminal I/O)
    #[test]
    fn test_handle_current_command_no_panic() {
        // Test that handle_current_command doesn't panic in test environment
        // This will fail due to no terminal but shouldn't panic
        let result = handle_current_command();
        
        match result {
            Ok(_) => {
                // Successful execution (unlikely in test environment)
            }
            Err(e) => {
                // Expected failure due to test environment limitations
                let error_msg = e.to_string();
                assert!(!error_msg.is_empty(), "Error message should not be empty");
            }
        }
    }

    #[test]
    fn test_handle_interactive_selection_no_panic() {
        let storage = create_test_storage_with_configs();
        
        // Test that handle_interactive_selection doesn't panic in test environment
        let result = handle_interactive_selection(&storage);
        
        match result {
            Ok(_) => {
                // Successful execution (unlikely in test environment)
            }
            Err(e) => {
                // Expected failure due to test environment limitations
                let error_msg = e.to_string();
                assert!(!error_msg.is_empty(), "Error message should not be empty");
            }
        }
    }

    #[test]
    fn test_handle_interactive_selection_empty_storage() {
        let storage = ConfigStorage::default();
        
        let result = handle_interactive_selection(&storage);
        
        match result {
            Ok(_) => {
                // Successful execution
            }
            Err(e) => {
                // May fail due to no configurations or test environment
                let error_msg = e.to_string();
                assert!(!error_msg.is_empty(), "Error message should not be empty");
            }
        }
    }

    // Input/Output Function Tests
    #[test]
    fn test_read_input_function_exists() {
        // We can't easily test the actual input reading in a test environment,
        // but we can verify the function exists and has the right signature
        
        // This would normally require stdin input, so we test it would fail appropriately
        let result = read_input("Test prompt: ");
        
        match result {
            Ok(_) => {
                // Unlikely to succeed without actual input
            }
            Err(e) => {
                // Expected failure in test environment
                let error_msg = e.to_string();
                assert!(!error_msg.is_empty(), "Error should provide meaningful message");
            }
        }
    }

    #[test]
    fn test_read_sensitive_input_function_exists() {
        // Similar to read_input, we test the function exists and fails appropriately
        let result = read_sensitive_input("Enter password: ");
        
        match result {
            Ok(_) => {
                // Unlikely to succeed without actual input
            }
            Err(e) => {
                // Expected failure in test environment
                let error_msg = e.to_string();
                assert!(!error_msg.is_empty(), "Error should provide meaningful message");
            }
        }
    }

    // Performance and Stress Tests
    #[test]
    fn test_large_storage_performance() {
        let start = std::time::Instant::now();
        
        let storage = create_large_storage();
        
        // Test retrieval performance with large dataset
        for i in 0..15 {
            let alias = format!("config{:02}", i);
            let config = storage.get_configuration(&alias);
            assert!(config.is_some());
        }
        
        let duration = start.elapsed();
        
        // Should complete quickly (1 second is very generous)
        assert!(duration.as_secs() < 1, "Large storage operations should be fast, took {:?}", duration);
    }

    #[test]
    fn test_very_large_storage_scalability() {
        let start = std::time::Instant::now();
        
        let mut storage = ConfigStorage::default();
        
        // Add 1000 configurations
        for i in 0..1000 {
            let config = create_test_config(
                &format!("scale-test-{:04}", i),
                &format!("sk-ant-scale-{:04}", i),
                &format!("https://scale-{:04}.api.com", i),
            );
            storage.add_configuration(config);
        }
        
        assert_eq!(storage.configurations.len(), 1000);
        
        // Test random access performance
        let mid_config = storage.get_configuration("scale-test-0500").unwrap();
        assert_eq!(mid_config.alias_name, "scale-test-0500");
        assert_eq!(mid_config.token, "sk-ant-scale-0500");
        
        let duration = start.elapsed();
        
        // Should complete within reasonable time (5 seconds is generous)
        assert!(duration.as_secs() < 5, "Large scale operations should complete within 5 seconds, took {:?}", duration);
    }

    // Environment Variable Generation Tests
    #[test]
    fn test_environment_tuples_consistency() {
        let config = create_full_config(
            "consistency-test",
            "sk-ant-consistency",
            "https://consistency.api.com",
            Some("claude-consistency-model"),
            Some("haiku-consistency-model")
        );
        
        // Generate environment config multiple times
        for _ in 0..100 {
            let env_config = EnvironmentConfig::from_config(&config);
            let env_tuples = env_config.as_env_tuples();
            
            assert_eq!(env_tuples.len(), 4);
            
            // Verify all expected tuples are present every time
            let token_present = env_tuples.iter().any(|(k, v)| k == "ANTHROPIC_AUTH_TOKEN" && v == "sk-ant-consistency");
            let url_present = env_tuples.iter().any(|(k, v)| k == "ANTHROPIC_BASE_URL" && v == "https://consistency.api.com");
            let model_present = env_tuples.iter().any(|(k, v)| k == "ANTHROPIC_MODEL" && v == "claude-consistency-model");
            let small_model_present = env_tuples.iter().any(|(k, v)| k == "ANTHROPIC_SMALL_FAST_MODEL" && v == "haiku-consistency-model");
            
            assert!(token_present, "TOKEN should always be present");
            assert!(url_present, "URL should always be present");
            assert!(model_present, "MODEL should always be present");
            assert!(small_model_present, "SMALL_FAST_MODEL should always be present");
        }
    }

    #[test]
    fn test_partial_model_configuration() {
        // Test config with only main model, no small_fast_model
        let config1 = create_full_config(
            "partial1",
            "sk-ant-partial1",
            "https://partial1.api.com",
            Some("claude-main-only"),
            None
        );
        
        let env_config1 = EnvironmentConfig::from_config(&config1);
        let env_tuples1 = env_config1.as_env_tuples();
        
        assert_eq!(env_tuples1.len(), 3);
        assert!(env_tuples1.iter().any(|(k, _)| k == "ANTHROPIC_MODEL"));
        assert!(!env_tuples1.iter().any(|(k, _)| k == "ANTHROPIC_SMALL_FAST_MODEL"));
        
        // Test config with only small_fast_model, no main model
        let config2 = create_full_config(
            "partial2",
            "sk-ant-partial2",
            "https://partial2.api.com",
            None,
            Some("haiku-small-only")
        );
        
        let env_config2 = EnvironmentConfig::from_config(&config2);
        let env_tuples2 = env_config2.as_env_tuples();
        
        assert_eq!(env_tuples2.len(), 3);
        assert!(!env_tuples2.iter().any(|(k, _)| k == "ANTHROPIC_MODEL"));
        assert!(env_tuples2.iter().any(|(k, _)| k == "ANTHROPIC_SMALL_FAST_MODEL"));
    }

    // Integration Test Between Different Components
    #[test]
    fn test_storage_to_environment_workflow() {
        let storage = create_test_storage_with_configs();
        
        // Test complete workflow: storage -> config -> environment
        let config = storage.get_configuration("test1").unwrap();
        let env_config = EnvironmentConfig::from_config(&config);
        let env_tuples = env_config.as_env_tuples();
        
        assert_eq!(env_tuples.len(), 2);
        
        // Verify the complete chain works correctly
        let expected_token = "sk-ant-test1";
        let expected_url = "https://api1.test.com";
        
        assert!(env_tuples.iter().any(|(k, v)| k == "ANTHROPIC_AUTH_TOKEN" && v == expected_token));
        assert!(env_tuples.iter().any(|(k, v)| k == "ANTHROPIC_BASE_URL" && v == expected_url));
    }

    #[test]
    fn test_multiple_configurations_environment_isolation() {
        let storage = create_test_storage_with_configs();
        
        // Test that different configurations produce different environment configs
        let config1 = storage.get_configuration("test1").unwrap();
        let config2 = storage.get_configuration("test2").unwrap();
        
        let env_config1 = EnvironmentConfig::from_config(&config1);
        let env_config2 = EnvironmentConfig::from_config(&config2);
        
        let env_tuples1 = env_config1.as_env_tuples();
        let env_tuples2 = env_config2.as_env_tuples();
        
        // Should have different tokens and URLs
        let token1 = env_tuples1.iter().find(|(k, _)| k == "ANTHROPIC_AUTH_TOKEN").unwrap().1.clone();
        let token2 = env_tuples2.iter().find(|(k, _)| k == "ANTHROPIC_AUTH_TOKEN").unwrap().1.clone();
        let url1 = env_tuples1.iter().find(|(k, _)| k == "ANTHROPIC_BASE_URL").unwrap().1.clone();
        let url2 = env_tuples2.iter().find(|(k, _)| k == "ANTHROPIC_BASE_URL").unwrap().1.clone();
        
        assert_ne!(token1, token2, "Different configurations should have different tokens");
        assert_ne!(url1, url2, "Different configurations should have different URLs");
        
        assert_eq!(token1, "sk-ant-test1");
        assert_eq!(token2, "sk-ant-test2");
        assert_eq!(url1, "https://api1.test.com");
        assert_eq!(url2, "https://api2.test.com");
    }
}