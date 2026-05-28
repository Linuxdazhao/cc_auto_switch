//! Supervisor tests: verify proxy respawn logic and state file updates.
//!
//! These tests exercise the supervisor's detection of dead proxy handles and the
//! state file update path. We don't spawn real ccs-proxy instances; instead we
//! test the deduplication logic, LifecycleConfig construction, and state file
//! mechanics that the supervisor depends on.

#[cfg(unix)]
mod daemon_supervisor {
    use cc_switch::config::ConfigStorage;
    use cc_switch::config::types::Configuration;
    use cc_switch::daemon::lifecycle::LifecycleConfig;
    use cc_switch::daemon::state::{DaemonState, ProxyEntry};
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn make_config(alias: &str, url: &str) -> Configuration {
        Configuration {
            alias_name: alias.to_string(),
            token: "sk-test".to_string(),
            url: url.to_string(),
            model: None,
            small_fast_model: None,
            max_thinking_tokens: None,
            api_timeout_ms: None,
            claude_code_disable_nonessential_traffic: None,
            anthropic_default_sonnet_model: None,
            anthropic_default_opus_model: None,
            anthropic_default_haiku_model: None,
            claude_code_experimental_agent_teams: None,
            claude_code_disable_1m_context: None,
            claude_code_subagent_model: None,
            claude_code_disable_nonstreaming_fallback: None,
            claude_code_effort_level: None,
            disable_prompt_caching: None,
            claude_code_disable_experimental_betas: None,
            disable_autoupdater: None,
        }
    }

    fn make_storage(configs: &[(&str, &str)]) -> ConfigStorage {
        let mut configurations = BTreeMap::new();
        for (alias, url) in configs {
            configurations.insert(alias.to_string(), make_config(alias, url));
        }
        ConfigStorage {
            configurations,
            claude_settings_dir: None,
            default_storage_mode: None,
            codex_configurations: None,
        }
    }

    #[test]
    fn lifecycle_config_deduplicates_same_upstream() {
        let storage = make_storage(&[
            ("work", "https://api.anthropic.com"),
            ("personal", "https://api.anthropic.com"),
            ("glm", "https://glm.example.com/v1"),
        ]);
        let cfg = LifecycleConfig::from_storage(&storage, false).unwrap();
        // Two unique upstreams, not three.
        assert_eq!(cfg.upstreams.len(), 2);
    }

    #[test]
    fn lifecycle_config_skips_empty_urls() {
        let storage = make_storage(&[("empty", ""), ("work", "https://api.anthropic.com")]);
        let cfg = LifecycleConfig::from_storage(&storage, false).unwrap();
        assert_eq!(cfg.upstreams.len(), 1);
        assert_eq!(cfg.upstreams[0].1, "https://api.anthropic.com");
    }

    #[test]
    fn lifecycle_config_paths_are_in_cc_switch_dir() {
        let storage = make_storage(&[("x", "https://api.anthropic.com")]);
        let cfg = LifecycleConfig::from_storage(&storage, false).unwrap();
        let state_str = cfg.state_path.to_string_lossy();
        let pid_str = cfg.pidfile_path.to_string_lossy();
        assert!(state_str.contains(".cc-switch"), "state_path: {state_str}");
        assert!(pid_str.contains(".cc-switch"), "pidfile_path: {pid_str}");
        assert!(state_str.ends_with("daemon-state.json"));
        assert!(pid_str.ends_with("daemon.pid"));
    }

    #[test]
    fn state_file_restart_count_increments() {
        let dir = TempDir::new().unwrap();
        let state_path = dir.path().join("daemon-state.json");

        let mut state = DaemonState {
            schema_version: 1,
            pid: 42,
            started_at: "2026-05-28T00:00:00Z".to_owned(),
            stopped_at: None,
            data_root: PathBuf::from("/tmp"),
            proxies: vec![ProxyEntry {
                provider: "claude".to_owned(),
                upstream: "https://api.anthropic.com".to_owned(),
                proxy_port: 41001,
                api_port: 41501,
                data_dir: PathBuf::from("/tmp/data"),
                started_at: "2026-05-28T00:00:00Z".to_owned(),
                restart_count: 0,
            }],
        };
        state.save(&state_path).unwrap();

        // Simulate supervisor respawn: bump restart_count, update ports.
        state.proxies[0].restart_count += 1;
        state.proxies[0].proxy_port = 41010;
        state.proxies[0].started_at = "2026-05-28T01:00:00Z".to_owned();
        state.save(&state_path).unwrap();

        let loaded = DaemonState::load(&state_path).unwrap().unwrap();
        assert_eq!(loaded.proxies[0].restart_count, 1);
        assert_eq!(loaded.proxies[0].proxy_port, 41010);
    }

    #[test]
    fn state_file_multiple_proxies_independent_restart() {
        let dir = TempDir::new().unwrap();
        let state_path = dir.path().join("daemon-state.json");

        let mut state = DaemonState {
            schema_version: 1,
            pid: 42,
            started_at: "2026-05-28T00:00:00Z".to_owned(),
            stopped_at: None,
            data_root: PathBuf::from("/tmp"),
            proxies: vec![
                ProxyEntry {
                    provider: "claude".to_owned(),
                    upstream: "https://a.example".to_owned(),
                    proxy_port: 8001,
                    api_port: 9001,
                    data_dir: PathBuf::from("/tmp/a"),
                    started_at: "2026-05-28T00:00:00Z".to_owned(),
                    restart_count: 0,
                },
                ProxyEntry {
                    provider: "claude".to_owned(),
                    upstream: "https://b.example".to_owned(),
                    proxy_port: 8002,
                    api_port: 9002,
                    data_dir: PathBuf::from("/tmp/b"),
                    started_at: "2026-05-28T00:00:00Z".to_owned(),
                    restart_count: 0,
                },
            ],
        };

        // Only proxy B restarts.
        state.proxies[1].restart_count = 3;
        state.proxies[1].proxy_port = 8099;
        state.save(&state_path).unwrap();

        let loaded = DaemonState::load(&state_path).unwrap().unwrap();
        assert_eq!(loaded.proxies[0].restart_count, 0, "proxy A untouched");
        assert_eq!(loaded.proxies[1].restart_count, 3, "proxy B restarted 3x");
        assert_eq!(loaded.proxies[1].proxy_port, 8099);
    }

    #[test]
    fn empty_configurations_yields_zero_upstreams() {
        let storage = make_storage(&[]);
        let cfg = LifecycleConfig::from_storage(&storage, false).unwrap();
        assert!(cfg.upstreams.is_empty());
    }
}
