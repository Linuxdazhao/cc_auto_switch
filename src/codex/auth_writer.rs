use crate::codex::CodexConfiguration;
use anyhow::{Result, anyhow};
use serde_json::json;
use std::fs;
use std::path::PathBuf;

/// Build the auth.json path, using home directory or an override for testing
fn get_auth_path(base_dir: Option<&PathBuf>) -> Result<PathBuf> {
    let codex_dir = match base_dir {
        Some(dir) => dir.join(".codex"),
        None => dirs::home_dir()
            .ok_or_else(|| anyhow!("Could not find home directory"))?
            .join(".codex"),
    };

    if !codex_dir.exists() {
        fs::create_dir_all(&codex_dir)
            .map_err(|e| anyhow!("Failed to create .codex directory: {}", e))?;
    }

    Ok(codex_dir.join("auth.json"))
}

/// Write CodexConfiguration to ~/.codex/auth.json
pub fn write_auth_json(config: &CodexConfiguration) -> Result<()> {
    let auth_path = get_auth_path(None)?;
    write_auth_json_to_path(config, &auth_path)
}

/// Write CodexConfiguration to a specific path (for testing)
fn write_auth_json_to_path(config: &CodexConfiguration, auth_path: &PathBuf) -> Result<()> {
    let json_value = if config.auth_mode == "apikey" {
        json!({
            "auth_mode": "apikey",
            "OPENAI_API_KEY": config.openai_api_key,
            "tokens": null
        })
    } else {
        json!({
            "auth_mode": "chatgpt",
            "OPENAI_API_KEY": config.openai_api_key,
            "tokens": {
                "id_token": config.id_token,
                "access_token": config.access_token,
                "refresh_token": config.refresh_token,
                "account_id": config.account_id
            },
            "last_refresh": config.last_refresh
        })
    };

    let json_string = serde_json::to_string_pretty(&json_value)
        .map_err(|e| anyhow!("Failed to serialize auth.json: {}", e))?;

    // Ensure parent directory exists
    if let Some(parent) = auth_path.parent()
        && !parent.exists()
    {
        fs::create_dir_all(parent)
            .map_err(|e| anyhow!("Failed to create directory: {}", e))?;
    }

    fs::write(auth_path, json_string)
        .map_err(|e| anyhow!("Failed to write auth.json: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_write_auth_json_chatgpt_mode() {
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let auth_path = temp_dir.path().join(".codex").join("auth.json");

        let config = CodexConfiguration {
            alias_name: "test".to_string(),
            auth_mode: "chatgpt".to_string(),
            openai_api_key: None,
            id_token: Some("test_id".to_string()),
            access_token: Some("test_access".to_string()),
            refresh_token: Some("test_refresh".to_string()),
            account_id: Some("test_account".to_string()),
            last_refresh: Some("2026-05-16T00:00:00Z".to_string()),
        };

        let result = write_auth_json_to_path(&config, &auth_path);
        assert!(result.is_ok());

        let content = fs::read_to_string(&auth_path).expect("Should read file");
        let parsed: serde_json::Value = serde_json::from_str(&content).expect("Should parse");

        assert_eq!(parsed["auth_mode"], "chatgpt");
        assert_eq!(parsed["tokens"]["id_token"], "test_id");
        assert_eq!(parsed["tokens"]["access_token"], "test_access");
        assert_eq!(parsed["tokens"]["refresh_token"], "test_refresh");
        assert_eq!(parsed["tokens"]["account_id"], "test_account");
    }

    #[test]
    fn test_write_auth_json_apikey_mode() {
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let auth_path = temp_dir.path().join(".codex").join("auth.json");

        let config = CodexConfiguration {
            alias_name: "test".to_string(),
            auth_mode: "apikey".to_string(),
            openai_api_key: Some("sk-ant-test-key".to_string()),
            id_token: None,
            access_token: None,
            refresh_token: None,
            account_id: None,
            last_refresh: None,
        };

        let result = write_auth_json_to_path(&config, &auth_path);
        assert!(result.is_ok());

        let content = fs::read_to_string(&auth_path).expect("Should read file");
        let parsed: serde_json::Value = serde_json::from_str(&content).expect("Should parse");

        assert_eq!(parsed["auth_mode"], "apikey");
        assert_eq!(parsed["OPENAI_API_KEY"], "sk-ant-test-key");
        assert!(parsed["tokens"].is_null());
    }
}
