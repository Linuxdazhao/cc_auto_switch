use crate::cli::completion::{
    generate_completion, list_aliases_for_completion, list_codex_aliases_for_completion,
    list_grok_aliases_for_completion, list_omp_aliases_for_completion,
};
use crate::cli::{Cli, Commands};
use crate::codex::{
    handle_codex_add, handle_codex_interactive, handle_codex_list, handle_codex_remove,
    handle_codex_use,
};
use crate::config::types::{AddCommandParams, ClaudeSettings, StorageMode};
use crate::config::{ConfigStorage, Configuration, EnvironmentConfig, validate_alias_name};
use crate::grok::{
    handle_grok_add, handle_grok_interactive, handle_grok_list, handle_grok_remove, handle_grok_use,
};
use crate::interactive::{
    handle_interactive_selection, launch_claude_with_env, read_input, read_sensitive_input,
};
use crate::omp::{
    handle_omp_add, handle_omp_interactive, handle_omp_list, handle_omp_remove, handle_omp_use,
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

/// Parsed configuration fields from text/file import
#[allow(clippy::type_complexity)]
type ParsedConfigFields = (
    String,         // token
    Option<String>, // api_key
    String,         // url
    Option<String>, // model
    Option<String>, // small_fast_model
    Option<u32>,    // max_thinking_tokens
    Option<u32>,    // api_timeout_ms
    Option<u32>,    // claude_code_disable_nonessential_traffic
    Option<String>, // anthropic_default_sonnet_model
    Option<String>, // anthropic_default_opus_model
    Option<String>, // anthropic_default_haiku_model
    Option<String>, // claude_code_subagent_model
    Option<u32>,    // claude_code_disable_nonstreaming_fallback
    Option<String>, // claude_code_effort_level
    Option<String>, // claude_code_auto_compact_window
    Option<u32>,    // disable_prompt_caching
    Option<u32>,    // claude_code_disable_experimental_betas
    Option<u32>,    // disable_autoupdater
);

/// Extract a string value for a known env var name from arbitrary text.
///
/// Matches patterns: `"NAME": "value"`, `"NAME": value`, `NAME=value`, `NAME: value`
fn extract_str_field(text: &str, name: &str) -> Option<String> {
    let pattern = format!(
        r#"(?m)(?:"{name}"\s*:\s*"([^"]*)"|"{name}"\s*:\s*([^\s,}}\]]+)|(?:export\s+)?{name}\s*=\s*"?([^"\s,}}\]]+)"?|(?:export\s+)?{name}\s*:\s*"?([^"\s,}}\]]+)"?)"#
    );
    let re = regex::Regex::new(&pattern).ok()?;
    re.captures(text).and_then(|caps| {
        caps.get(1)
            .or_else(|| caps.get(2))
            .or_else(|| caps.get(3))
            .or_else(|| caps.get(4))
            .map(|m| m.as_str().to_string())
            .filter(|s| !s.is_empty())
    })
}

/// Extract a u32 value for a known env var name from arbitrary text.
fn extract_u32_field(text: &str, name: &str) -> Option<u32> {
    extract_str_field(text, name).and_then(|s| s.parse::<u32>().ok())
}

/// Parse configuration fields from arbitrary text containing env var definitions.
///
/// Supports JSON (with or without "env" wrapper), shell assignments (KEY=value),
/// and colon-separated (KEY: value) formats.
fn parse_config_from_text(text: &str) -> Result<ParsedConfigFields> {
    let auth_token = extract_str_field(text, "ANTHROPIC_AUTH_TOKEN");
    let api_key = extract_str_field(text, "ANTHROPIC_API_KEY");

    if auth_token.is_some() && api_key.is_some() {
        anyhow::bail!(
            "Input contains both ANTHROPIC_AUTH_TOKEN and ANTHROPIC_API_KEY — only one is allowed"
        );
    }

    let token = match (&auth_token, &api_key) {
        (Some(t), None) => t.clone(),
        (None, Some(_)) => String::new(),
        (None, None) => {
            eprintln!(
                "Warning: No ANTHROPIC_AUTH_TOKEN or ANTHROPIC_API_KEY found in input. Token will be empty — you need to configure it later."
            );
            String::new()
        }
        _ => unreachable!(),
    };

    let url = extract_str_field(text, "ANTHROPIC_BASE_URL").unwrap_or_default();

    Ok((
        token,
        api_key,
        url,
        extract_str_field(text, "ANTHROPIC_MODEL"),
        extract_str_field(text, "ANTHROPIC_SMALL_FAST_MODEL"),
        extract_u32_field(text, "ANTHROPIC_MAX_THINKING_TOKENS"),
        extract_u32_field(text, "API_TIMEOUT_MS"),
        extract_u32_field(text, "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC"),
        extract_str_field(text, "ANTHROPIC_DEFAULT_SONNET_MODEL"),
        extract_str_field(text, "ANTHROPIC_DEFAULT_OPUS_MODEL"),
        extract_str_field(text, "ANTHROPIC_DEFAULT_HAIKU_MODEL"),
        extract_str_field(text, "CLAUDE_CODE_SUBAGENT_MODEL"),
        extract_u32_field(text, "CLAUDE_CODE_DISABLE_NONSTREAMING_FALLBACK"),
        extract_str_field(text, "CLAUDE_CODE_EFFORT_LEVEL"),
        extract_str_field(text, "CLAUDE_CODE_AUTO_COMPACT_WINDOW"),
        extract_u32_field(text, "DISABLE_PROMPT_CACHING"),
        extract_u32_field(text, "CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS"),
        extract_u32_field(text, "DISABLE_AUTOUPDATER"),
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
    // Resolve import source: from_file, from_json, or from_stdin
    let import_text: Option<String> = if let Some(file_path) = &params.from_file {
        println!("Importing configuration from file: {}", file_path);
        Some(
            fs::read_to_string(file_path)
                .map_err(|e| anyhow!("Failed to read file '{}': {}", file_path, e))?,
        )
    } else if let Some(json_text) = &params.from_json {
        println!("Importing configuration from provided text");
        Some(json_text.clone())
    } else if params.from_stdin {
        println!("Importing configuration from stdin");
        let mut buf = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)
            .map_err(|e| anyhow!("Failed to read stdin: {}", e))?;
        Some(buf)
    } else {
        None
    };

    if let Some(text) = &import_text {
        let (
            file_token,
            file_api_key,
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
            file_auto_compact_window,
            file_disable_prompt_caching,
            file_disable_experimental_betas,
            file_disable_autoupdater,
        ) = parse_config_from_text(text)?;

        params.token = Some(file_token);
        params.api_key = file_api_key;
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
        params.claude_code_auto_compact_window = file_auto_compact_window;
        params.disable_prompt_caching = file_disable_prompt_caching;
        params.claude_code_disable_experimental_betas = file_disable_experimental_betas;
        params.disable_autoupdater = file_disable_autoupdater;

        println!("Configuration '{}' will be imported", params.alias_name);
    }

    // Validate alias name
    validate_alias_name(&params.alias_name)?;

    // Check if alias already exists
    if storage.get_configuration(&params.alias_name).is_some() && !params.force {
        eprintln!("Configuration '{}' already exists.", params.alias_name);
        eprintln!("Use --force to overwrite or choose a different alias name.");
        return Ok(());
    }

    // Cannot use interactive mode with import sources
    if params.interactive
        && (params.from_file.is_some() || params.from_json.is_some() || params.from_stdin)
    {
        anyhow::bail!("Cannot use --interactive mode with --from-file/--from-json/--from-stdin");
    }

    // Enforce mutual exclusivity: --token and --api-key cannot both be provided
    if params.token.is_some() && params.api_key.is_some() {
        anyhow::bail!(
            "Cannot use both --token and --api-key. Choose one:\n\
             --token / -t  → sets ANTHROPIC_AUTH_TOKEN\n\
             --api-key / -k → sets ANTHROPIC_API_KEY"
        );
    }

    // Determine authentication value (token or api_key)
    let (final_token, final_api_key): (String, Option<String>) = if params.interactive {
        if params.token.is_some() || params.token_arg.is_some() || params.api_key.is_some() {
            eprintln!(
                "Warning: Token/API key provided via flags/arguments will be ignored in interactive mode"
            );
        }
        let auth_type = read_input(
            "Auth type — (1) ANTHROPIC_AUTH_TOKEN  (2) ANTHROPIC_API_KEY [default: 1]: ",
        )?;
        if auth_type == "2" {
            let key = read_sensitive_input("Enter API key (ANTHROPIC_API_KEY): ")?;
            (String::new(), Some(key))
        } else {
            let token = read_sensitive_input("Enter API token (sk-ant-xxx): ")?;
            (token, None)
        }
    } else if let Some(key) = params.api_key {
        (String::new(), Some(key))
    } else {
        match (&params.token, &params.token_arg) {
            (Some(t), _) => (t.clone(), None),
            (None, Some(t)) => (t.clone(), None),
            (None, None) => {
                anyhow::bail!(
                    "Authentication is required. Use one of:\n\
                     --token / -t  → sets ANTHROPIC_AUTH_TOKEN\n\
                     --api-key / -k → sets ANTHROPIC_API_KEY\n\
                     -i            → interactive mode"
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

    // Determine auto compact window value
    let final_claude_code_auto_compact_window = if params.interactive {
        if params.claude_code_auto_compact_window.is_some() {
            eprintln!(
                "Warning: Auto compact window provided via flags will be ignored in interactive mode"
            );
        }
        let window_input =
            read_input("Enter auto compact window size (optional, press enter to skip): ")?;
        if window_input.is_empty() {
            None
        } else {
            Some(window_input)
        }
    } else {
        params.claude_code_auto_compact_window
    };

    // Determine disable prompt caching flag value
    let final_disable_prompt_caching = if params.interactive {
        if params.disable_prompt_caching.is_some() {
            eprintln!(
                "Warning: Disable prompt caching flag provided via flags will be ignored in interactive mode"
            );
        }
        let flag_input = read_input(
            "Enter disable prompt caching flag (optional, press enter to skip, enter 0 to clear): ",
        )?;
        if flag_input.is_empty() {
            None
        } else if let Ok(flag) = flag_input.parse::<u32>() {
            if flag == 0 { None } else { Some(flag) }
        } else {
            eprintln!("Warning: Invalid disable prompt caching flag value, skipping");
            None
        }
    } else {
        params.disable_prompt_caching
    };

    // Determine disable experimental betas flag value
    let final_claude_code_disable_experimental_betas = if params.interactive {
        if params.claude_code_disable_experimental_betas.is_some() {
            eprintln!(
                "Warning: Disable experimental betas flag provided via flags will be ignored in interactive mode"
            );
        }
        let flag_input = read_input(
            "Enter disable experimental betas flag (optional, press enter to skip, enter 0 to clear): ",
        )?;
        if flag_input.is_empty() {
            None
        } else if let Ok(flag) = flag_input.parse::<u32>() {
            if flag == 0 { None } else { Some(flag) }
        } else {
            eprintln!("Warning: Invalid disable experimental betas flag value, skipping");
            None
        }
    } else {
        params.claude_code_disable_experimental_betas
    };

    // Determine disable auto-updater flag value
    let final_disable_autoupdater = if params.interactive {
        if params.disable_autoupdater.is_some() {
            eprintln!(
                "Warning: Disable auto-updater flag provided via flags will be ignored in interactive mode"
            );
        }
        let flag_input = read_input(
            "Enter disable auto-updater flag (optional, press enter to skip, enter 0 to clear): ",
        )?;
        if flag_input.is_empty() {
            None
        } else if let Ok(flag) = flag_input.parse::<u32>() {
            if flag == 0 { None } else { Some(flag) }
        } else {
            eprintln!("Warning: Invalid disable auto-updater flag value, skipping");
            None
        }
    } else {
        params.disable_autoupdater
    };

    // Validate token format with flexible API provider support (only for AUTH_TOKEN)
    if final_api_key.is_none() {
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
                eprintln!(
                    "Warning: Using official Claude token format with non-official API endpoint"
                );
            }
        }
    }

    // Create and add configuration
    let config = Configuration {
        alias_name: params.alias_name.clone(),
        token: final_token,
        api_key: final_api_key,
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
        claude_code_auto_compact_window: final_claude_code_auto_compact_window,
        disable_prompt_caching: final_disable_prompt_caching,
        claude_code_disable_experimental_betas: final_claude_code_disable_experimental_betas,
        disable_autoupdater: final_disable_autoupdater,
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
    // Handle --list-omp-aliases flag for completion
    if cli.list_omp_aliases {
        list_omp_aliases_for_completion()?;
        return Ok(());
    }
    // Handle --list-grok-aliases flag for completion
    if cli.list_grok_aliases {
        list_grok_aliases_for_completion()?;
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
                api_key,
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
                claude_code_auto_compact_window,
                disable_prompt_caching,
                claude_code_disable_experimental_betas,
                disable_autoupdater,
                force,
                interactive,
                token_arg,
                url_arg,
                from_file,
                from_json,
                from_stdin,
            } => {
                let resolved_from_file: Option<String> = match from_file {
                    Some(Some(path)) => {
                        if !std::path::Path::new(&path).exists() {
                            anyhow::bail!("Config file not found: {}", path);
                        }
                        Some(path)
                    }
                    Some(None) => {
                        let custom_dir = storage.get_claude_settings_dir().map(|s| s.as_str());
                        let default_path = crate::utils::get_claude_settings_path(custom_dir)
                            .map(|p| p.to_string_lossy().into_owned())
                            .map_err(|e| {
                                anyhow!("Failed to resolve default Claude settings path: {}", e)
                            })?;
                        if !std::path::Path::new(&default_path).exists() {
                            anyhow::bail!(
                                "Default Claude config not found at: {}\n\
                                 Run `claude` once to create it, or pass an explicit path: --from-file <path>",
                                default_path
                            );
                        }
                        Some(default_path)
                    }
                    None => None,
                };

                let import_count = resolved_from_file.is_some() as u8
                    + from_json.is_some() as u8
                    + from_stdin as u8;
                if import_count > 1 {
                    anyhow::bail!(
                        "Cannot use more than one of --from-file, --from-json, --from-stdin"
                    );
                }

                let params = AddCommandParams {
                    alias_name,
                    token,
                    api_key,
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
                    claude_code_auto_compact_window,
                    disable_prompt_caching,
                    claude_code_disable_experimental_betas,
                    disable_autoupdater,
                    force,
                    interactive,
                    token_arg,
                    url_arg,
                    from_file: resolved_from_file,
                    from_json,
                    from_stdin,
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
                            let (auth_label, auth_value) = config.auth_env_pair();
                            let mut info =
                                format!("{}={}, url={}", auth_label, auth_value, config.url);
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
                            if let Some(flag) = config.disable_prompt_caching {
                                info.push_str(&format!(", disable_prompt_caching={flag}"));
                            }
                            if let Some(flag) = config.claude_code_disable_experimental_betas {
                                info.push_str(&format!(", disable_experimental_betas={flag}"));
                            }
                            if let Some(flag) = config.disable_autoupdater {
                                info.push_str(&format!(", disable_autoupdater={flag}"));
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
                    use colored::Colorize;
                    println!("{}", "Using official Claude configuration".blue());

                    let mut settings = ClaudeSettings::load(
                        storage.get_claude_settings_dir().map(|s| s.as_str()),
                    )?;
                    settings.remove_anthropic_env();
                    settings.save(storage.get_claude_settings_dir().map(|s| s.as_str()))?;

                    let env = EnvironmentConfig::empty().with_alias("official");
                    launch_claude_with_env(env, None, None, r#continue)?;
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
                let (auth_label, auth_value) = config.auth_env_pair();
                println!(
                    "  {}: {}",
                    auth_label,
                    crate::cli::display_utils::format_token_for_display(auth_value)
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
                    let resolved_from_file: Option<String> = match from_file {
                        Some(Some(path)) => {
                            if !std::path::Path::new(&path).exists() {
                                anyhow::bail!("Codex auth file not found: {}", path);
                            }
                            Some(path)
                        }
                        Some(None) => {
                            let default_path = crate::codex::default_codex_auth_path()
                                .map(|p| p.to_string_lossy().into_owned())
                                .map_err(|e| {
                                    anyhow!("Failed to resolve default Codex auth path: {}", e)
                                })?;
                            if !std::path::Path::new(&default_path).exists() {
                                anyhow::bail!(
                                    "Default Codex auth file not found at: {}\n\
                                     Run `codex login` first, or pass an explicit path: --from-file <path>",
                                    default_path
                                );
                            }
                            Some(default_path)
                        }
                        None => None,
                    };

                    handle_codex_add(
                        alias_name,
                        api_key,
                        force,
                        interactive,
                        resolved_from_file,
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
            Commands::Omp { command } => match command {
                Some(crate::cli::OmpCommands::Add {
                    alias_name,
                    api_key,
                    force,
                    interactive,
                    from_file,
                    base_url,
                    slow_model,
                    smol_model,
                    plan_model,
                }) => {
                    let resolved_from_file: Option<String> = match from_file {
                        Some(Some(path)) => {
                            if !std::path::Path::new(&path).exists() {
                                anyhow::bail!("OMP env file not found: {}", path);
                            }
                            Some(path)
                        }
                        Some(None) => {
                            let default_path = crate::omp::default_omp_env_path()
                                .map(|p| p.to_string_lossy().into_owned())
                                .map_err(|e| {
                                    anyhow!("Failed to resolve default OMP env path: {}", e)
                                })?;
                            if !std::path::Path::new(&default_path).exists() {
                                anyhow::bail!(
                                    "Default OMP env file not found at: {}\n\
                                     Run `omp` once to create it, or pass an explicit path: --from-file <path>",
                                    default_path
                                );
                            }
                            Some(default_path)
                        }
                        None => None,
                    };

                    handle_omp_add(
                        alias_name,
                        api_key,
                        force,
                        interactive,
                        resolved_from_file,
                        base_url,
                        slow_model,
                        smol_model,
                        plan_model,
                        &mut storage,
                    )?;
                }
                Some(crate::cli::OmpCommands::List { plain, name }) => {
                    handle_omp_list(plain, name, &storage)?;
                }
                Some(crate::cli::OmpCommands::Use {
                    alias_name,
                    r#continue,
                    resume,
                    prompt,
                }) => {
                    handle_omp_use(alias_name, r#continue, resume, prompt, &mut storage)?;
                }
                Some(crate::cli::OmpCommands::Remove { alias_names }) => {
                    handle_omp_remove(alias_names, &mut storage)?;
                }
                None => {
                    // Enter interactive mode for OMP configurations
                    handle_omp_interactive(&storage)?;
                }
            },
            Commands::Grok { command } => match command {
                Some(crate::cli::GrokCommands::Add {
                    alias_name,
                    model,
                    base_url,
                    name,
                    api_backend,
                    context_window,
                    api_key,
                    env_key,
                    header,
                    force,
                    interactive,
                    from_file,
                }) => {
                    let resolved_from_file: Option<String> = match from_file {
                        Some(Some(path)) => {
                            if !std::path::Path::new(&path).exists() {
                                anyhow::bail!("grok config file not found: {}", path);
                            }
                            Some(path)
                        }
                        Some(None) => {
                            let default_path = crate::grok::default_grok_config_path()
                                .map(|p| p.to_string_lossy().into_owned())
                                .map_err(|e| {
                                    anyhow!("Failed to resolve default grok config path: {}", e)
                                })?;
                            if !std::path::Path::new(&default_path).exists() {
                                anyhow::bail!(
                                    "Default grok config file not found at: {}\n\
                                     Run `grok` once to create it, or pass an explicit path: --from-file <path>",
                                    default_path
                                );
                            }
                            Some(default_path)
                        }
                        None => None,
                    };

                    handle_grok_add(
                        alias_name,
                        model,
                        base_url,
                        name,
                        api_backend,
                        context_window,
                        api_key,
                        env_key,
                        header,
                        force,
                        interactive,
                        resolved_from_file,
                        &mut storage,
                    )?;
                }
                Some(crate::cli::GrokCommands::List { plain, name }) => {
                    handle_grok_list(plain, name, &storage)?;
                }
                Some(crate::cli::GrokCommands::Use { alias_name, prompt }) => {
                    handle_grok_use(alias_name, prompt, &mut storage)?;
                }
                Some(crate::cli::GrokCommands::Remove { alias_names }) => {
                    handle_grok_remove(alias_names, &mut storage)?;
                }
                None => {
                    // Enter interactive mode for grok configurations
                    handle_grok_interactive(&storage)?;
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
