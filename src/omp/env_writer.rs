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

    let key = config.openai_api_key.as_ref().ok_or_else(|| {
        anyhow!(
            "OMP configuration '{}' has no OpenAI API key.",
            config.alias_name
        )
    })?;
    lines.push(format!("OPENAI_API_KEY={}", key));

    // The OpenAI base URL for a custom gateway is registered in models.yml as the
    // provider `baseUrl`, so it is not written to .env.

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

/// Build the indented `models.yml` provider block (YAML lines) for a config.
///
/// omp is registered as an OpenAI-standard provider, so the `api` value is
/// always `openai-completions` (the OpenAI `/v1/chat/completions` protocol) and
/// the resolved key is sent as an `Authorization: Bearer` header.
fn build_models_yml_provider_block(
    config: &OmpConfiguration,
    base_url: &str,
    model_ids: &[String],
) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!("  {}:", config.alias_name));
    lines.push(format!("    baseUrl: {}", base_url));
    if let Some(ref key) = config.openai_api_key {
        lines.push(format!("    apiKey: \"{}\"", key));
    }
    lines.push("    api: openai-completions".to_string());
    lines.push("    authHeader: true".to_string());
    lines.push("    models:".to_string());
    for model_id in model_ids {
        lines.push(format!("      - id: {}", model_id));
        lines.push(format!("        name: {}", model_id));
        lines.push("        contextWindow: 1000000".to_string());
        lines.push("        maxTokens: 16384".to_string());
    }
    lines
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
    let provider_lines = build_models_yml_provider_block(config, base_url, &model_ids);

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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_config(alias: &str) -> OmpConfiguration {
        OmpConfiguration {
            alias_name: alias.to_string(),
            openai_api_key: Some("sk-test".to_string()),
            base_url: None,
            slow_model: None,
            smol_model: None,
            plan_model: None,
        }
    }

    #[test]
    fn test_write_openai_api_key() {
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let env_path = temp_dir.path().join(".omp").join("agent").join(".env");

        let config = make_config("test");
        write_env_file_to_path(&config, &env_path).expect("Should write");

        let content = fs::read_to_string(&env_path).expect("Should read");
        assert!(content.contains("OPENAI_API_KEY=sk-test"));
        assert!(content.contains("# Managed by cc-switch (alias: test)"));
        assert!(!content.contains("ANTHROPIC"));
    }

    #[test]
    fn test_write_with_models() {
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let env_path = temp_dir.path().join(".omp").join("agent").join(".env");

        let config = OmpConfiguration {
            alias_name: "gpt".to_string(),
            openai_api_key: Some("sk-openai-test".to_string()),
            base_url: Some("https://api.example.com/v1".to_string()),
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
        // base_url is registered in models.yml, never written to .env
        assert!(!content.contains("BASE_URL"));
    }

    #[test]
    fn test_write_requires_key() {
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let env_path = temp_dir.path().join(".omp").join("agent").join(".env");

        let config = OmpConfiguration {
            alias_name: "nokey".to_string(),
            openai_api_key: None,
            base_url: None,
            slow_model: None,
            smol_model: None,
            plan_model: None,
        };
        assert!(write_env_file_to_path(&config, &env_path).is_err());
    }

    #[test]
    fn test_models_yml_block_uses_openai_completions() {
        let config = OmpConfiguration {
            alias_name: "gw".to_string(),
            openai_api_key: Some("sk-openai".to_string()),
            base_url: Some("https://api.example.com/v1".to_string()),
            slow_model: Some("gpt-4o".to_string()),
            smol_model: None,
            plan_model: None,
        };
        let block = build_models_yml_provider_block(
            &config,
            "https://api.example.com/v1",
            &["gpt-4o".to_string()],
        );
        let text = block.join("\n");
        assert!(text.contains("api: openai-completions"), "got:\n{text}");
        assert!(!text.contains("openai-chat"));
        assert!(!text.contains("anthropic"));
        assert!(text.contains("apiKey: \"sk-openai\""));
        assert!(text.contains("authHeader: true"));
        // disableStrictTools is Anthropic-only; must never appear now
        assert!(!text.contains("disableStrictTools"));
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
