use crate::codex::{CodexConfiguration, write_auth_json};
use crate::config::{ConfigStorage, validate_alias_name};
use crate::interactive::read_input;
use anyhow::{Result, anyhow};
use std::fs;
use std::process::Command;

/// Add a Codex configuration
pub fn handle_codex_add(
    alias_name: String,
    api_key: Option<String>,
    force: bool,
    interactive: bool,
    from_file: Option<String>,
    storage: &mut ConfigStorage,
) -> Result<()> {
    validate_alias_name(&alias_name)?;

    if storage.get_codex_configuration(&alias_name).is_some() {
        if !force {
            eprintln!(
                "Warning: Configuration '{}' already exists. Use --force to overwrite.",
                alias_name
            );
            return Ok(());
        }
        eprintln!("Overwriting existing configuration '{}'", alias_name);
    }

    let config = if let Some(file_path) = from_file {
        parse_auth_json_file(&file_path, &alias_name)?
    } else if interactive {
        parse_interactive_codex_config(&alias_name)?
    } else {
        let key = api_key.ok_or_else(|| {
            anyhow!(
                "API key is required. Use -k <key>, --from-file <path>, or -i for interactive mode."
            )
        })?;
        CodexConfiguration {
            alias_name: alias_name.clone(),
            auth_mode: "apikey".to_string(),
            openai_api_key: Some(key),
            id_token: None,
            access_token: None,
            refresh_token: None,
            account_id: None,
            last_refresh: None,
        }
    };

    storage.add_codex_configuration(config);
    storage.save()?;
    println!("Configuration '{}' added successfully.", alias_name);
    Ok(())
}

/// Parse an existing auth.json file into a CodexConfiguration
pub fn parse_auth_json_file(file_path: &str, alias_name: &str) -> Result<CodexConfiguration> {
    let content = fs::read_to_string(file_path)
        .map_err(|e| anyhow!("Failed to read auth.json file '{}': {}", file_path, e))?;

    let json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| anyhow!("Failed to parse auth.json file '{}': {}", file_path, e))?;

    let auth_mode = json["auth_mode"]
        .as_str()
        .ok_or_else(|| {
            anyhow!(
                "Missing 'auth_mode' field in auth.json file '{}'",
                file_path
            )
        })?
        .to_string();

    let openai_api_key = json["OPENAI_API_KEY"].as_str().map(|s| s.to_string());

    let tokens = &json["tokens"];
    let id_token = tokens["id_token"].as_str().map(|s| s.to_string());
    let access_token = tokens["access_token"].as_str().map(|s| s.to_string());
    let refresh_token = tokens["refresh_token"].as_str().map(|s| s.to_string());
    let account_id = tokens["account_id"].as_str().map(|s| s.to_string());
    let last_refresh = json["last_refresh"].as_str().map(|s| s.to_string());

    Ok(CodexConfiguration {
        alias_name: alias_name.to_string(),
        auth_mode,
        openai_api_key,
        id_token,
        access_token,
        refresh_token,
        account_id,
        last_refresh,
    })
}

/// Interactive mode for creating a Codex configuration
fn parse_interactive_codex_config(alias_name: &str) -> Result<CodexConfiguration> {
    use crate::interactive::{read_input, read_sensitive_input};

    let mode_input = read_input("Auth mode (chatgpt/apikey) [chatgpt]: ")?;
    let auth_mode = if mode_input.is_empty() {
        "chatgpt".to_string()
    } else {
        mode_input.to_lowercase()
    };

    match auth_mode.as_str() {
        "chatgpt" => {
            let id_token = read_sensitive_input("ID Token: ")?;
            let access_token = read_sensitive_input("Access Token: ")?;
            let refresh_token = read_sensitive_input("Refresh Token: ")?;
            let account_id = read_input("Account ID: ")?;

            Ok(CodexConfiguration {
                alias_name: alias_name.to_string(),
                auth_mode: "chatgpt".to_string(),
                openai_api_key: None,
                id_token: if id_token.is_empty() {
                    None
                } else {
                    Some(id_token)
                },
                access_token: if access_token.is_empty() {
                    None
                } else {
                    Some(access_token)
                },
                refresh_token: if refresh_token.is_empty() {
                    None
                } else {
                    Some(refresh_token)
                },
                account_id: if account_id.is_empty() {
                    None
                } else {
                    Some(account_id)
                },
                last_refresh: None,
            })
        }
        "apikey" => {
            let api_key = read_sensitive_input("OpenAI API Key: ")?;
            if api_key.is_empty() {
                return Err(anyhow!("API key cannot be empty"));
            }
            Ok(CodexConfiguration {
                alias_name: alias_name.to_string(),
                auth_mode: "apikey".to_string(),
                openai_api_key: Some(api_key),
                id_token: None,
                access_token: None,
                refresh_token: None,
                account_id: None,
                last_refresh: None,
            })
        }
        _ => Err(anyhow!(
            "Invalid auth mode '{}'. Use 'chatgpt' or 'apikey'.",
            auth_mode
        )),
    }
}

/// List Codex configurations
pub fn handle_codex_list(plain: bool, storage: &ConfigStorage) -> Result<()> {
    let configs = storage.codex_configurations.as_ref();

    if configs.is_none() || configs.unwrap().is_empty() {
        println!("No Codex configurations found.");
        return Ok(());
    }

    if plain {
        for (alias, config) in configs.unwrap() {
            println!("{}", alias);
            println!("  Auth Mode: {}", config.auth_mode);
            if let Some(ref key) = config.openai_api_key {
                let truncated = if key.len() > 8 {
                    format!("{}...", &key[..8])
                } else {
                    key.clone()
                };
                println!("  API Key: {}", truncated);
            }
            if let Some(ref token) = config.id_token {
                let truncated = if token.len() > 8 {
                    format!("{}...", &token[..8])
                } else {
                    token.clone()
                };
                println!("  ID Token: {}", truncated);
            }
            if let Some(ref id) = config.account_id {
                println!("  Account ID: {}", id);
            }
        }
    } else {
        let json = serde_json::to_string_pretty(configs.unwrap())
            .map_err(|e| anyhow!("Failed to serialize configurations: {}", e))?;
        println!("{}", json);
    }
    Ok(())
}

/// Switch to a Codex configuration and launch Codex CLI
pub fn handle_codex_use(
    alias_name: String,
    continue_flag: bool,
    resume: Option<String>,
    prompt: Vec<String>,
    storage: &mut ConfigStorage,
) -> Result<()> {
    let config = storage
        .get_codex_configuration(&alias_name)
        .ok_or_else(|| anyhow!("Codex configuration '{}' not found.", alias_name))?
        .clone();

    write_auth_json(&config)?;
    println!("Switched to Codex configuration '{}'", alias_name);

    launch_codex(continue_flag, resume, prompt)?;
    Ok(())
}

/// Launch Codex CLI with optional arguments
fn launch_codex(continue_flag: bool, resume: Option<String>, prompt: Vec<String>) -> Result<()> {
    let mut cmd = Command::new("codex");

    if continue_flag {
        cmd.arg("--continue");
    }

    if let Some(ref session_id) = resume {
        cmd.arg("--resume").arg(session_id);
    }

    if !prompt.is_empty() {
        cmd.args(prompt);
    }

    let status = cmd
        .status()
        .map_err(|e| anyhow!("Failed to launch Codex: {}", e))?;

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}

/// Remove Codex configurations
pub fn handle_codex_remove(alias_names: Vec<String>, storage: &mut ConfigStorage) -> Result<()> {
    let mut removed_count = 0;
    let mut not_found_aliases = Vec::new();

    for alias in &alias_names {
        if storage.remove_codex_configuration(alias) {
            removed_count += 1;
            println!("Codex configuration '{}' removed successfully", alias);
        } else {
            not_found_aliases.push(alias.clone());
            println!("Codex configuration '{}' not found", alias);
        }
    }

    if removed_count > 0 {
        storage.save()?;
    }

    if !not_found_aliases.is_empty() {
        eprintln!(
            "Warning: The following Codex configurations were not found: {}",
            not_found_aliases.join(", ")
        );
    }

    if removed_count > 0 {
        println!("Successfully removed {removed_count} Codex configuration(s)");
    }

    Ok(())
}

/// Enter interactive mode for Codex configuration selection
pub fn handle_codex_interactive(storage: &ConfigStorage) -> Result<()> {
    let configs = match &storage.codex_configurations {
        Some(configs) if !configs.is_empty() => configs,
        _ => {
            println!("No Codex configurations available. Use 'cc-switch codex add' to create configurations first.");
            return Ok(());
        }
    };

    // Collect and sort configurations
    let mut sorted_configs: Vec<_> = configs.values().collect();
    sorted_configs.sort_by(|a, b| a.alias_name.cmp(&b.alias_name));

    println!("Available Codex configurations:\n");

    for (i, config) in sorted_configs.iter().enumerate() {
        let auth_info = if config.auth_mode == "apikey" {
            if let Some(ref key) = config.openai_api_key {
                format!("apikey: {}...", &key[..8.min(key.len())])
            } else {
                "apikey: <none>".to_string()
            }
        } else {
            format!("chatgpt (account: {})",
                config.account_id.as_deref().unwrap_or("unknown"))
        };

        println!("  {}. {} - {}", i + 1, config.alias_name, auth_info);
    }

    println!("\nSelect a configuration (1-{}), or 'q' to quit: ", sorted_configs.len());

    let input = read_input("")?;
    let input = input.trim();

    if input.eq_ignore_ascii_case("q") || input.is_empty() {
        println!("Selection cancelled");
        return Ok(());
    }

    match input.parse::<usize>() {
        Ok(n) if n >= 1 && n <= sorted_configs.len() => {
            let config = sorted_configs[n - 1];
            println!("\nSwitching to Codex configuration: {}", config.alias_name);

            // Write auth.json
            write_auth_json(config)?;

            println!("  Auth mode: {}", config.auth_mode);
            if let Some(ref account_id) = config.account_id {
                println!("  Account ID: {}", account_id);
            }

            // Launch codex
            launch_codex(false, None, Vec::new())?;
        }
        _ => {
            println!("Invalid selection");
        }
    }

    Ok(())
}
