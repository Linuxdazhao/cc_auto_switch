use crate::config::{ConfigStorage, validate_alias_name};
use crate::omp::OmpConfiguration;
use crate::omp::env_writer::write_env_file;
use crate::platform::resolve_npm_cli;
use anyhow::{Result, anyhow};
use std::process::Command;

/// Add an OMP configuration
#[allow(clippy::too_many_arguments)]
pub fn handle_omp_add(
    alias_name: String,
    api_key: Option<String>,
    force: bool,
    interactive: bool,
    from_file: Option<String>,
    base_url: Option<String>,
    slow_model: Option<String>,
    smol_model: Option<String>,
    plan_model: Option<String>,
    storage: &mut ConfigStorage,
) -> Result<()> {
    validate_alias_name(&alias_name)?;

    if storage.get_omp_configuration(&alias_name).is_some() {
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
        parse_omp_env_file(&file_path, &alias_name)?
    } else if interactive {
        parse_interactive_omp_config(&alias_name)?
    } else {
        let key = api_key.ok_or_else(|| {
            anyhow!(
                "API key is required. Use --api-key <key>, --from-file [<path>], or -i for interactive mode."
            )
        })?;

        OmpConfiguration {
            alias_name: alias_name.clone(),
            openai_api_key: Some(key),
            base_url,
            slow_model,
            smol_model,
            plan_model,
        }
    };

    storage.add_omp_configuration(config.clone());
    storage.save()?;
    crate::omp::env_writer::update_models_yml(&config)?;
    println!("Configuration '{}' added successfully.", alias_name);
    Ok(())
}

/// Parse an existing .env file into an OmpConfiguration
pub fn parse_omp_env_file(file_path: &str, alias_name: &str) -> Result<OmpConfiguration> {
    let content = std::fs::read_to_string(file_path)
        .map_err(|e| anyhow!("Failed to read .env file '{}': {}", file_path, e))?;

    let mut openai_api_key = None;
    let mut base_url = None;
    let mut slow_model = None;
    let mut smol_model = None;
    let mut plan_model = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once('=') {
            let key = key.trim();
            let value = value.trim().trim_matches('"').trim_matches('\'');
            match key {
                "OPENAI_API_KEY" => openai_api_key = Some(value.to_string()),
                // Accept either the OpenAI or legacy Anthropic base-url var on import
                "OPENAI_BASE_URL" | "ANTHROPIC_BASE_URL" => base_url = Some(value.to_string()),
                "PI_SLOW_MODEL" => slow_model = Some(value.to_string()),
                "PI_SMOL_MODEL" => smol_model = Some(value.to_string()),
                "PI_PLAN_MODEL" => plan_model = Some(value.to_string()),
                _ => {} // ignore unknown keys
            }
        }
    }

    if openai_api_key.is_none() {
        anyhow::bail!("No OPENAI_API_KEY found in .env file '{}'.", file_path);
    }

    Ok(OmpConfiguration {
        alias_name: alias_name.to_string(),
        openai_api_key,
        base_url,
        slow_model,
        smol_model,
        plan_model,
    })
}

/// Interactive mode for creating an OMP configuration
fn parse_interactive_omp_config(alias_name: &str) -> Result<OmpConfiguration> {
    use crate::interactive::read_input;

    let key = read_input("OpenAI API key: ")?;
    if key.is_empty() {
        anyhow::bail!("OpenAI API key is required.");
    }

    let base_url = read_input("Base URL for a custom gateway (press Enter to skip): ")?;
    let slow_model = read_input("PI_SLOW_MODEL (press Enter to skip): ")?;
    let smol_model = read_input("PI_SMOL_MODEL (press Enter to skip): ")?;
    let plan_model = read_input("PI_PLAN_MODEL (press Enter to skip): ")?;

    let opt = |s: String| if s.is_empty() { None } else { Some(s) };

    Ok(OmpConfiguration {
        alias_name: alias_name.to_string(),
        openai_api_key: Some(key),
        base_url: opt(base_url),
        slow_model: opt(slow_model),
        smol_model: opt(smol_model),
        plan_model: opt(plan_model),
    })
}
/// List OMP configurations
pub fn handle_omp_list(plain: bool, name: bool, storage: &ConfigStorage) -> Result<()> {
    let configs = storage.omp_configurations.as_ref();

    if configs.is_none() || configs.as_ref().map(|m| m.is_empty()).unwrap_or(true) {
        println!("No OMP configurations found.");
        return Ok(());
    }

    let mut configs: Vec<&OmpConfiguration> = configs.unwrap().values().collect();
    configs.sort_by(|a, b| a.alias_name.cmp(&b.alias_name));

    if name {
        if plain {
            for config in configs {
                println!("{}", config.alias_name);
            }
        } else {
            let mut json_array = Vec::new();
            for config in &configs {
                json_array.push(serde_json::json!({
                    "alias_name": config.alias_name,
                }));
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&json_array).expect("Should serialize to JSON")
            );
        }
        return Ok(());
    }

    if plain {
        println!("{:<20} Key", "Name");
        println!("{}", "-".repeat(44));
        for config in &configs {
            let key_display = config
                .openai_api_key
                .as_deref()
                .map(mask_key)
                .unwrap_or_else(|| "(none)".to_string());
            println!("{:<20} {}", config.alias_name, key_display);
            if let Some(url) = &config.base_url {
                println!("  base_url: {}", url);
            }
        }
    } else {
        let mut json_array = Vec::new();
        for config in &configs {
            let mut json_obj = serde_json::json!({
                "alias_name": config.alias_name,
            });

            if let Some(ref key) = config.openai_api_key {
                json_obj["openai_api_key"] = serde_json::json!(mask_key(key));
            }
            if let Some(ref model) = config.slow_model {
                json_obj["slow_model"] = serde_json::json!(model);
            }
            if let Some(ref model) = config.smol_model {
                json_obj["smol_model"] = serde_json::json!(model);
            }
            if let Some(ref model) = config.plan_model {
                json_obj["plan_model"] = serde_json::json!(model);
            }
            if let Some(url) = &config.base_url {
                json_obj["base_url"] = serde_json::json!(url);
            }

            json_array.push(json_obj);
        }
        println!(
            "{}",
            serde_json::to_string_pretty(&json_array).expect("Should serialize to JSON")
        );
    }

    Ok(())
}

/// Switch to an OMP configuration and launch OMP CLI
pub fn handle_omp_use(
    alias_name: String,
    r#continue: bool,
    resume: Option<String>,
    prompt: Vec<String>,
    storage: &mut ConfigStorage,
) -> Result<()> {
    let config = storage
        .get_omp_configuration(&alias_name)
        .ok_or_else(|| anyhow!("OMP configuration '{}' not found.", alias_name))?
        .clone();

    write_env_file(&config)?;
    crate::omp::env_writer::update_config_yml(&config)?;
    println!("Switched to OMP configuration '{}'.", config.alias_name);

    launch_omp(r#continue, resume, prompt)?;
    Ok(())
}

/// Launch OMP CLI with optional arguments
fn launch_omp(r#continue: bool, resume: Option<String>, prompt: Vec<String>) -> Result<()> {
    let mut cmd = Command::new(resolve_npm_cli("omp"));

    if r#continue {
        cmd.arg("--continue");
    }
    if let Some(session) = resume {
        cmd.arg("--resume").arg(session);
    }
    if !prompt.is_empty() {
        cmd.arg("--prompt").arg(prompt.join(" "));
    }

    match cmd.status() {
        Ok(status) => {
            if !status.success() {
                return Err(anyhow!("OMP exited with status: {}", status));
            }
        }
        Err(e) => {
            return Err(anyhow!(
                "Failed to launch OMP: {}. Make sure OMP is installed and available in your PATH.",
                e
            ));
        }
    }
    Ok(())
}

/// Remove OMP configurations
pub fn handle_omp_remove(alias_names: Vec<String>, storage: &mut ConfigStorage) -> Result<()> {
    let mut _removed_count = 0;
    let mut not_found_aliases = Vec::new();

    for alias in &alias_names {
        if storage.remove_omp_configuration(alias) {
            _removed_count += 1;
            println!("OMP configuration '{}' removed successfully", alias);
        } else {
            not_found_aliases.push(alias.clone());
        }
    }

    storage.save()?;

    if !not_found_aliases.is_empty() {
        for alias in &not_found_aliases {
            println!("OMP configuration '{}' not found, skipping", alias);
        }
    }

    Ok(())
}

/// Enter interactive mode for OMP configuration selection
pub fn handle_omp_interactive(storage: &ConfigStorage) -> Result<()> {
    crate::interactive::handle_omp_interactive_selection(storage)
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
    fn test_mask_key_short() {
        let masked = mask_key("abcdef");
        assert_eq!(masked, "abc****");
    }

    #[test]
    fn test_mask_key_long() {
        let masked = mask_key("sk-ant-abcdef1234567890");
        assert_eq!(masked, "sk-a****7890");
    }

    #[test]
    fn test_mask_key_very_long() {
        let masked = mask_key("sk-openai-abc123def456ghi789");
        assert_eq!(masked, "sk-o****i789");
    }
}
