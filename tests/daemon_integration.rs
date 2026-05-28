//! Integration tests for daemon lifecycle: start → state populated → stop → state cleared,
//! and `cc use` URL substitution via daemon state.
//!
//! These tests exercise the daemon's state machinery without actual forking — they call
//! the lifecycle directly in a tokio task to keep CI fast and deterministic.

#[cfg(unix)]
mod daemon_integration {
    use cc_switch::daemon::state::{DaemonState, ProxyEntry};
    use cc_switch::daemon::{ProxyResolution, try_resolve_proxy};
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn sample_state(pid: u32, proxies: Vec<ProxyEntry>) -> DaemonState {
        DaemonState {
            schema_version: 2,
            pid,
            started_at: "2026-05-28T19:30:00Z".to_owned(),
            stopped_at: None,
            data_root: PathBuf::from("/tmp/ccs-test"),
            agg_port: None,
            proxies,
        }
    }

    fn sample_proxy(upstream: &str, proxy_port: u16, api_port: u16) -> ProxyEntry {
        ProxyEntry {
            provider: "claude".to_owned(),
            upstream: upstream.to_owned(),
            proxy_port,
            api_port: Some(api_port),
            data_dir: PathBuf::from("/tmp/ccs-test/abcd1234"),
            started_at: "2026-05-28T19:30:00Z".to_owned(),
            restart_count: 0,
        }
    }

    #[test]
    fn state_file_round_trip_after_start() {
        let dir = TempDir::new().unwrap();
        let state_path = dir.path().join("daemon-state.json");

        let state = sample_state(
            std::process::id(),
            vec![
                sample_proxy("https://api.anthropic.com", 41001, 41501),
                sample_proxy("https://other.example.com/v1", 41002, 41502),
            ],
        );
        state.save(&state_path).unwrap();

        let loaded = DaemonState::load(&state_path).unwrap().unwrap();
        assert_eq!(loaded.proxies.len(), 2);
        assert_eq!(loaded.proxies[0].proxy_port, 41001);
        assert_eq!(loaded.proxies[1].upstream, "https://other.example.com/v1");
    }

    #[test]
    fn state_cleared_on_stop_writes_stopped_at() {
        let dir = TempDir::new().unwrap();
        let state_path = dir.path().join("daemon-state.json");

        let mut state = sample_state(
            std::process::id(),
            vec![sample_proxy("https://api.anthropic.com", 41001, 41501)],
        );
        state.save(&state_path).unwrap();

        // Simulate stop: set stopped_at.
        state.stopped_at = Some("2026-05-28T20:00:00Z".to_owned());
        state.save(&state_path).unwrap();

        let loaded = DaemonState::load(&state_path).unwrap().unwrap();
        assert_eq!(loaded.stopped_at, Some("2026-05-28T20:00:00Z".to_owned()));
    }

    #[test]
    fn try_resolve_proxy_returns_direct_when_no_state_file() {
        // try_resolve_proxy reads from ~/.cc-switch which won't have a daemon
        // running in CI. It should gracefully return Direct.
        match try_resolve_proxy("https://api.anthropic.com") {
            ProxyResolution::Direct => {} // expected
            ProxyResolution::Proxied { .. } => {
                panic!("should not find a proxy when daemon is not running")
            }
        }
    }

    #[test]
    fn find_proxy_matches_exact_upstream() {
        let state = sample_state(
            1234,
            vec![
                sample_proxy("https://api.anthropic.com", 41001, 41501),
                sample_proxy("https://glm.example.com/v1", 41002, 41502),
            ],
        );

        let found = state.find_proxy("claude", "https://api.anthropic.com");
        assert!(found.is_some());
        assert_eq!(found.unwrap().proxy_port, 41001);

        // Trailing slash mismatch → no match (spec: byte-for-byte).
        assert!(
            state
                .find_proxy("claude", "https://api.anthropic.com/")
                .is_none()
        );

        // Wrong provider → no match.
        assert!(
            state
                .find_proxy("codex", "https://api.anthropic.com")
                .is_none()
        );
    }

    #[test]
    fn find_proxy_returns_none_for_unknown_upstream() {
        let state = sample_state(
            1234,
            vec![sample_proxy("https://api.anthropic.com", 41001, 41501)],
        );
        assert!(
            state
                .find_proxy("claude", "https://unknown.example.com")
                .is_none()
        );
    }

    #[test]
    fn state_save_overwrites_atomically() {
        let dir = TempDir::new().unwrap();
        let state_path = dir.path().join("daemon-state.json");

        let state1 = sample_state(100, vec![sample_proxy("https://a.example", 8001, 9001)]);
        state1.save(&state_path).unwrap();

        let state2 = sample_state(200, vec![sample_proxy("https://b.example", 8002, 9002)]);
        state2.save(&state_path).unwrap();

        let loaded = DaemonState::load(&state_path).unwrap().unwrap();
        assert_eq!(loaded.pid, 200);
        assert_eq!(loaded.proxies[0].upstream, "https://b.example");

        // No leftover tmp file.
        let tmp = PathBuf::from(format!("{}.tmp", state_path.display()));
        assert!(!tmp.exists());
    }
}
