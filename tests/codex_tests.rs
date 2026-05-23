use cc_switch::codex::CodexConfiguration;
use cc_switch::codex::commands::parse_auth_json_file;
use cc_switch::config::ConfigStorage;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_add_codex_config_from_file() {
    let temp_dir = TempDir::new().expect("Should create temp dir");
    let config_path = temp_dir.path().join("test_auth.json");

    let auth_json = r#"{
        "auth_mode": "chatgpt",
        "OPENAI_API_KEY": null,
        "tokens": {
            "id_token": "test_id",
            "access_token": "test_access",
            "refresh_token": "test_refresh",
            "account_id": "test_account"
        },
        "last_refresh": "2026-05-16T00:00:00Z"
    }"#;

    fs::write(&config_path, auth_json).expect("Should write file");

    let mut storage = ConfigStorage::default();
    let config = parse_auth_json_file(config_path.to_str().unwrap(), "test_alias")
        .expect("Should parse file");

    storage.add_codex_configuration(config);

    let retrieved = storage.get_codex_configuration("test_alias");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().auth_mode, "chatgpt");
}

#[test]
fn test_list_codex_configurations() {
    let mut storage = ConfigStorage::default();

    let config1 = CodexConfiguration {
        alias_name: "work".to_string(),
        auth_mode: "chatgpt".to_string(),
        openai_api_key: None,
        id_token: Some("id1".to_string()),
        access_token: Some("access1".to_string()),
        refresh_token: Some("refresh1".to_string()),
        account_id: Some("account1".to_string()),
        last_refresh: None,
    };

    let config2 = CodexConfiguration {
        alias_name: "personal".to_string(),
        auth_mode: "apikey".to_string(),
        openai_api_key: Some("sk-test-key".to_string()),
        id_token: None,
        access_token: None,
        refresh_token: None,
        account_id: None,
        last_refresh: None,
    };

    storage.add_codex_configuration(config1);
    storage.add_codex_configuration(config2);

    assert_eq!(storage.codex_configurations.as_ref().unwrap().len(), 2);
    assert!(storage.get_codex_configuration("work").is_some());
    assert!(storage.get_codex_configuration("personal").is_some());
}

#[test]
fn test_remove_codex_configurations() {
    let mut storage = ConfigStorage::default();

    let config = CodexConfiguration {
        alias_name: "test".to_string(),
        auth_mode: "chatgpt".to_string(),
        openai_api_key: None,
        id_token: Some("id".to_string()),
        access_token: Some("access".to_string()),
        refresh_token: Some("refresh".to_string()),
        account_id: Some("account".to_string()),
        last_refresh: None,
    };

    storage.add_codex_configuration(config);
    assert!(storage.get_codex_configuration("test").is_some());

    let removed = storage.remove_codex_configuration("test");
    assert!(removed);
    assert!(storage.get_codex_configuration("test").is_none());

    let removed_again = storage.remove_codex_configuration("test");
    assert!(!removed_again);
}

#[cfg(test)]
mod cli_parse_tests {
    use cc_switch::cli::{CodexCommands, Commands};
    use clap::Parser;

    #[test]
    fn test_codex_add_from_file_no_value() {
        let args = vec!["cc-switch", "codex", "add", "work", "--from-file"];
        let cli =
            cc_switch::cli::Cli::try_parse_from(args).expect("Should parse codex add --from-file");
        let Some(Commands::Codex {
            command:
                Some(CodexCommands::Add {
                    alias_name,
                    from_file,
                    ..
                }),
        }) = cli.command
        else {
            panic!("Expected codex add command");
        };
        assert_eq!(alias_name, "work");
        assert_eq!(from_file, Some(None));
    }

    #[test]
    fn test_codex_add_from_file_with_value() {
        let args = vec![
            "cc-switch",
            "codex",
            "add",
            "work",
            "--from-file",
            "/tmp/auth.json",
        ];
        let cli = cc_switch::cli::Cli::try_parse_from(args)
            .expect("Should parse codex add --from-file path");
        let Some(Commands::Codex {
            command:
                Some(CodexCommands::Add {
                    alias_name,
                    from_file,
                    ..
                }),
        }) = cli.command
        else {
            panic!("Expected codex add command");
        };
        assert_eq!(alias_name, "work");
        assert_eq!(from_file, Some(Some("/tmp/auth.json".to_string())));
    }
}
