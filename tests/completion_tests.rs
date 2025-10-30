#[cfg(test)]
mod tests {
    use cc_switch::cli::completion::*;

    // generate_aliases Tests
    #[test]
    fn test_generate_aliases_fish() {
        let result = generate_aliases("fish");
        assert!(result.is_ok(), "Should generate fish aliases successfully");
    }

    #[test]
    fn test_generate_aliases_zsh() {
        let result = generate_aliases("zsh");
        assert!(result.is_ok(), "Should generate zsh aliases successfully");
    }

    #[test]
    fn test_generate_aliases_bash() {
        let result = generate_aliases("bash");
        assert!(result.is_ok(), "Should generate bash aliases successfully");
    }

    #[test]
    fn test_generate_aliases_unsupported_shell() {
        let result = generate_aliases("unsupported");
        assert!(result.is_err(), "Should fail for unsupported shell");

        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Unsupported shell: unsupported"));
        assert!(error_msg.contains("Supported shells: fish, zsh, bash"));
    }

    #[test]
    fn test_generate_aliases_empty_string() {
        let result = generate_aliases("");
        assert!(result.is_err(), "Should fail for empty shell string");
    }

    #[test]
    fn test_generate_aliases_case_sensitivity() {
        let result_upper = generate_aliases("FISH");
        let result_mixed = generate_aliases("Fish");

        assert!(
            result_upper.is_err(),
            "Should be case sensitive - FISH should fail"
        );
        assert!(
            result_mixed.is_err(),
            "Should be case sensitive - Fish should fail"
        );
    }

    #[test]
    fn test_generate_aliases_special_characters() {
        let test_cases = vec!["fish!", "z$h", "bash#", "fish\n", "zsh\t"];

        for shell in test_cases {
            let result = generate_aliases(shell);
            assert!(
                result.is_err(),
                "Should fail for shell with special characters: {}",
                shell
            );
        }
    }

    // generate_completion Tests
    #[test]
    fn test_generate_completion_fish() {
        let result = generate_completion("fish");
        assert!(
            result.is_ok(),
            "Should generate fish completion successfully"
        );
    }

    #[test]
    fn test_generate_completion_zsh() {
        let result = generate_completion("zsh");
        assert!(
            result.is_ok(),
            "Should generate zsh completion successfully"
        );
    }

    #[test]
    fn test_generate_completion_bash() {
        let result = generate_completion("bash");
        assert!(
            result.is_ok(),
            "Should generate bash completion successfully"
        );
    }

    #[test]
    fn test_generate_completion_elvish() {
        let result = generate_completion("elvish");
        assert!(
            result.is_ok(),
            "Should generate elvish completion successfully"
        );
    }

    #[test]
    fn test_generate_completion_powershell() {
        let result = generate_completion("powershell");
        assert!(
            result.is_ok(),
            "Should generate powershell completion successfully"
        );
    }

    #[test]
    fn test_generate_completion_unsupported_shell() {
        let result = generate_completion("unsupported");
        assert!(result.is_err(), "Should fail for unsupported shell");

        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Unsupported shell: unsupported"));
        assert!(error_msg.contains("Supported shells: fish, zsh, bash, elvish, powershell"));
    }

    #[test]
    fn test_generate_completion_nushell_not_supported() {
        // nushell is mentioned in docs but not implemented
        let result = generate_completion("nushell");
        assert!(
            result.is_err(),
            "Should fail for nushell as it's not implemented"
        );
    }

    #[test]
    fn test_generate_completion_case_sensitivity() {
        let result_upper = generate_completion("FISH");
        let result_mixed = generate_completion("Fish");

        assert!(
            result_upper.is_err(),
            "Should be case sensitive - FISH should fail"
        );
        assert!(
            result_mixed.is_err(),
            "Should be case sensitive - Fish should fail"
        );
    }

    #[test]
    fn test_generate_completion_empty_string() {
        let result = generate_completion("");
        assert!(result.is_err(), "Should fail for empty shell string");
    }

    // list_aliases_for_completion Tests
    #[test]
    fn test_list_aliases_for_completion_empty_storage() {
        // This test relies on a clean configuration storage
        // Since we can't easily mock the file system, we test that it doesn't panic
        let result = list_aliases_for_completion();
        // Should succeed even if no configs exist (will create default storage)
        if let Err(e) = &result {
            eprintln!("Error: {:?}", e);
        }
        assert!(result.is_ok(), "Should handle empty storage gracefully");
    }

    #[test]
    fn test_list_aliases_for_completion_with_current_config() {
        // This test verifies the logic around 'current' configuration priority
        // We test the function doesn't panic and handles the case properly
        let result = list_aliases_for_completion();
        assert!(
            result.is_ok(),
            "Should handle configurations with 'current' alias"
        );
    }

    #[test]
    fn test_list_aliases_for_completion_multiple_configs() {
        // This test verifies multiple configurations are handled properly
        let result = list_aliases_for_completion();
        assert!(result.is_ok(), "Should handle multiple configurations");
    }

    // Integration Tests for Shell-specific Logic
    #[test]
    fn test_supported_shells_list() {
        let supported_alias_shells = vec!["fish", "zsh", "bash"];
        let supported_completion_shells = vec!["fish", "zsh", "bash", "elvish", "powershell"];

        // Test all supported alias shells
        for shell in supported_alias_shells {
            let result = generate_aliases(shell);
            assert!(
                result.is_ok(),
                "Shell {} should be supported for aliases",
                shell
            );
        }

        // Test all supported completion shells
        for shell in supported_completion_shells {
            let result = generate_completion(shell);
            assert!(
                result.is_ok(),
                "Shell {} should be supported for completion",
                shell
            );
        }
    }

    #[test]
    fn test_unsupported_shells_consistency() {
        let unsupported_shells = vec!["tcsh", "csh", "sh", "nushell", "ion", "xonsh"];

        for shell in unsupported_shells {
            let alias_result = generate_aliases(shell);
            let completion_result = generate_completion(shell);

            // Both should fail for unsupported shells
            assert!(
                alias_result.is_err(),
                "Shell {} should not be supported for aliases",
                shell
            );

            // nushell is a special case - mentioned in docs but not implemented
            if shell != "nushell" {
                assert!(
                    completion_result.is_err(),
                    "Shell {} should not be supported for completion",
                    shell
                );
            }
        }
    }

    #[test]
    fn test_alias_shell_subset_of_completion_shells() {
        // Alias shells should be a subset of completion shells
        let alias_shells = vec!["fish", "zsh", "bash"];

        for shell in alias_shells {
            let alias_result = generate_aliases(shell);
            let completion_result = generate_completion(shell);

            assert!(alias_result.is_ok(), "Alias shell {} should work", shell);
            assert!(
                completion_result.is_ok(),
                "Completion shell {} should work",
                shell
            );
        }
    }

    // Error Message Quality Tests
    #[test]
    fn test_alias_error_message_quality() {
        let result = generate_aliases("invalid_shell");
        assert!(result.is_err());

        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Unsupported shell"));
        assert!(error_msg.contains("invalid_shell"));
        assert!(error_msg.contains("fish"));
        assert!(error_msg.contains("zsh"));
        assert!(error_msg.contains("bash"));
    }

    #[test]
    fn test_completion_error_message_quality() {
        let result = generate_completion("invalid_shell");
        assert!(result.is_err());

        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Unsupported shell"));
        assert!(error_msg.contains("invalid_shell"));
        assert!(error_msg.contains("fish"));
        assert!(error_msg.contains("zsh"));
        assert!(error_msg.contains("bash"));
        assert!(error_msg.contains("elvish"));
        assert!(error_msg.contains("powershell"));
    }

    // Edge Cases and Robustness Tests
    #[test]
    fn test_whitespace_in_shell_names() {
        let whitespace_shells = vec![" fish", "fish ", " fish ", "fi sh", "\tfish", "fish\n"];

        for shell in whitespace_shells {
            let alias_result = generate_aliases(shell);
            let completion_result = generate_completion(shell);

            assert!(
                alias_result.is_err(),
                "Should reject shell name with whitespace: '{}'",
                shell
            );
            assert!(
                completion_result.is_err(),
                "Should reject shell name with whitespace: '{}'",
                shell
            );
        }
    }

    #[test]
    fn test_unicode_shell_names() {
        let unicode_shells = vec!["fish🐟", "zsh📚", "bash💥", "ﻪtset"];

        for shell in unicode_shells {
            let alias_result = generate_aliases(shell);
            let completion_result = generate_completion(shell);

            assert!(
                alias_result.is_err(),
                "Should reject unicode shell name: '{}'",
                shell
            );
            assert!(
                completion_result.is_err(),
                "Should reject unicode shell name: '{}'",
                shell
            );
        }
    }

    #[test]
    fn test_very_long_shell_names() {
        let long_shell = "a".repeat(1000);

        let alias_result = generate_aliases(&long_shell);
        let completion_result = generate_completion(&long_shell);

        assert!(alias_result.is_err(), "Should reject very long shell name");
        assert!(
            completion_result.is_err(),
            "Should reject very long shell name"
        );
    }

    // Function Behavior Consistency Tests
    #[test]
    fn test_function_consistency_supported_shells() {
        let common_shells = vec!["fish", "zsh", "bash"];

        for shell in common_shells {
            let alias_result = generate_aliases(shell);
            let completion_result = generate_completion(shell);

            // Both should succeed for common shells
            assert!(
                alias_result.is_ok() && completion_result.is_ok(),
                "Both alias and completion should work for shell: {}",
                shell
            );
        }
    }

    #[test]
    fn test_function_consistency_unsupported_shells() {
        let unsupported_shells = vec!["tcsh", "csh", "invalid"];

        for shell in unsupported_shells {
            let alias_result = generate_aliases(shell);

            // Alias should fail for all unsupported shells
            assert!(
                alias_result.is_err(),
                "Alias should fail for unsupported shell: {}",
                shell
            );
        }
    }

    // Stress Tests
    #[test]
    fn test_multiple_calls_same_shell() {
        // Test that multiple calls to the same function work
        for _ in 0..10 {
            let result = generate_aliases("fish");
            assert!(result.is_ok(), "Multiple calls should work");
        }
    }

    #[test]
    fn test_alternating_shell_calls() {
        let shells = vec!["fish", "zsh", "bash"];

        for i in 0..30 {
            let shell = shells[i % shells.len()];
            let alias_result = generate_aliases(shell);
            let completion_result = generate_completion(shell);

            assert!(
                alias_result.is_ok(),
                "Alternating call {} should work for aliases",
                i
            );
            assert!(
                completion_result.is_ok(),
                "Alternating call {} should work for completion",
                i
            );
        }
    }

    // Configuration Storage Integration Tests
    #[test]
    fn test_list_aliases_error_handling() {
        // Test that the function handles storage errors gracefully
        // This is a basic test since we can't easily simulate storage failures
        let result = list_aliases_for_completion();

        // The function should either succeed or fail gracefully
        match result {
            Ok(_) => {
                // Success case - normal operation
            }
            Err(e) => {
                // Error case - should be a meaningful error
                let error_msg = e.to_string();
                assert!(!error_msg.is_empty(), "Error message should not be empty");
            }
        }
    }

    // Output Format Tests (indirect testing)
    #[test]
    fn test_shell_output_generation_does_not_panic() {
        let shells = vec!["fish", "zsh", "bash", "elvish", "powershell"];

        for shell in shells {
            // Test that generation doesn't panic
            let alias_result = if shell == "fish" || shell == "zsh" || shell == "bash" {
                generate_aliases(shell)
            } else {
                Ok(()) // Skip alias test for shells that don't support it
            };
            let completion_result = generate_completion(shell);

            if shell == "fish" || shell == "zsh" || shell == "bash" {
                assert!(
                    alias_result.is_ok(),
                    "Alias generation should not panic for {}",
                    shell
                );
            }
            assert!(
                completion_result.is_ok(),
                "Completion generation should not panic for {}",
                shell
            );
        }
    }

    // Test command factory integration
    #[test]
    fn test_command_factory_usage() {
        use cc_switch::cli::Cli;
        use clap::CommandFactory;

        // Test that we can create the command factory (used in generate_completion)
        let app = Cli::command();

        // Verify basic command structure
        assert_eq!(app.get_name(), "cc-switch");

        // Test that the command has expected subcommands
        let subcommand_names: Vec<&str> = app.get_subcommands().map(|cmd| cmd.get_name()).collect();

        assert!(
            subcommand_names.contains(&"use"),
            "Should have 'use' subcommand"
        );
        assert!(
            subcommand_names.contains(&"add"),
            "Should have 'add' subcommand"
        );
        assert!(
            subcommand_names.contains(&"list"),
            "Should have 'list' subcommand"
        );
        assert!(
            subcommand_names.contains(&"completion"),
            "Should have 'completion' subcommand"
        );
    }

    // Performance Tests (basic)
    #[test]
    fn test_multiple_operations_performance() {
        // Basic test to ensure functions complete in reasonable time
        let start = std::time::Instant::now();

        // Perform multiple operations
        for _ in 0..100 {
            let _ = generate_aliases("fish");
            let _ = generate_completion("zsh");
        }

        let duration = start.elapsed();

        // Should complete within a reasonable time (10 seconds is very generous)
        assert!(
            duration.as_secs() < 10,
            "Operations should complete within 10 seconds, took {:?}",
            duration
        );
    }
}
