use crate::grok::GrokConfiguration;
use anyhow::{Result, anyhow};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use toml_edit::{Array, DocumentMut, InlineTable, Item, Table, Value, value};

/// Build the default path to `~/.grok/config.toml` without filesystem side effects.
pub fn default_grok_config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not find home directory"))?;
    Ok(default_grok_config_path_in(&home))
}

fn default_grok_config_path_in(home: &Path) -> PathBuf {
    home.join(".grok").join("config.toml")
}

/// Write the configuration into `~/.grok/config.toml`, creating/merging the
/// `[model.<alias>]` block and setting `[models].default` to the alias.
pub fn write_grok_config(config: &GrokConfiguration) -> Result<()> {
    let path = default_grok_config_path()?;
    write_grok_config_to_path(config, &path)
}

/// Write the configuration to a specific config.toml path (for testing).
fn write_grok_config_to_path(config: &GrokConfiguration, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.exists()
    {
        fs::create_dir_all(parent).map_err(|e| anyhow!("Failed to create directory: {}", e))?;
    }

    let mut doc = if path.exists() {
        let content = fs::read_to_string(path)
            .map_err(|e| anyhow!("Failed to read {}: {}", path.display(), e))?;
        content
            .parse::<DocumentMut>()
            .map_err(|e| anyhow!("Failed to parse {}: {}", path.display(), e))?
    } else {
        DocumentMut::new()
    };

    // Set the active default model: [models] default = "<alias>"
    if !doc.contains_key("models") {
        doc["models"] = Item::Table(Table::new());
    }
    doc["models"]["default"] = value(config.alias_name.as_str());

    // Ensure [model] exists as an implicit parent so we render [model.<alias>].
    if doc.get("model").and_then(|i| i.as_table()).is_none() {
        let mut parent = Table::new();
        parent.set_implicit(true);
        doc["model"] = Item::Table(parent);
    }
    let model_parent = doc["model"]
        .as_table_mut()
        .ok_or_else(|| anyhow!("`model` in config.toml is not a table"))?;
    model_parent.insert(&config.alias_name, Item::Table(build_model_table(config)));

    fs::write(path, doc.to_string())
        .map_err(|e| anyhow!("Failed to write {}: {}", path.display(), e))?;

    Ok(())
}

fn build_model_table(config: &GrokConfiguration) -> Table {
    let mut tbl = Table::new();
    tbl["model"] = value(config.model.as_str());
    if let Some(ref url) = config.base_url {
        tbl["base_url"] = value(url.as_str());
    }
    if let Some(ref name) = config.name {
        tbl["name"] = value(name.as_str());
    }
    if let Some(ref backend) = config.api_backend {
        tbl["api_backend"] = value(backend.as_str());
    }
    if let Some(ctx) = config.context_window {
        tbl["context_window"] = value(ctx as i64);
    }
    if let Some(ref key) = config.api_key {
        tbl["api_key"] = value(key.as_str());
    }
    if let Some(ref keys) = config.env_key
        && !keys.is_empty()
    {
        let mut arr = Array::new();
        for k in keys {
            arr.push(k.as_str());
        }
        tbl["env_key"] = value(arr);
    }
    if let Some(ref headers) = config.extra_headers
        && !headers.is_empty()
    {
        let mut inline = InlineTable::new();
        for (k, v) in headers {
            inline.insert(k, Value::from(v.as_str()));
        }
        tbl["extra_headers"] = value(inline);
    }
    tbl
}

/// Parse a `[model.<alias>]` entry from a grok config.toml into a GrokConfiguration.
pub fn parse_grok_config_file(file_path: &str, alias_name: &str) -> Result<GrokConfiguration> {
    let content = fs::read_to_string(file_path)
        .map_err(|e| anyhow!("Failed to read config file '{}': {}", file_path, e))?;
    let doc = content
        .parse::<DocumentMut>()
        .map_err(|e| anyhow!("Failed to parse config file '{}': {}", file_path, e))?;

    let model_tbl = doc
        .get("model")
        .and_then(|i| i.as_table())
        .and_then(|t| t.get(alias_name))
        .and_then(|i| i.as_table())
        .ok_or_else(|| anyhow!("No [model.{}] entry found in '{}'.", alias_name, file_path))?;

    let model = model_tbl
        .get("model")
        .and_then(|i| i.as_str())
        .ok_or_else(|| {
            anyhow!(
                "[model.{}] is missing the required `model` field.",
                alias_name
            )
        })?
        .to_string();

    let get_str = |k: &str| {
        model_tbl
            .get(k)
            .and_then(|i| i.as_str())
            .map(str::to_string)
    };

    let context_window = model_tbl
        .get("context_window")
        .and_then(|i| i.as_integer())
        .map(|n| n as u32);

    let env_key = model_tbl.get("env_key").and_then(|item| {
        if let Some(s) = item.as_str() {
            Some(vec![s.to_string()])
        } else {
            item.as_array().map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
        }
    });

    let extra_headers = model_tbl
        .get("extra_headers")
        .and_then(|i| i.as_inline_table())
        .map(|it| {
            let mut map = BTreeMap::new();
            for (k, v) in it.iter() {
                if let Some(s) = v.as_str() {
                    map.insert(k.to_string(), s.to_string());
                }
            }
            map
        });

    Ok(GrokConfiguration {
        alias_name: alias_name.to_string(),
        model,
        base_url: get_str("base_url"),
        name: get_str("name"),
        api_backend: get_str("api_backend"),
        context_window,
        api_key: get_str("api_key"),
        env_key,
        extra_headers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample() -> GrokConfiguration {
        let mut headers = BTreeMap::new();
        headers.insert("x-api-key".to_string(), "sk-ant-xxx".to_string());
        GrokConfiguration {
            alias_name: "claude-opus".to_string(),
            model: "claude-opus-4-6".to_string(),
            base_url: Some("https://api.anthropic.com/v1".to_string()),
            name: Some("Claude Opus 4.6".to_string()),
            api_backend: Some("messages".to_string()),
            context_window: Some(200000),
            api_key: None,
            env_key: Some(vec!["ANTHROPIC_API_KEY".to_string()]),
            extra_headers: Some(headers),
        }
    }

    #[test]
    fn test_write_creates_default_and_model() {
        let temp = TempDir::new().expect("temp dir");
        let path = temp.path().join(".grok").join("config.toml");
        write_grok_config_to_path(&sample(), &path).expect("write");

        let content = fs::read_to_string(&path).expect("read");
        assert!(content.contains("[models]"));
        assert!(content.contains("default = \"claude-opus\""));
        assert!(content.contains("[model.claude-opus]"));
        assert!(content.contains("model = \"claude-opus-4-6\""));
        assert!(content.contains("env_key = [\"ANTHROPIC_API_KEY\"]"));
        assert!(content.contains("api_backend = \"messages\""));
        assert!(content.contains("context_window = 200000"));
        assert!(content.contains("x-api-key"));
    }

    #[test]
    fn test_write_preserves_existing_entries() {
        let temp = TempDir::new().expect("temp dir");
        let path = temp.path().join(".grok").join("config.toml");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            "# my config\n[model.other]\nmodel = \"gpt-4o\"\nenv_key = \"OPENAI_API_KEY\"\n",
        )
        .unwrap();

        write_grok_config_to_path(&sample(), &path).expect("write");
        let content = fs::read_to_string(&path).expect("read");
        assert!(content.contains("# my config"));
        assert!(content.contains("[model.other]"));
        assert!(content.contains("[model.claude-opus]"));
        assert!(content.contains("default = \"claude-opus\""));
    }

    #[test]
    fn test_roundtrip_parse() {
        let temp = TempDir::new().expect("temp dir");
        let path = temp.path().join(".grok").join("config.toml");
        write_grok_config_to_path(&sample(), &path).expect("write");

        let parsed = parse_grok_config_file(path.to_str().unwrap(), "claude-opus").expect("parse");
        assert_eq!(parsed, sample());
    }

    #[test]
    fn test_parse_missing_entry_errors() {
        let temp = TempDir::new().expect("temp dir");
        let path = temp.path().join("config.toml");
        fs::write(&path, "[model.a]\nmodel = \"x\"\n").unwrap();
        let result = parse_grok_config_file(path.to_str().unwrap(), "missing");
        assert!(result.is_err());
    }

    #[test]
    fn test_default_path_shape() {
        let path = default_grok_config_path().expect("path");
        let s = path.to_string_lossy();
        assert!(s.ends_with(".grok/config.toml") || s.ends_with(".grok\\config.toml"));
    }
}
