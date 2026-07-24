use cc_switch::config::ConfigStorage;
use cc_switch::grok::GrokConfiguration;
use cc_switch::grok::config_writer::parse_grok_config_file;
use std::collections::BTreeMap;
use std::fs;
use tempfile::TempDir;

fn make_config(alias: &str) -> GrokConfiguration {
    GrokConfiguration {
        alias_name: alias.to_string(),
        model: "claude-opus-4-6".to_string(),
        base_url: Some("https://api.anthropic.com/v1".to_string()),
        name: Some("Claude Opus".to_string()),
        api_backend: Some("messages".to_string()),
        context_window: Some(200000),
        api_key: None,
        env_key: Some(vec!["ANTHROPIC_API_KEY".to_string()]),
        extra_headers: None,
    }
}

#[test]
fn test_add_and_get_grok_configuration() {
    let mut storage = ConfigStorage::default();
    storage.add_grok_configuration(make_config("opus"));

    let retrieved = storage.get_grok_configuration("opus");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().model, "claude-opus-4-6");
}

#[test]
fn test_list_and_remove_grok_configurations() {
    let mut storage = ConfigStorage::default();
    storage.add_grok_configuration(make_config("work"));
    storage.add_grok_configuration(make_config("personal"));

    assert_eq!(storage.grok_configurations.as_ref().unwrap().len(), 2);

    assert!(storage.remove_grok_configuration("work"));
    assert!(storage.get_grok_configuration("work").is_none());
    assert!(!storage.remove_grok_configuration("work"));
}

#[test]
fn test_parse_grok_config_from_file() {
    let temp_dir = TempDir::new().expect("Should create temp dir");
    let config_path = temp_dir.path().join("config.toml");

    let toml = r#"
[models]
default = "gpt"

[model.gpt]
model = "gpt-4o"
base_url = "https://api.openai.com/v1"
name = "GPT-4o"
env_key = "OPENAI_API_KEY"

[model.opus]
model = "claude-opus-4-6"
base_url = "https://api.anthropic.com/v1"
api_backend = "messages"
context_window = 200000
extra_headers = { "x-api-key" = "sk-ant-xxx", "anthropic-version" = "2023-06-01" }
"#;
    fs::write(&config_path, toml).expect("Should write file");

    let gpt = parse_grok_config_file(config_path.to_str().unwrap(), "gpt").expect("parse gpt");
    assert_eq!(gpt.model, "gpt-4o");
    assert_eq!(gpt.base_url.as_deref(), Some("https://api.openai.com/v1"));
    assert_eq!(gpt.env_key, Some(vec!["OPENAI_API_KEY".to_string()]));

    let opus = parse_grok_config_file(config_path.to_str().unwrap(), "opus").expect("parse opus");
    assert_eq!(opus.api_backend.as_deref(), Some("messages"));
    assert_eq!(opus.context_window, Some(200000));
    let headers = opus.extra_headers.expect("headers present");
    assert_eq!(headers.get("x-api-key").unwrap(), "sk-ant-xxx");
    assert_eq!(headers.get("anthropic-version").unwrap(), "2023-06-01");
}

#[test]
fn test_parse_grok_config_missing_alias_errors() {
    let temp_dir = TempDir::new().expect("Should create temp dir");
    let config_path = temp_dir.path().join("config.toml");
    fs::write(&config_path, "[model.a]\nmodel = \"x\"\n").expect("write");

    let result = parse_grok_config_file(config_path.to_str().unwrap(), "missing");
    assert!(result.is_err());
}

#[test]
fn test_grok_configuration_with_headers_roundtrip_json() {
    let mut headers = BTreeMap::new();
    headers.insert("x-api-key".to_string(), "sk-ant-xxx".to_string());

    let original = GrokConfiguration {
        alias_name: "opus".to_string(),
        model: "claude-opus-4-6".to_string(),
        base_url: Some("https://api.anthropic.com/v1".to_string()),
        name: None,
        api_backend: Some("messages".to_string()),
        context_window: None,
        api_key: None,
        env_key: None,
        extra_headers: Some(headers),
    };

    let json = serde_json::to_string(&original).expect("serialize");
    let parsed: GrokConfiguration = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(original, parsed);
}

#[cfg(test)]
mod cli_parse_tests {
    use cc_switch::cli::{Commands, GrokCommands};
    use clap::Parser;

    #[test]
    fn test_grok_add_with_flags() {
        let args = vec![
            "cc-switch",
            "grok",
            "add",
            "opus",
            "--model",
            "claude-opus-4-6",
            "--base-url",
            "https://api.anthropic.com/v1",
            "--api-backend",
            "messages",
            "--env-key",
            "ANTHROPIC_API_KEY",
            "--header",
            "x-api-key=sk-ant-xxx",
        ];
        let cli = cc_switch::cli::Cli::try_parse_from(args).expect("Should parse grok add");
        let Some(Commands::Grok {
            command:
                Some(GrokCommands::Add {
                    alias_name,
                    model,
                    api_backend,
                    env_key,
                    header,
                    ..
                }),
        }) = cli.command
        else {
            panic!("Expected grok add command");
        };
        assert_eq!(alias_name, "opus");
        assert_eq!(model, Some("claude-opus-4-6".to_string()));
        assert_eq!(api_backend, Some("messages".to_string()));
        assert_eq!(env_key, vec!["ANTHROPIC_API_KEY".to_string()]);
        assert_eq!(header, vec!["x-api-key=sk-ant-xxx".to_string()]);
    }

    #[test]
    fn test_grok_add_from_file_no_value() {
        let args = vec!["cc-switch", "grok", "add", "work", "--from-file"];
        let cli = cc_switch::cli::Cli::try_parse_from(args).expect("Should parse grok --from-file");
        let Some(Commands::Grok {
            command: Some(GrokCommands::Add { from_file, .. }),
        }) = cli.command
        else {
            panic!("Expected grok add command");
        };
        assert_eq!(from_file, Some(None));
    }

    #[test]
    fn test_grok_use_with_prompt() {
        let args = vec!["cc-switch", "grok", "use", "opus", "hello", "world"];
        let cli = cc_switch::cli::Cli::try_parse_from(args).expect("Should parse grok use");
        let Some(Commands::Grok {
            command: Some(GrokCommands::Use { alias_name, prompt }),
        }) = cli.command
        else {
            panic!("Expected grok use command");
        };
        assert_eq!(alias_name, "opus");
        assert_eq!(prompt, vec!["hello".to_string(), "world".to_string()]);
    }
}
