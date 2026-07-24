use crate::config::{ConfigStorage, validate_alias_name};
use crate::grok::GrokConfiguration;
use crate::grok::config_writer::{parse_grok_config_file, write_grok_config};
use crate::grok::types::HeaderMap;
use crate::platform::resolve_npm_cli;
use anyhow::{Result, anyhow};
use std::collections::BTreeMap;
use std::process::Command;

/// Parse `KEY=VALUE` header strings into a map.
fn parse_headers(headers: Vec<String>) -> Result<Option<HeaderMap>> {
    if headers.is_empty() {
        return Ok(None);
    }
    let mut map = BTreeMap::new();
    for h in headers {
        let (k, v) = h
            .split_once('=')
            .ok_or_else(|| anyhow!("Invalid header '{}'. Expected KEY=VALUE.", h))?;
        map.insert(k.trim().to_string(), v.trim().to_string());
    }
    Ok(Some(map))
}

/// Add a grok configuration.
#[allow(clippy::too_many_arguments)]
pub fn handle_grok_add(
    alias_name: String,
    model: Option<String>,
    base_url: Option<String>,
    name: Option<String>,
    api_backend: Option<String>,
    context_window: Option<u32>,
    api_key: Option<String>,
    env_key: Vec<String>,
    headers: Vec<String>,
    force: bool,
    interactive: bool,
    from_file: Option<String>,
    storage: &mut ConfigStorage,
) -> Result<()> {
    validate_alias_name(&alias_name)?;

    if storage.get_grok_configuration(&alias_name).is_some() {
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
        parse_grok_config_file(&file_path, &alias_name)?
    } else if interactive {
        parse_interactive_grok_config(&alias_name)?
    } else {
        let model = model.ok_or_else(|| {
            anyhow!(
                "--model is required. Use --model <id>, --from-file [<path>], or -i for interactive mode."
            )
        })?;
        GrokConfiguration {
            alias_name: alias_name.clone(),
            model,
            base_url,
            name,
            api_backend,
            context_window,
            api_key,
            env_key: if env_key.is_empty() {
                None
            } else {
                Some(env_key)
            },
            extra_headers: parse_headers(headers)?,
        }
    };

    storage.add_grok_configuration(config);
    storage.save()?;
    println!("Configuration '{}' added successfully.", alias_name);
    Ok(())
}

/// Interactive mode for creating a grok configuration.
fn parse_interactive_grok_config(alias_name: &str) -> Result<GrokConfiguration> {
    use crate::interactive::read_input;

    let model = read_input("Model id (e.g. claude-opus-4-6): ")?;
    if model.is_empty() {
        anyhow::bail!("Model id is required.");
    }
    let base_url = read_input("Base URL (press Enter to skip): ")?;
    let name = read_input("Display name (press Enter to skip): ")?;
    let api_backend =
        read_input("API backend (chat_completions/responses/messages) [chat_completions]: ")?;
    let context_window = read_input("Context window (press Enter to skip): ")?;
    let env_key = read_input("Env var name for key (e.g. XAI_API_KEY, press Enter to skip): ")?;
    let api_key = read_input("API key written to disk (press Enter to skip): ")?;

    let opt = |s: String| if s.is_empty() { None } else { Some(s) };

    Ok(GrokConfiguration {
        alias_name: alias_name.to_string(),
        model,
        base_url: opt(base_url),
        name: opt(name),
        api_backend: if api_backend.is_empty() {
            Some("chat_completions".to_string())
        } else {
            Some(api_backend)
        },
        context_window: if context_window.is_empty() {
            None
        } else {
            context_window.parse::<u32>().ok()
        },
        api_key: opt(api_key),
        env_key: if env_key.is_empty() {
            None
        } else {
            Some(vec![env_key])
        },
        extra_headers: None,
    })
}

/// List grok configurations.
pub fn handle_grok_list(plain: bool, name: bool, storage: &ConfigStorage) -> Result<()> {
    let configs = storage.grok_configurations.as_ref();

    if configs.is_none() || configs.as_ref().map(|m| m.is_empty()).unwrap_or(true) {
        println!("No grok configurations found.");
        return Ok(());
    }

    let mut configs: Vec<&GrokConfiguration> = configs.unwrap().values().collect();
    configs.sort_by(|a, b| a.alias_name.cmp(&b.alias_name));

    if name {
        if plain {
            for config in configs {
                println!("{:<20} {}", config.alias_name, config.model);
            }
        } else {
            let json_array: Vec<_> = configs
                .iter()
                .map(|c| serde_json::json!({ "alias_name": c.alias_name, "model": c.model }))
                .collect();
            println!(
                "{}",
                serde_json::to_string_pretty(&json_array).expect("Should serialize to JSON")
            );
        }
        return Ok(());
    }

    if plain {
        println!("{:<20} {:<24} Auth", "Name", "Model");
        println!("{}", "-".repeat(60));
        for config in &configs {
            let auth = auth_display(config);
            println!("{:<20} {:<24} {}", config.alias_name, config.model, auth);
            if let Some(url) = &config.base_url {
                println!("  base_url: {}", url);
            }
        }
    } else {
        let mut json_array = Vec::new();
        for config in &configs {
            let mut obj = serde_json::json!({
                "alias_name": config.alias_name,
                "model": config.model,
            });
            if let Some(v) = &config.base_url {
                obj["base_url"] = serde_json::json!(v);
            }
            if let Some(v) = &config.name {
                obj["name"] = serde_json::json!(v);
            }
            if let Some(v) = &config.api_backend {
                obj["api_backend"] = serde_json::json!(v);
            }
            if let Some(v) = config.context_window {
                obj["context_window"] = serde_json::json!(v);
            }
            if let Some(v) = &config.env_key {
                obj["env_key"] = serde_json::json!(v);
            }
            if let Some(v) = &config.api_key {
                obj["api_key"] = serde_json::json!(mask_key(v));
            }
            if let Some(h) = &config.extra_headers {
                let masked: BTreeMap<String, String> =
                    h.iter().map(|(k, v)| (k.clone(), mask_key(v))).collect();
                obj["extra_headers"] = serde_json::json!(masked);
            }
            json_array.push(obj);
        }
        println!(
            "{}",
            serde_json::to_string_pretty(&json_array).expect("Should serialize to JSON")
        );
    }

    Ok(())
}

fn auth_display(config: &GrokConfiguration) -> String {
    if let Some(keys) = &config.env_key
        && !keys.is_empty()
    {
        return format!("env:{}", keys.join(","));
    }
    if let Some(key) = &config.api_key {
        return mask_key(key);
    }
    if config.extra_headers.is_some() {
        return "headers".to_string();
    }
    "(none)".to_string()
}

/// Switch to a grok configuration and launch the grok CLI.
pub fn handle_grok_use(
    alias_name: String,
    prompt: Vec<String>,
    storage: &mut ConfigStorage,
) -> Result<()> {
    let config = storage
        .get_grok_configuration(&alias_name)
        .ok_or_else(|| anyhow!("grok configuration '{}' not found.", alias_name))?
        .clone();

    write_grok_config(&config)?;
    println!(
        "Switched to grok configuration '{}' (model: {}).",
        config.alias_name, config.model
    );

    launch_grok(prompt)?;
    Ok(())
}

/// Launch the grok CLI, forwarding any trailing arguments verbatim.
fn launch_grok(prompt: Vec<String>) -> Result<()> {
    let mut cmd = Command::new(resolve_npm_cli("grok"));
    if !prompt.is_empty() {
        cmd.args(prompt);
    }

    match cmd.status() {
        Ok(status) => {
            if !status.success() {
                return Err(anyhow!("grok exited with status: {}", status));
            }
        }
        Err(e) => {
            return Err(anyhow!(
                "Failed to launch grok: {}. Make sure grok is installed and available in your PATH.",
                e
            ));
        }
    }
    Ok(())
}

/// Remove grok configurations.
pub fn handle_grok_remove(alias_names: Vec<String>, storage: &mut ConfigStorage) -> Result<()> {
    let mut not_found = Vec::new();

    for alias in &alias_names {
        if storage.remove_grok_configuration(alias) {
            println!("grok configuration '{}' removed successfully", alias);
        } else {
            not_found.push(alias.clone());
        }
    }

    storage.save()?;

    for alias in &not_found {
        println!("grok configuration '{}' not found, skipping", alias);
    }

    Ok(())
}

/// Enter interactive mode for grok configuration selection.
pub fn handle_grok_interactive(storage: &ConfigStorage) -> Result<()> {
    crate::interactive::handle_grok_interactive_selection(storage)
}

fn mask_key(key: &str) -> String {
    let len = key.len();
    if len <= 8 {
        return key[..len / 2].to_string() + "****";
    }
    format!("{}****{}", &key[..4], &key[len - 4..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_headers_ok() {
        let map = parse_headers(vec![
            "x-api-key=sk-ant-xxx".to_string(),
            "anthropic-version=2023-06-01".to_string(),
        ])
        .unwrap()
        .unwrap();
        assert_eq!(map.get("x-api-key").unwrap(), "sk-ant-xxx");
        assert_eq!(map.get("anthropic-version").unwrap(), "2023-06-01");
    }

    #[test]
    fn test_parse_headers_empty() {
        assert!(parse_headers(vec![]).unwrap().is_none());
    }

    #[test]
    fn test_parse_headers_invalid() {
        assert!(parse_headers(vec!["nokeyvalue".to_string()]).is_err());
    }

    #[test]
    fn test_mask_key() {
        assert_eq!(mask_key("abcdef"), "abc****");
        assert_eq!(mask_key("sk-ant-abcdef1234567890"), "sk-a****7890");
    }
}
