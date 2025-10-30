use crate::cli::completion::{generate_aliases, generate_completion, list_aliases_for_completion};
use crate::cli::{Cli, Commands};
use crate::config::types::AddCommandParams;
use crate::config::{ConfigStorage, Configuration, EnvironmentConfig, validate_alias_name};
use crate::interactive::{
    handle_current_command, handle_interactive_selection, read_input, read_sensitive_input,
};
use anyhow::{Result, anyhow};
use clap::Parser;
use colored::*;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

/// Parse a configuration from a JSON file
///
/// # Arguments
/// * `file_path` - Path to the JSON configuration file
///
/// # Returns
/// Result containing a tuple of (alias_name, token, url, model, small_fast_model, max_thinking_tokens, api_timeout_ms, claude_code_disable_nonessential_traffic, anthropic_default_sonnet_model, anthropic_default_opus_model, anthropic_default_haiku_model)
///
/// # Errors
/// Returns error if file cannot be read or parsed
#[allow(clippy::type_complexity)]
fn parse_config_from_file(
    file_path: &str,
) -> Result<(
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<u32>,
    Option<u32>,
    Option<u32>,
    Option<String>,
    Option<String>,
    Option<String>,
)> {
    // Read the file
    let file_content = fs::read_to_string(file_path)
        .map_err(|e| anyhow!("Failed to read file '{}': {}", file_path, e))?;

    // Parse JSON
    let json: serde_json::Value = serde_json::from_str(&file_content)
        .map_err(|e| anyhow!("Failed to parse JSON from file '{}': {}", file_path, e))?;

    // Extract env section
    let env = json.get("env").and_then(|v| v.as_object()).ok_or_else(|| {
        anyhow!(
            "File '{}' does not contain a valid 'env' section",
            file_path
        )
    })?;

    // Extract alias name from filename (without extension)
    let path = Path::new(file_path);
    let alias_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("Invalid file path: {}", file_path))?
        .to_string();

    // Extract and map environment variables
    let token = env
        .get("ANTHROPIC_AUTH_TOKEN")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Missing ANTHROPIC_AUTH_TOKEN in file '{}'", file_path))?
        .to_string();

    let url = env
        .get("ANTHROPIC_BASE_URL")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Missing ANTHROPIC_BASE_URL in file '{}'", file_path))?
        .to_string();

    let model = env
        .get("ANTHROPIC_MODEL")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let small_fast_model = env
        .get("ANTHROPIC_SMALL_FAST_MODEL")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let max_thinking_tokens = env
        .get("ANTHROPIC_MAX_THINKING_TOKENS")
        .and_then(|v| v.as_u64())
        .map(|u| u as u32);

    let api_timeout_ms = env
        .get("API_TIMEOUT_MS")
        .and_then(|v| v.as_u64())
        .map(|u| u as u32);

    let claude_code_disable_nonessential_traffic = env
        .get("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC")
        .and_then(|v| v.as_u64())
        .map(|u| u as u32);

    let anthropic_default_sonnet_model = env
        .get("ANTHROPIC_DEFAULT_SONNET_MODEL")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let anthropic_default_opus_model = env
        .get("ANTHROPIC_DEFAULT_OPUS_MODEL")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let anthropic_default_haiku_model = env
        .get("ANTHROPIC_DEFAULT_HAIKU_MODEL")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok((
        alias_name,
        token,
        url,
        model,
        small_fast_model,
        max_thinking_tokens,
        api_timeout_ms,
        claude_code_disable_nonessential_traffic,
        anthropic_default_sonnet_model,
        anthropic_default_opus_model,
        anthropic_default_haiku_model,
    ))
}

/// Handle adding a configuration with all the new features
///
/// # Arguments
/// * `params` - Parameters for the add command
/// * `storage` - Mutable reference to config storage
///
/// # Errors
/// Returns error if validation fails or user cancels interactive input
fn handle_add_command(mut params: AddCommandParams, storage: &mut ConfigStorage) -> Result<()> {
    // If from-file is provided, parse the file and use those values
    if let Some(file_path) = &params.from_file {
        println!("Importing configuration from file: {}", file_path);

        let (
            file_alias_name,
            file_token,
            file_url,
            file_model,
            file_small_fast_model,
            file_max_thinking_tokens,
            file_api_timeout_ms,
            file_claude_disable_nonessential_traffic,
            file_sonnet_model,
            file_opus_model,
            file_haiku_model,
        ) = parse_config_from_file(file_path)?;

        // Use the file's alias name (ignoring the one provided via command line)
        params.alias_name = file_alias_name;

        // Override params with file values
        params.token = Some(file_token);
        params.url = Some(file_url);
        params.model = file_model;
        params.small_fast_model = file_small_fast_model;
        params.max_thinking_tokens = file_max_thinking_tokens;
        params.api_timeout_ms = file_api_timeout_ms;
        params.claude_code_disable_nonessential_traffic = file_claude_disable_nonessential_traffic;
        params.anthropic_default_sonnet_model = file_sonnet_model;
        params.anthropic_default_opus_model = file_opus_model;
        params.anthropic_default_haiku_model = file_haiku_model;

        println!(
            "Configuration '{}' will be imported from file",
            params.alias_name
        );
    }

    // Validate alias name
    validate_alias_name(&params.alias_name)?;

    // Check if alias already exists
    if storage.get_configuration(&params.alias_name).is_some() && !params.force {
        eprintln!("Configuration '{}' already exists.", params.alias_name);
        eprintln!("Use --force to overwrite or choose a different alias name.");
        return Ok(());
    }

    // Cannot use interactive mode with --from-file
    if params.interactive && params.from_file.is_some() {
        anyhow::bail!("Cannot use --interactive mode with --from-file");
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
            if tokens == 0 { None } else { Some(tokens) }
        } else {
            eprintln!("Warning: Invalid max thinking tokens value, skipping");
            None
        }
    } else {
        params.max_thinking_tokens
    };

    // Determine API timeout value
    let final_api_timeout_ms = if params.interactive {
        if params.api_timeout_ms.is_some() {
            eprintln!(
                "Warning: API timeout provided via flags will be ignored in interactive mode"
            );
        }
        let timeout_input = read_input(
            "Enter API timeout in milliseconds (optional, press enter to skip, enter 0 to clear): ",
        )?;
        if timeout_input.is_empty() {
            None
        } else if let Ok(timeout) = timeout_input.parse::<u32>() {
            if timeout == 0 { None } else { Some(timeout) }
        } else {
            eprintln!("Warning: Invalid API timeout value, skipping");
            None
        }
    } else {
        params.api_timeout_ms
    };

    // Determine disable nonessential traffic flag value
    let final_claude_code_disable_nonessential_traffic = if params.interactive {
        if params.claude_code_disable_nonessential_traffic.is_some() {
            eprintln!(
                "Warning: Disable nonessential traffic flag provided via flags will be ignored in interactive mode"
            );
        }
        let flag_input = read_input(
            "Enter disable nonessential traffic flag (optional, press enter to skip, enter 0 to clear): ",
        )?;
        if flag_input.is_empty() {
            None
        } else if let Ok(flag) = flag_input.parse::<u32>() {
            if flag == 0 { None } else { Some(flag) }
        } else {
            eprintln!("Warning: Invalid disable nonessential traffic flag value, skipping");
            None
        }
    } else {
        params.claude_code_disable_nonessential_traffic
    };

    // Determine default Sonnet model value
    let final_anthropic_default_sonnet_model = if params.interactive {
        if params.anthropic_default_sonnet_model.is_some() {
            eprintln!(
                "Warning: Default Sonnet model provided via flags will be ignored in interactive mode"
            );
        }
        let model_input =
            read_input("Enter default Sonnet model name (optional, press enter to skip): ")?;
        if model_input.is_empty() {
            None
        } else {
            Some(model_input)
        }
    } else {
        params.anthropic_default_sonnet_model
    };

    // Determine default Opus model value
    let final_anthropic_default_opus_model = if params.interactive {
        if params.anthropic_default_opus_model.is_some() {
            eprintln!(
                "Warning: Default Opus model provided via flags will be ignored in interactive mode"
            );
        }
        let model_input =
            read_input("Enter default Opus model name (optional, press enter to skip): ")?;
        if model_input.is_empty() {
            None
        } else {
            Some(model_input)
        }
    } else {
        params.anthropic_default_opus_model
    };

    // Determine default Haiku model value
    let final_anthropic_default_haiku_model = if params.interactive {
        if params.anthropic_default_haiku_model.is_some() {
            eprintln!(
                "Warning: Default Haiku model provided via flags will be ignored in interactive mode"
            );
        }
        let model_input =
            read_input("Enter default Haiku model name (optional, press enter to skip): ")?;
        if model_input.is_empty() {
            None
        } else {
            Some(model_input)
        }
    } else {
        params.anthropic_default_haiku_model
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
        api_timeout_ms: final_api_timeout_ms,
        claude_code_disable_nonessential_traffic: final_claude_code_disable_nonessential_traffic,
        anthropic_default_sonnet_model: final_anthropic_default_sonnet_model,
        anthropic_default_opus_model: final_anthropic_default_opus_model,
        anthropic_default_haiku_model: final_anthropic_default_haiku_model,
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
        if let Some(api_timeout_ms) = config.api_timeout_ms {
            config_info.push_str(&format!(", api_timeout_ms: {api_timeout_ms}"));
        }
        if let Some(claude_code_disable_nonessential_traffic) =
            config.claude_code_disable_nonessential_traffic
        {
            config_info.push_str(&format!(
                ", disable_nonessential_traffic: {claude_code_disable_nonessential_traffic}"
            ));
        }
        if let Some(sonnet_model) = &config.anthropic_default_sonnet_model {
            config_info.push_str(&format!(", default_sonnet_model: {sonnet_model}"));
        }
        if let Some(opus_model) = &config.anthropic_default_opus_model {
            config_info.push_str(&format!(", default_opus_model: {opus_model}"));
        }
        if let Some(haiku_model) = &config.anthropic_default_haiku_model {
            config_info.push_str(&format!(", default_haiku_model: {haiku_model}"));
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

    // Wait 0.2 second
    println!("Waiting 0.2 second before launching Claude...");
    println!(
        "Executing: claude {}",
        "--dangerously-skip-permissions".red()
    );
    thread::sleep(Duration::from_millis(200));

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
                api_timeout_ms,
                claude_code_disable_nonessential_traffic,
                anthropic_default_sonnet_model,
                anthropic_default_opus_model,
                anthropic_default_haiku_model,
                force,
                interactive,
                token_arg,
                url_arg,
                from_file,
            } => {
                // When from_file is provided, alias_name will be extracted from the file
                // For other cases, use the provided alias_name or provide a default
                let final_alias_name = if from_file.is_some() {
                    // Will be set from file parsing, use a placeholder for now
                    "placeholder".to_string()
                } else {
                    alias_name.unwrap_or_else(|| {
                        eprintln!("Error: alias_name is required when not using --from-file");
                        std::process::exit(1);
                    })
                };

                let params = AddCommandParams {
                    alias_name: final_alias_name,
                    token,
                    url,
                    model,
                    small_fast_model,
                    max_thinking_tokens,
                    api_timeout_ms,
                    claude_code_disable_nonessential_traffic,
                    anthropic_default_sonnet_model,
                    anthropic_default_opus_model,
                    anthropic_default_haiku_model,
                    force,
                    interactive,
                    token_arg,
                    url_arg,
                    from_file,
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
            Commands::List { plain } => {
                if plain {
                    // Text output when -p flag is used
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
                                info.push_str(&format!(
                                    ", max_thinking_tokens={max_thinking_tokens}"
                                ));
                            }
                            println!("  {alias_name}: {info}");
                        }
                    }
                } else {
                    // JSON output (default)
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&storage.configurations)
                            .map_err(|e| anyhow!("Failed to serialize configurations: {}", e))?
                    );
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
