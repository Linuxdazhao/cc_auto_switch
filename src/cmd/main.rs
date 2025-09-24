use crate::cmd::cli::{Cli, Commands};
use crate::cmd::completion::{generate_aliases, generate_completion, list_aliases_for_completion};
use crate::cmd::config::{ConfigStorage, Configuration, EnvironmentConfig, validate_alias_name};
use crate::cmd::interactive::{
    handle_current_command, handle_interactive_selection, read_input, read_sensitive_input,
};
use anyhow::Result;
use clap::Parser;
use colored::*;
use std::process::Command;
use std::thread;
use std::time::Duration;

/// Parameters for adding a new configuration
pub struct AddCommandParams {
    pub alias_name: String,
    pub token: Option<String>,
    pub url: Option<String>,
    pub model: Option<String>,
    pub small_fast_model: Option<String>,
    pub max_thinking_tokens: Option<u32>,
    pub force: bool,
    pub interactive: bool,
    pub token_arg: Option<String>,
    pub url_arg: Option<String>,
}

/// Handle adding a configuration with all the new features
///
/// # Arguments
/// * `params` - Parameters for the add command
/// * `storage` - Mutable reference to config storage
///
/// # Errors
/// Returns error if validation fails or user cancels interactive input
fn handle_add_command(params: AddCommandParams, storage: &mut ConfigStorage) -> Result<()> {
    // Validate alias name
    validate_alias_name(&params.alias_name)?;

    // Check if alias already exists
    if storage.get_configuration(&params.alias_name).is_some() && !params.force {
        eprintln!("Configuration '{}' already exists.", params.alias_name);
        eprintln!("Use --force to overwrite or choose a different alias name.");
        return Ok(());
    }

    // Determine token value
    let final_token = if params.interactive {
        if params.token.is_some() || params.token_arg.is_some() {
            eprintln!(
                "Warning: Token provided via flags/arguments will be ignored in interactive mode"
            );
        }
        read_sensitive_input("Enter API token (sk-ant-xxx): ")?
    } else {
        match (&params.token, &params.token_arg) {
            (Some(t), _) => t.clone(),
            (None, Some(t)) => t.clone(),
            (None, None) => {
                anyhow::bail!(
                    "Token is required. Use -t flag, provide as argument, or use interactive mode with -i"
                );
            }
        }
    };

    // Determine URL value
    let final_url = if params.interactive {
        if params.url.is_some() || params.url_arg.is_some() {
            eprintln!(
                "Warning: URL provided via flags/arguments will be ignored in interactive mode"
            );
        }
        read_input("Enter API URL (default: https://api.anthropic.com): ")?
    } else {
        match (&params.url, &params.url_arg) {
            (Some(u), _) => u.clone(),
            (None, Some(u)) => u.clone(),
            (None, None) => "https://api.anthropic.com".to_string(),
        }
    };

    // Use default URL if empty
    let final_url = if final_url.is_empty() {
        "https://api.anthropic.com".to_string()
    } else {
        final_url
    };

    // Determine model value
    let final_model = if params.interactive {
        if params.model.is_some() {
            eprintln!("Warning: Model provided via flags will be ignored in interactive mode");
        }
        let model_input = read_input("Enter model name (optional, press enter to skip): ")?;
        if model_input.is_empty() {
            None
        } else {
            Some(model_input)
        }
    } else {
        params.model
    };

    // Determine small fast model value
    let final_small_fast_model = if params.interactive {
        if params.small_fast_model.is_some() {
            eprintln!(
                "Warning: Small fast model provided via flags will be ignored in interactive mode"
            );
        }
        let small_model_input =
            read_input("Enter small fast model name (optional, press enter to skip): ")?;
        if small_model_input.is_empty() {
            None
        } else {
            Some(small_model_input)
        }
    } else {
        params.small_fast_model
    };

    // Determine max thinking tokens value
    let final_max_thinking_tokens = if params.interactive {
        if params.max_thinking_tokens.is_some() {
            eprintln!(
                "Warning: Max thinking tokens provided via flags will be ignored in interactive mode"
            );
        }
        let tokens_input = read_input(
            "Enter maximum thinking tokens (optional, press enter to skip, enter 0 to clear): ",
        )?;
        if tokens_input.is_empty() {
            None
        } else if let Ok(tokens) = tokens_input.parse::<u32>() {
            if tokens == 0 {
                None
            } else {
                Some(tokens)
            }
        } else {
            eprintln!("Warning: Invalid max thinking tokens value, skipping");
            None
        }
    } else {
        params.max_thinking_tokens
    };

    // Validate token format with flexible API provider support
    let is_anthropic_official = final_url.contains("api.anthropic.com");
    if is_anthropic_official {
        if !final_token.starts_with("sk-ant-api03-") {
            eprintln!(
                "Warning: For official Anthropic API (api.anthropic.com), token should start with 'sk-ant-api03-'"
            );
        }
    } else {
        // For non-official APIs, provide general guidance
        if final_token.starts_with("sk-ant-api03-") {
            eprintln!("Warning: Using official Claude token format with non-official API endpoint");
        }
        // Don't validate format for third-party APIs as they may use different formats
    }

    // Create and add configuration
    let config = Configuration {
        alias_name: params.alias_name.clone(),
        token: final_token,
        url: final_url,
        model: final_model,
        small_fast_model: final_small_fast_model,
        max_thinking_tokens: final_max_thinking_tokens,
    };

    storage.add_configuration(config);
    storage.save()?;

    println!("Configuration '{}' added successfully", params.alias_name);
    if params.force {
        println!("(Overwrote existing configuration)");
    }

    Ok(())
}

/// Handle configuration switching command
///
/// Processes the switch subcommand to switch Claude API configuration:
/// - None: Enter interactive selection mode  
/// - "cc": Launch Claude without custom environment variables (default behavior)
/// - Other alias: Launch Claude with the specified configuration's environment variables
///
/// After switching, displays the current URL and automatically launches Claude CLI
///
/// # Arguments
/// * `alias_name` - Optional name of configuration to switch to, or "cc" for default
///
/// # Errors
/// Returns error if configuration is not found or Claude CLI fails to launch
pub fn handle_switch_command(alias_name: Option<&str>) -> Result<()> {
    let storage = ConfigStorage::load()?;

    // If no alias provided, enter interactive mode
    if alias_name.is_none() {
        return handle_interactive_selection(&storage);
    }

    let alias_name = alias_name.unwrap();

    let env_config = if alias_name == "cc" {
        // Default operation: launch Claude without custom environment variables
        println!("Using default Claude configuration (no custom API settings)");
        println!("Current URL: Default (api.anthropic.com)");
        EnvironmentConfig::empty()
    } else if let Some(config) = storage.get_configuration(alias_name) {
        let mut config_info = format!("token: {}, url: {}", config.token, config.url);
        if let Some(model) = &config.model {
            config_info.push_str(&format!(", model: {model}"));
        }
        if let Some(small_fast_model) = &config.small_fast_model {
            config_info.push_str(&format!(", small_fast_model: {small_fast_model}"));
        }
        if let Some(max_thinking_tokens) = config.max_thinking_tokens {
            config_info.push_str(&format!(", max_thinking_tokens: {max_thinking_tokens}"));
        }
        println!("Switched to configuration '{alias_name}' ({config_info})");
        println!("Current URL: {}", config.url);
        EnvironmentConfig::from_config(config)
    } else {
        anyhow::bail!(
            "Configuration '{}' not found. Use 'list' command to see available configurations.",
            alias_name
        );
    };

    // Wait 0.5 second
    println!("Waiting 0.5 second before launching Claude...");
    println!(
        "Executing: claude {}",
        "--dangerously-skip-permissions".red()
    );
    thread::sleep(Duration::from_millis(500));

    // Set environment variables for current process
    for (key, value) in env_config.as_env_tuples() {
        unsafe {
            std::env::set_var(&key, &value);
        }
    }

    // Launch Claude CLI with exec to replace current process
    println!("Launching Claude CLI...");

    // On Unix systems, use exec to replace current process
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let error = Command::new("claude")
            .arg("--dangerously-skip-permissions")
            .exec();
        // exec never returns on success, so if we get here, it failed
        anyhow::bail!("Failed to exec claude: {}", error);
    }

    // On non-Unix systems, fallback to spawn and wait
    #[cfg(not(unix))]
    {
        use std::process::Stdio;
        let mut child = Command::new("claude")
            .arg("--dangerously-skip-permissions")
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .context(
                "Failed to launch Claude CLI. Make sure 'claude' command is available in PATH",
            )?;

        // Wait for the Claude process to finish and pass control to it
        let status = child
            .wait()
            .context("Failed to wait for Claude CLI process")?;

        if !status.success() {
            anyhow::bail!("Claude CLI exited with error status: {}", status);
        }
    }
}

/// Main entry point for the CLI application
///
/// Parses command-line arguments and executes the appropriate action:
/// - Switch configuration with `-c` flag
/// - Execute subcommands (add, remove, list)
/// - Show help if no arguments provided
///
/// # Errors
/// Returns error if any operation fails (file I/O, parsing, etc.)
pub fn run() -> Result<()> {
    let cli = Cli::parse();

    // Handle --list-aliases flag for completion
    if cli.list_aliases {
        list_aliases_for_completion()?;
        return Ok(());
    }

    // Handle subcommands
    if let Some(command) = cli.command {
        let mut storage = ConfigStorage::load()?;

        match command {
            Commands::Add {
                alias_name,
                token,
                url,
                model,
                small_fast_model,
                max_thinking_tokens,
                force,
                interactive,
                token_arg,
                url_arg,
            } => {
                let params = AddCommandParams {
                    alias_name,
                    token,
                    url,
                    model,
                    small_fast_model,
                    max_thinking_tokens,
                    force,
                    interactive,
                    token_arg,
                    url_arg,
                };
                handle_add_command(params, &mut storage)?;
            }
            Commands::Remove { alias_names } => {
                let mut removed_count = 0;
                let mut not_found_aliases = Vec::new();

                for alias_name in &alias_names {
                    if storage.remove_configuration(alias_name) {
                        removed_count += 1;
                        println!("Configuration '{alias_name}' removed successfully");
                    } else {
                        not_found_aliases.push(alias_name.clone());
                        println!("Configuration '{alias_name}' not found");
                    }
                }

                if removed_count > 0 {
                    storage.save()?;
                }

                if !not_found_aliases.is_empty() {
                    eprintln!(
                        "Warning: The following configurations were not found: {}",
                        not_found_aliases.join(", ")
                    );
                }

                if removed_count > 0 {
                    println!("Successfully removed {removed_count} configuration(s)");
                }
            }
            Commands::List => {
                if storage.configurations.is_empty() {
                    println!("No configurations stored");
                } else {
                    println!("Stored configurations:");
                    for (alias_name, config) in &storage.configurations {
                        let mut info = format!("token={}, url={}", config.token, config.url);
                        if let Some(model) = &config.model {
                            info.push_str(&format!(", model={model}"));
                        }
                        if let Some(small_fast_model) = &config.small_fast_model {
                            info.push_str(&format!(", small_fast_model={small_fast_model}"));
                        }
                        if let Some(max_thinking_tokens) = config.max_thinking_tokens {
                            info.push_str(&format!(", max_thinking_tokens={max_thinking_tokens}"));
                        }
                        println!("  {alias_name}: {info}");
                    }
                }
            }
            Commands::Completion { shell } => {
                generate_completion(&shell)?;
            }
            Commands::Alias { shell } => {
                generate_aliases(&shell)?;
            }
            Commands::Use { alias_name } => {
                handle_switch_command(Some(&alias_name))?;
            }
            Commands::Current => {
                handle_current_command()?;
            }
            Commands::Version => {
                println!("{}", env!("CARGO_PKG_VERSION"));
            }
        }
    } else {
        // No command provided, show interactive configuration selection
        let storage = ConfigStorage::load()?;
        handle_interactive_selection(&storage)?;
    }

    Ok(())
}
