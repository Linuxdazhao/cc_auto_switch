use crate::cli::completion::{
    generate_completion, list_aliases_for_completion, list_codex_aliases_for_completion,
};
use crate::cli::{Cli, Commands};
use crate::codex::{
    handle_codex_add, handle_codex_interactive, handle_codex_list, handle_codex_remove,
    handle_codex_use,
};
use crate::config::types::{AddCommandParams, ClaudeSettings, StorageMode};
use crate::config::{ConfigStorage, Configuration, EnvironmentConfig, validate_alias_name};
use crate::interactive::{
    handle_interactive_selection, launch_claude_with_env, read_input, read_sensitive_input,
};
use anyhow::{Result, anyhow};
use clap::Parser;
use std::fs;

/// Parse storage mode string to StorageMode enum
///
/// # Arguments
/// * `store_str` - String representation of storage mode ("env" or "config")
///
/// # Returns
/// Result containing StorageMode or error if invalid
fn parse_storage_mode(store_str: &str) -> Result<StorageMode> {
    match store_str.to_lowercase().as_str() {
        "env" => Ok(StorageMode::Env),
        "config" => Ok(StorageMode::Config),
        _ => Err(anyhow!(
            "Invalid storage mode '{}'. Use 'env' or 'config'",
            store_str
        )),
    }
}

/// Parse a configuration from a JSON file
///
/// # Arguments
/// * `file_path` - Path to the JSON configuration file
///
/// # Returns
/// Result containing a tuple of configuration values (token, url, and optional fields)
///
/// # Errors
/// Returns error if file cannot be read or parsed
#[allow(clippy::type_complexity)]
fn parse_config_from_file(
    file_path: &str,
) -> Result<(
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
    Option<String>,
    Option<u32>,
    Option<String>,
)> {
    let file_content = fs::read_to_string(file_path)
        .map_err(|e| anyhow!("Failed to read file '{}': {}", file_path, e))?;

    let json: serde_json::Value = serde_json::from_str(&file_content)
        .map_err(|e| anyhow!("Failed to parse JSON from file '{}': {}", file_path, e))?;

    let env = json.get("env").and_then(|v| v.as_object()).ok_or_else(|| {
        anyhow!(
            "File '{}' does not contain a valid 'env' section",
            file_path
        )
    })?;

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

    let claude_code_subagent_model = env
        .get("CLAUDE_CODE_SUBAGENT_MODEL")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let claude_code_disable_nonstreaming_fallback = env
        .get("CLAUDE_CODE_DISABLE_NONSTREAMING_FALLBACK")
        .and_then(|v| v.as_u64())
        .map(|u| u as u32);

    let claude_code_effort_level = env
        .get("CLAUDE_CODE_EFFORT_LEVEL")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok((
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
        claude_code_subagent_model,
        claude_code_disable_nonstreaming_fallback,
        claude_code_effort_level,
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
        if !std::path::Path::new(file_path).exists() {
            anyhow::bail!(
                "Config file not found: {}\n\
                 If you intended to import from Claude's default config, run `claude` once to create it.\n\
                 Otherwise pass an explicit path: --from-file <path>",
                file_path
            );
        }
        println!("Importing configuration from file: {}", file_path);

        let (
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
            file_subagent_model,
            file_disable_nonstreaming_fallback,
            file_effort_level,
        ) = parse_config_from_file(file_path)?;

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
        params.claude_code_subagent_model = file_subagent_model;
        params.claude_code_disable_nonstreaming_fallback = file_disable_nonstreaming_fallback;
        params.claude_code_effort_level = file_effort_level;

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

    // Determine subagent model value
    let final_claude_code_subagent_model = if params.interactive {
        if params.claude_code_subagent_model.is_some() {
            eprintln!(
                "Warning: Subagent model provided via flags will be ignored in interactive mode"
            );
        }
        let model_input =
            read_input("Enter subagent model name (optional, press enter to skip): ")?;
        if model_input.is_empty() {
            None
        } else {
            Some(model_input)
        }
    } else {
        params.claude_code_subagent_model
    };

    // Determine disable non-streaming fallback flag value
    let final_claude_code_disable_nonstreaming_fallback = if params.interactive {
        if params.claude_code_disable_nonstreaming_fallback.is_some() {
            eprintln!(
                "Warning: Disable non-streaming fallback flag provided via flags will be ignored in interactive mode"
            );
        }
        let flag_input = read_input(
            "Enter disable non-streaming fallback flag (optional, press enter to skip, enter 0 to clear): ",
        )?;
        if flag_input.is_empty() {
            None
        } else if let Ok(flag) = flag_input.parse::<u32>() {
            if flag == 0 { None } else { Some(flag) }
        } else {
            eprintln!("Warning: Invalid disable non-streaming fallback flag value, skipping");
            None
        }
    } else {
        params.claude_code_disable_nonstreaming_fallback
    };

    // Determine effort level value
    let final_claude_code_effort_level = if params.interactive {
        if params.claude_code_effort_level.is_some() {
            eprintln!(
                "Warning: Effort level provided via flags will be ignored in interactive mode"
            );
        }
        let level_input = read_input("Enter effort level (optional, press enter to skip): ")?;
        if level_input.is_empty() {
            None
        } else {
            Some(level_input)
        }
    } else {
        params.claude_code_effort_level
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
        claude_code_subagent_model: final_claude_code_subagent_model,
        claude_code_disable_nonstreaming_fallback: final_claude_code_disable_nonstreaming_fallback,
        claude_code_effort_level: final_claude_code_effort_level,
        claude_code_experimental_agent_teams: None,
        claude_code_disable_1m_context: None,
    };

    storage.add_configuration(config);
    storage.save()?;

    println!("Configuration '{}' added successfully", params.alias_name);
    if params.force {
        println!("(Overwrote existing configuration)");
    }

    Ok(())
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

    // Handle --migrate flag: migrate old path to new path and exit
    if cli.migrate {
        ConfigStorage::migrate_from_old_path()?;
        return Ok(());
    }

    // Handle --list-aliases flag for completion
    if cli.list_aliases {
        list_aliases_for_completion()?;
        return Ok(());
    }

    // Handle --list-codex-aliases flag for completion
    if cli.list_codex_aliases {
        list_codex_aliases_for_completion()?;
        return Ok(());
    }

    // Reap per-PID alias files for terminated sessions on every invocation.
    // Skipped for completion-only paths above to keep shell completion fast.
    let _ = ClaudeSettings::cleanup_orphan_alias_files();

    // Handle --store flag: set default storage mode and exit
    if let Some(ref store_str) = cli.store
        && cli.command.is_none()
    {
        // No command provided, so --store is a setter
        let mode = match parse_storage_mode(store_str) {
            Ok(mode) => mode,
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        };

        let mut storage = ConfigStorage::load()?;
        storage.default_storage_mode = Some(mode.clone());
        storage.save()?;

        let mode_str = match mode {
            StorageMode::Env => "env",
            StorageMode::Config => "config",
        };

        println!("Default storage mode set to: {}", mode_str);
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
                claude_code_subagent_model,
                claude_code_disable_nonstreaming_fallback,
                claude_code_effort_level,
                force,
                interactive,
                token_arg,
                url_arg,
                from_file,
            } => {
                let resolved_from_file: Option<String> = match from_file {
                    Some(Some(path)) => Some(path),
                    Some(None) => {
                        let custom_dir = storage.get_claude_settings_dir().map(|s| s.as_str());
                        Some(
                            crate::utils::get_claude_settings_path(custom_dir)
                                .map(|p| p.to_string_lossy().into_owned())
                                .map_err(|e| {
                                    anyhow!("Failed to resolve default Claude settings path: {}", e)
                                })?,
                        )
                    }
                    None => None,
                };

                let params = AddCommandParams {
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
                    claude_code_subagent_model,
                    claude_code_disable_nonstreaming_fallback,
                    claude_code_effort_level,
                    force,
                    interactive,
                    token_arg,
                    url_arg,
                    from_file: resolved_from_file,
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
            Commands::List { plain, name } => {
                if name {
                    if storage.configurations.is_empty() {
                        println!("No configurations stored");
                    } else {
                        for (alias_name, config) in &storage.configurations {
                            println!("{}: {}", alias_name, config.url);
                        }
                    }
                } else if plain {
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
                            if let Some(subagent_model) = &config.claude_code_subagent_model {
                                info.push_str(&format!(", subagent_model={subagent_model}"));
                            }
                            if let Some(flag) = config.claude_code_disable_nonstreaming_fallback {
                                info.push_str(&format!(", disable_nonstreaming_fallback={flag}"));
                            }
                            if let Some(effort_level) = &config.claude_code_effort_level {
                                info.push_str(&format!(", effort_level={effort_level}"));
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
            Commands::Use {
                alias_name,
                resume,
                r#continue,
                prompt,
            } => {
                // Handle special reset aliases
                if alias_name == "cc" || alias_name == "official" {
                    println!("Using official Claude configuration");

                    let mut settings = ClaudeSettings::load(
                        storage.get_claude_settings_dir().map(|s| s.as_str()),
                    )?;
                    settings.remove_anthropic_env();
                    settings.save(storage.get_claude_settings_dir().map(|s| s.as_str()))?;

                    launch_claude_with_env(
                        EnvironmentConfig::empty().with_alias("official"),
                        None,
                        None,
                        r#continue,
                    )?;
                    return Ok(());
                }

                let config = storage
                    .configurations
                    .get(&alias_name)
                    .ok_or_else(|| anyhow!("Configuration '{}' not found", alias_name))?
                    .clone();

                let env_config = EnvironmentConfig::from_config(&config).with_alias(&alias_name);
                let storage_mode = storage.default_storage_mode.clone().unwrap_or_default();

                // Update settings.json with the configuration
                let mut settings =
                    ClaudeSettings::load(storage.get_claude_settings_dir().map(|s| s.as_str()))?;
                settings.switch_to_config_with_mode(
                    &config,
                    storage_mode,
                    storage.get_claude_settings_dir().map(|s| s.as_str()),
                )?;

                println!("Switched to configuration '{}'", alias_name);
                println!("  URL:   {}", config.url);
                println!(
                    "  Token: {}",
                    crate::cli::display_utils::format_token_for_display(&config.token)
                );

                let prompt_str = if prompt.is_empty() {
                    None
                } else {
                    Some(prompt.join(" "))
                };

                launch_claude_with_env(
                    env_config,
                    prompt_str.as_deref(),
                    resume.as_deref(),
                    r#continue,
                )?;
            }
            Commands::Codex { command } => match command {
                Some(crate::cli::CodexCommands::Add {
                    alias_name,
                    api_key,
                    force,
                    interactive,
                    from_file,
                }) => {
                    handle_codex_add(
                        alias_name,
                        api_key,
                        force,
                        interactive,
                        from_file,
                        &mut storage,
                    )?;
                }
                Some(crate::cli::CodexCommands::List { plain, name }) => {
                    handle_codex_list(plain, name, &storage)?;
                }
                Some(crate::cli::CodexCommands::Use {
                    alias_name,
                    r#continue,
                    resume,
                    prompt,
                }) => {
                    handle_codex_use(alias_name, r#continue, resume, prompt, &mut storage)?;
                }
                Some(crate::cli::CodexCommands::Remove { alias_names }) => {
                    handle_codex_remove(alias_names, &mut storage)?;
                }
                None => {
                    // Enter interactive mode for Codex configurations
                    handle_codex_interactive(&storage)?;
                }
            },
            Commands::Statusline { action } => {
                let custom_dir = storage.get_claude_settings_dir().map(|s| s.as_str());
                match action {
                    crate::cli::StatuslineAction::Install => {
                        crate::statusline::install(custom_dir)?;
                    }
                    crate::cli::StatuslineAction::Uninstall => {
                        crate::statusline::uninstall(custom_dir)?;
                    }
                }
            }
        }
    } else {
        // No command provided, show interactive configuration selection
        let storage = ConfigStorage::load()?;
        handle_interactive_selection(&storage)?;
    }

    Ok(())
}
