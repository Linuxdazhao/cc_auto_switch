use crate::omp::OmpConfiguration;
use anyhow::{Result, anyhow};
use std::fs;
use std::path::PathBuf;

/// Build the .env path under ~/.omp/agent/.env
fn get_env_path(base_dir: Option<&PathBuf>) -> Result<PathBuf> {
    let agent_dir = match base_dir {
        Some(dir) => dir.join(".omp").join("agent"),
        None => dirs::home_dir()
            .ok_or_else(|| anyhow!("Could not find home directory"))?
            .join(".omp")
            .join("agent"),
    };

    if !agent_dir.exists() {
        fs::create_dir_all(&agent_dir)
            .map_err(|e| anyhow!("Failed to create .omp/agent directory: {}", e))?;
    }

    Ok(agent_dir.join(".env"))
}

/// Build the default path to `~/.omp/agent/.env` without creating any
/// directories. Read-only lookup.
pub fn default_omp_env_path() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not find home directory"))?;
    Ok(default_omp_env_path_in(&home))
}

/// Inner helper: build the `<home>/.omp/agent/.env` path without filesystem
/// side effects. Exposed for tests.
fn default_omp_env_path_in(home: &std::path::Path) -> PathBuf {
    home.join(".omp").join("agent").join(".env")
}

/// Write OmpConfiguration to ~/.omp/agent/.env
pub fn write_env_file(config: &OmpConfiguration) -> Result<()> {
    let env_path = get_env_path(None)?;
    write_env_file_to_path(config, &env_path)
}

/// Write OmpConfiguration to a specific path (for testing)
fn write_env_file_to_path(config: &OmpConfiguration, env_path: &PathBuf) -> Result<()> {
    let mut lines = Vec::new();

    lines.push(format!(
        "# Managed by cc-switch (alias: {})",
        config.alias_name
    ));
    lines.push(String::new());

    match config.auth_mode.as_str() {
        "anthropic" => {
            if let Some(ref token) = config.anthropic_oauth_token {
                lines.push(format!("ANTHROPIC_OAUTH_TOKEN={}", token));
            } else if let Some(ref key) = config.anthropic_api_key {
                lines.push(format!("ANTHROPIC_API_KEY={}", key));
            }
        }
        "openai" => {
            if let Some(ref key) = config.openai_api_key {
                lines.push(format!("OPENAI_API_KEY={}", key));
            }
        }
        _ => {
            return Err(anyhow!(
                "Unknown auth mode: {}. Expected 'anthropic' or 'openai'.",
                config.auth_mode
            ));
        }
    }
    // Optional base URL
    if let Some(ref url) = config.base_url {
        lines.push(format!("ANTHROPIC_BASE_URL={}", url));
    }

    // Optional model knobs
    if let Some(ref model) = config.slow_model {
        lines.push(format!("PI_SLOW_MODEL={}", model));
    }
    if let Some(ref model) = config.smol_model {
        lines.push(format!("PI_SMOL_MODEL={}", model));
    }
    if let Some(ref model) = config.plan_model {
        lines.push(format!("PI_PLAN_MODEL={}", model));
    }

    lines.push(String::new());
    let content = lines.join("\n");

    // Ensure parent directory exists
    if let Some(parent) = env_path.parent()
        && !parent.exists()
    {
        fs::create_dir_all(parent).map_err(|e| anyhow!("Failed to create directory: {}", e))?;
    }

    fs::write(env_path, content).map_err(|e| anyhow!("Failed to write .env file: {}", e))?;

    Ok(())
}
/// Update ~/.omp/agent/config.yml modelRoles.default with the slow_model from config
pub fn update_config_yml(config: &OmpConfiguration) -> Result<()> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not find home directory"))?;
    let config_path = home.join(".omp").join("agent").join("config.yml");

    if !config_path.exists() {
        return Ok(()); // No config.yml, nothing to update
    }

    // Use slow_model as the default model (all model roles use the same model)
    let default_model = match &config.slow_model {
        Some(model) => model,
        None => return Ok(()), // No model configured, skip
    };

    let content = fs::read_to_string(&config_path)
        .map_err(|e| anyhow!("Failed to read config.yml: {}", e))?;

    // Replace the default model line using regex-like approach
    let mut new_lines = Vec::new();
    let mut in_model_roles = false;
    let mut updated = false;

    for line in content.lines() {
        if line.trim().starts_with("modelRoles:") {
            in_model_roles = true;
            new_lines.push(line.to_string());
        } else if in_model_roles && line.trim().starts_with("default:") {
            // Replace the default line
            let indent = &line[..line.len() - line.trim_start().len()];
            new_lines.push(format!("{}default: \"{}\"", indent, default_model));
            in_model_roles = false;
            updated = true;
        } else {
            // Check if we're leaving modelRoles section (new top-level key)
            if in_model_roles
                && !line.starts_with(' ')
                && !line.starts_with('\t')
                && !line.is_empty()
            {
                in_model_roles = false;
            }
            new_lines.push(line.to_string());
        }
    }

    if updated {
        let new_content = new_lines.join("\n");
        fs::write(&config_path, new_content)
            .map_err(|e| anyhow!("Failed to write config.yml: {}", e))?;
    }

    Ok(())
}
/// Update ~/.omp/agent/models.yml with the provider and models from config
///
/// When a config has a base_url, creates/updates a provider entry in models.yml
/// so OMP can recognize the custom models in its model picker.
pub fn update_models_yml(config: &OmpConfiguration) -> Result<()> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not find home directory"))?;
    let agent_dir = home.join(".omp").join("agent");
    let models_path = agent_dir.join("models.yml");

    // Only update if we have a base_url (custom endpoint)
    let base_url = match &config.base_url {
        Some(url) => url,
        None => return Ok(()),
    };

    // Collect unique model IDs from all model roles, stripping provider prefix
    let mut model_ids: Vec<String> = Vec::new();
    for model in [&config.slow_model, &config.smol_model, &config.plan_model]
        .into_iter()
        .flatten()
    {
        // Strip provider prefix if present (e.g., "token-hub/model_98358b" -> "model_98358b")
        let model_id = if let Some(pos) = model.rfind('/') {
            &model[pos + 1..]
        } else {
            model.as_str()
        };
        if !model_ids.contains(&model_id.to_string()) {
            model_ids.push(model_id.to_string());
        }
    }

    if model_ids.is_empty() {
        return Ok(()); // No models to register
    }

    // Determine provider name from alias
    let provider_name = &config.alias_name;

    // Determine API type from auth mode
    let api_type = match config.auth_mode.as_str() {
        "anthropic" => "anthropic-messages",
        "openai" => "openai-chat",
        _ => "anthropic-messages",
    };

    // Get API key
    let api_key = match config.auth_mode.as_str() {
        "anthropic" => config
            .anthropic_api_key
            .as_deref()
            .or(config.anthropic_oauth_token.as_deref()),
        "openai" => config.openai_api_key.as_deref(),
        _ => None,
    };

    // Ensure agent directory exists
    if !agent_dir.exists() {
        fs::create_dir_all(&agent_dir)
            .map_err(|e| anyhow!("Failed to create agent directory: {}", e))?;
    }

    // Read existing models.yml or start fresh
    let existing_content = if models_path.exists() {
        fs::read_to_string(&models_path).map_err(|e| anyhow!("Failed to read models.yml: {}", e))?
    } else {
        "providers:\n".to_string()
    };

    // Parse existing content to find providers section
    let mut lines: Vec<String> = existing_content.lines().map(|l| l.to_string()).collect();

    // Remove existing provider entry with same name if present
    let provider_header = format!("  {}:", provider_name);
    let mut start_idx = None;
    let mut end_idx = None;
    for (i, line) in lines.iter().enumerate() {
        if line == &provider_header {
            start_idx = Some(i);
        } else if start_idx.is_some() && end_idx.is_none() {
            // Check if we've reached the next provider (2-space indent, not 4+)
            if line.starts_with("  ") && !line.starts_with("    ") && !line.trim().is_empty() {
                end_idx = Some(i);
                break;
            }
        }
    }

    if let Some(start) = start_idx {
        let end = end_idx.unwrap_or(lines.len());
        lines.drain(start..end);
    }

    // Build new provider entry
    let mut provider_lines = Vec::new();
    provider_lines.push(format!("  {}:", provider_name));
    provider_lines.push(format!("    baseUrl: {}", base_url));
    if let Some(key) = api_key {
        provider_lines.push(format!("    apiKey: \"{}\"", key));
    }
    provider_lines.push(format!("    api: {}", api_type));
    provider_lines.push("    authHeader: true".to_string());
    provider_lines.push("    disableStrictTools: true".to_string());
    provider_lines.push("    models:".to_string());
    for model_id in &model_ids {
        provider_lines.push(format!("      - id: {}", model_id));
        provider_lines.push(format!("        name: {}", model_id));
        provider_lines.push("        contextWindow: 1000000".to_string());
        provider_lines.push("        maxTokens: 16384".to_string());
    }

    // Append provider entry to end of file
    // Remove trailing empty lines first
    while lines.last().map(|l| l.is_empty()).unwrap_or(false) {
        lines.pop();
    }
    lines.extend(provider_lines);
    lines.push(String::new()); // trailing newline

    let new_content = lines.join("\n");
    fs::write(&models_path, new_content)
        .map_err(|e| anyhow!("Failed to write models.yml: {}", e))?;

    Ok(())
}

/// Parse a .env file into an OmpConfiguration
pub fn parse_env_file(file_path: &str, alias_name: &str) -> Result<OmpConfiguration> {
    let content = fs::read_to_string(file_path)
        .map_err(|e| anyhow!("Failed to read .env file '{}': {}", file_path, e))?;

    let mut anthropic_api_key = None;
    let mut anthropic_oauth_token = None;
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
                "ANTHROPIC_API_KEY" => anthropic_api_key = Some(value.to_string()),
                "ANTHROPIC_OAUTH_TOKEN" => anthropic_oauth_token = Some(value.to_string()),
                "OPENAI_API_KEY" => openai_api_key = Some(value.to_string()),
                "PI_SLOW_MODEL" => slow_model = Some(value.to_string()),
                "PI_SMOL_MODEL" => smol_model = Some(value.to_string()),
                "ANTHROPIC_BASE_URL" => base_url = Some(value.to_string()),
                "PI_PLAN_MODEL" => plan_model = Some(value.to_string()),
                _ => {} // ignore unknown keys
            }
        }
    }

    // Determine auth mode from what's present
    let auth_mode = if anthropic_api_key.is_some() || anthropic_oauth_token.is_some() {
        "anthropic"
    } else if openai_api_key.is_some() {
        "openai"
    } else {
        anyhow::bail!(
            "Could not determine auth mode from .env file '{}'. Expected ANTHROPIC_API_KEY, ANTHROPIC_OAUTH_TOKEN, or OPENAI_API_KEY.",
            file_path
        );
    };

    Ok(OmpConfiguration {
        alias_name: alias_name.to_string(),
        auth_mode: auth_mode.to_string(),
        anthropic_api_key,
        anthropic_oauth_token,
        openai_api_key,
        base_url,
        slow_model,
        smol_model,
        plan_model,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_anthropic_config(alias: &str) -> OmpConfiguration {
        OmpConfiguration {
            alias_name: alias.to_string(),
            auth_mode: "anthropic".to_string(),
            anthropic_api_key: Some("sk-ant-test".to_string()),
            anthropic_oauth_token: None,
            openai_api_key: None,
            base_url: None,
            slow_model: None,
            smol_model: None,
            plan_model: None,
        }
    }

    #[test]
    fn test_write_anthropic_api_key() {
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let env_path = temp_dir.path().join(".omp").join("agent").join(".env");

        let config = make_anthropic_config("test");
        write_env_file_to_path(&config, &env_path).expect("Should write");

        let content = fs::read_to_string(&env_path).expect("Should read");
        assert!(content.contains("ANTHROPIC_API_KEY=sk-ant-test"));
        assert!(content.contains("# Managed by cc-switch (alias: test)"));
    }

    #[test]
    fn test_write_anthropic_oauth_token() {
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let env_path = temp_dir.path().join(".omp").join("agent").join(".env");

        let config = OmpConfiguration {
            alias_name: "test".to_string(),
            auth_mode: "anthropic".to_string(),
            anthropic_api_key: Some("sk-ant-test".to_string()),
            anthropic_oauth_token: Some("oauth-token-123".to_string()),
            openai_api_key: None,
            base_url: None,
            slow_model: None,
            smol_model: None,
            plan_model: None,
        };
        write_env_file_to_path(&config, &env_path).expect("Should write");

        let content = fs::read_to_string(&env_path).expect("Should read");
        // OAuth token takes precedence
        assert!(content.contains("ANTHROPIC_OAUTH_TOKEN=oauth-token-123"));
        assert!(!content.contains("ANTHROPIC_API_KEY"));
    }

    #[test]
    fn test_write_openai_mode() {
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let env_path = temp_dir.path().join(".omp").join("agent").join(".env");

        let config = OmpConfiguration {
            alias_name: "gpt".to_string(),
            auth_mode: "openai".to_string(),
            anthropic_api_key: None,
            anthropic_oauth_token: None,
            openai_api_key: Some("sk-openai-test".to_string()),
            base_url: None,
            slow_model: Some("openai/gpt-5.3-codex:high".to_string()),
            smol_model: Some("openai/gpt-4o-mini".to_string()),
            plan_model: None,
        };
        write_env_file_to_path(&config, &env_path).expect("Should write");

        let content = fs::read_to_string(&env_path).expect("Should read");
        assert!(content.contains("OPENAI_API_KEY=sk-openai-test"));
        assert!(content.contains("PI_SLOW_MODEL=openai/gpt-5.3-codex:high"));
        assert!(content.contains("PI_SMOL_MODEL=openai/gpt-4o-mini"));
        assert!(!content.contains("PI_PLAN_MODEL"));
    }
    #[test]
    fn test_write_with_base_url() {
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let env_path = temp_dir.path().join(".omp").join("agent").join(".env");

        let config = OmpConfiguration {
            alias_name: "token-hub".to_string(),
            auth_mode: "anthropic".to_string(),
            anthropic_api_key: Some("th-test-key".to_string()),
            anthropic_oauth_token: None,
            openai_api_key: None,
            base_url: Some("http://token-hub.example.com/compatible".to_string()),
            slow_model: Some("model_98358b".to_string()),
            smol_model: Some("model_98358b".to_string()),
            plan_model: Some("model_98358b".to_string()),
        };
        write_env_file_to_path(&config, &env_path).expect("Should write");

        let content = fs::read_to_string(&env_path).expect("Should read");
        assert!(content.contains("ANTHROPIC_API_KEY=th-test-key"));
        assert!(content.contains("ANTHROPIC_BASE_URL=http://token-hub.example.com/compatible"));
        assert!(content.contains("PI_SLOW_MODEL=model_98358b"));
        assert!(content.contains("PI_SMOL_MODEL=model_98358b"));
        assert!(content.contains("PI_PLAN_MODEL=model_98358b"));
    }

    #[test]
    fn test_parse_env_file_anthropic() {
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let env_path = temp_dir.path().join(".env");

        fs::write(
            &env_path,
            "# comment\nANTHROPIC_API_KEY=sk-ant-parsed\nPI_SLOW_MODEL=claude-3-5-sonnet\n",
        )
        .expect("Should write");

        let config = parse_env_file(env_path.to_str().unwrap(), "parsed").expect("Should parse");
        assert_eq!(config.auth_mode, "anthropic");
        assert_eq!(config.anthropic_api_key, Some("sk-ant-parsed".to_string()));
        assert_eq!(config.slow_model, Some("claude-3-5-sonnet".to_string()));
    }

    #[test]
    fn test_parse_env_file_openai() {
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let env_path = temp_dir.path().join(".env");

        fs::write(
            &env_path,
            "OPENAI_API_KEY=sk-parsed\nPI_SLOW_MODEL=openai/gpt-5\n",
        )
        .expect("Should write");

        let config = parse_env_file(env_path.to_str().unwrap(), "parsed").expect("Should parse");
        assert_eq!(config.auth_mode, "openai");
        assert_eq!(config.openai_api_key, Some("sk-parsed".to_string()));
        assert_eq!(config.slow_model, Some("openai/gpt-5".to_string()));
    }

    #[test]
    fn test_parse_env_file_no_credentials() {
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let env_path = temp_dir.path().join(".env");

        fs::write(&env_path, "PI_SLOW_MODEL=some-model\n").expect("Should write");

        let result = parse_env_file(env_path.to_str().unwrap(), "parsed");
        assert!(result.is_err());
    }

    #[test]
    fn test_default_omp_env_path_ends_correctly() {
        let path = default_omp_env_path().expect("Should resolve default omp env path");
        let path_str = path.to_string_lossy();
        assert!(
            path_str.ends_with(".omp/agent/.env") || path_str.ends_with(".omp\\agent\\.env"),
            "Path should end with .omp/agent/.env, got: {}",
            path_str
        );
    }
}
