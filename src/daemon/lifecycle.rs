//! Daemon main loop: spawn proxies, write state, supervise, shutdown.

use crate::config::ConfigStorage;
use crate::daemon::pidfile::Pidfile;
use crate::daemon::state::{DaemonState, ProxyEntry};
use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::path::PathBuf;
use url::Url;

pub type Upstream = (String, String);

pub struct LifecycleConfig {
    pub state_path: PathBuf,
    pub pidfile_path: PathBuf,
    pub data_root: PathBuf,
    pub upstreams: Vec<Upstream>,
}

impl LifecycleConfig {
    pub fn from_storage(storage: &ConfigStorage) -> Result<Self> {
        let home = dirs::home_dir().context("could not find home directory")?;
        let cc_switch_dir = home.join(".cc-switch");
        std::fs::create_dir_all(&cc_switch_dir)
            .with_context(|| format!("failed to create {}", cc_switch_dir.display()))?;

        let upstreams = dedupe_upstreams(storage);

        Ok(Self {
            state_path: cc_switch_dir.join("daemon-state.json"),
            pidfile_path: cc_switch_dir.join("daemon.pid"),
            data_root: cc_switch_dir.join("daemon-data"),
            upstreams,
        })
    }
}

fn dedupe_upstreams(storage: &ConfigStorage) -> Vec<Upstream> {
    let mut seen = BTreeSet::new();
    let mut result = Vec::new();
    for config in storage.configurations.values() {
        if config.url.is_empty() {
            continue;
        }
        let key = ("claude".to_string(), config.url.clone());
        if seen.insert(key.clone()) {
            result.push(key);
        }
    }
    result
}

fn upstream_hash(url: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    url.hash(&mut hasher);
    format!("{:08x}", hasher.finish() & 0xFFFF_FFFF)
}

pub fn run_daemon_blocking(cfg: LifecycleConfig) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to build tokio runtime")?;

    rt.block_on(run_daemon_async(cfg))
}

async fn run_daemon_async(cfg: LifecycleConfig) -> Result<()> {
    let pidfile = Pidfile::new(cfg.pidfile_path.clone());
    pidfile
        .acquire()
        .context("failed to acquire pidfile — is another daemon already running?")?;

    std::fs::create_dir_all(&cfg.data_root)
        .with_context(|| format!("failed to create data_root {}", cfg.data_root.display()))?;

    let mut handles: Vec<ccs_proxy::ProxyHandle> = Vec::new();
    let mut proxy_entries: Vec<ProxyEntry> = Vec::new();

    for (_provider, upstream_url) in &cfg.upstreams {
        let parsed_url = match Url::parse(upstream_url) {
            Ok(u) => u,
            Err(err) => {
                eprintln!("warning: skipping upstream {upstream_url}: invalid URL: {err}");
                continue;
            }
        };

        let hash = upstream_hash(upstream_url);
        let data_dir = cfg.data_root.join(&hash);

        let serve_cfg = ccs_proxy::ServeConfig::new(
            ccs_proxy::ProviderKind::Claude,
            parsed_url,
            data_dir.clone(),
        );

        match ccs_proxy::serve(serve_cfg).await {
            Ok(handle) => {
                proxy_entries.push(ProxyEntry {
                    provider: "claude".to_string(),
                    upstream: upstream_url.clone(),
                    proxy_port: handle.proxy_port,
                    api_port: handle.api_port,
                    data_dir,
                    started_at: chrono::Utc::now().to_rfc3339(),
                    restart_count: 0,
                });
                handles.push(handle);
            }
            Err(err) => {
                eprintln!("warning: failed to start proxy for {upstream_url}: {err}");
            }
        }
    }

    let state = DaemonState {
        schema_version: 1,
        pid: std::process::id(),
        started_at: chrono::Utc::now().to_rfc3339(),
        stopped_at: None,
        data_root: cfg.data_root.clone(),
        proxies: proxy_entries.clone(),
    };
    state
        .save(&cfg.state_path)
        .context("failed to write initial daemon state")?;

    eprintln!(
        "ccs-daemon: started (pid {}), {} proxies active",
        state.pid,
        handles.len()
    );

    // Wait for shutdown signal.
    let shutdown = async {
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler");
        let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
            .expect("failed to install SIGINT handler");
        tokio::select! {
            _ = sigterm.recv() => {}
            _ = sigint.recv() => {}
        }
    };

    // Supervisor loop: check handle health every 30s, respawn if needed.
    let supervisor = supervisor_loop(&mut handles, &mut proxy_entries, &cfg);

    tokio::select! {
        _ = shutdown => {
            eprintln!("ccs-daemon: shutting down...");
        }
        _ = supervisor => {
            // supervisor_loop runs forever unless cancelled
        }
    }

    // Graceful shutdown: drop handles (triggers axum shutdown).
    for handle in handles {
        handle.shutdown().await;
    }

    // Write final state with stopped_at.
    let final_state = DaemonState {
        schema_version: 1,
        pid: std::process::id(),
        started_at: state.started_at,
        stopped_at: Some(chrono::Utc::now().to_rfc3339()),
        data_root: cfg.data_root,
        proxies: proxy_entries,
    };
    let _ = final_state.save(&cfg.state_path);

    // Remove pidfile.
    let _ = pidfile.release();

    eprintln!("ccs-daemon: stopped");
    Ok(())
}

async fn supervisor_loop(
    handles: &mut [ccs_proxy::ProxyHandle],
    entries: &mut [ProxyEntry],
    cfg: &LifecycleConfig,
) {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;

        for i in 0..handles.len() {
            let finished = handles[i].is_finished();

            if finished {
                eprintln!(
                    "ccs-daemon: proxy for {} exited unexpectedly, respawning...",
                    entries[i].upstream
                );

                let parsed_url = match Url::parse(&entries[i].upstream) {
                    Ok(u) => u,
                    Err(_) => continue,
                };

                let serve_cfg = ccs_proxy::ServeConfig::new(
                    ccs_proxy::ProviderKind::Claude,
                    parsed_url,
                    entries[i].data_dir.clone(),
                );

                match ccs_proxy::serve(serve_cfg).await {
                    Ok(new_handle) => {
                        entries[i].proxy_port = new_handle.proxy_port;
                        entries[i].api_port = new_handle.api_port;
                        entries[i].restart_count += 1;
                        entries[i].started_at = chrono::Utc::now().to_rfc3339();
                        handles[i] = new_handle;

                        let state = DaemonState {
                            schema_version: 1,
                            pid: std::process::id(),
                            started_at: entries.first().map_or_else(
                                || chrono::Utc::now().to_rfc3339(),
                                |e| e.started_at.clone(),
                            ),
                            stopped_at: None,
                            data_root: cfg.data_root.clone(),
                            proxies: entries.to_vec(),
                        };
                        let _ = state.save(&cfg.state_path);
                    }
                    Err(err) => {
                        eprintln!(
                            "ccs-daemon: failed to respawn proxy for {}: {err}",
                            entries[i].upstream
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types::Configuration;
    use std::collections::BTreeMap;

    fn make_storage(urls: &[&str]) -> ConfigStorage {
        let mut configurations = BTreeMap::new();
        for (i, url) in urls.iter().enumerate() {
            let alias = format!("alias{i}");
            configurations.insert(
                alias.clone(),
                Configuration {
                    alias_name: alias,
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
                },
            );
        }
        ConfigStorage {
            configurations,
            claude_settings_dir: None,
            default_storage_mode: None,
            codex_configurations: None,
        }
    }

    #[test]
    fn dedupe_upstreams_removes_duplicates() {
        let storage = make_storage(&[
            "https://api.anthropic.com",
            "https://api.anthropic.com",
            "https://other.example.com/v1",
        ]);
        let result = dedupe_upstreams(&storage);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].1, "https://api.anthropic.com");
        assert_eq!(result[1].1, "https://other.example.com/v1");
    }

    #[test]
    fn dedupe_upstreams_skips_empty_urls() {
        let storage = make_storage(&["", "https://api.anthropic.com"]);
        let result = dedupe_upstreams(&storage);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, "https://api.anthropic.com");
    }

    #[test]
    fn upstream_hash_is_deterministic() {
        let h1 = upstream_hash("https://api.anthropic.com");
        let h2 = upstream_hash("https://api.anthropic.com");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 8);
    }

    #[test]
    fn upstream_hash_differs_for_different_urls() {
        let h1 = upstream_hash("https://api.anthropic.com");
        let h2 = upstream_hash("https://other.example.com");
        assert_ne!(h1, h2);
    }
}
