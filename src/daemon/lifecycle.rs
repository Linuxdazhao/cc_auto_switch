//! Daemon main loop: spawn proxies, write state, supervise, shutdown.

use crate::config::ConfigStorage;
use crate::daemon::aggregate;
use crate::daemon::aggregate::state::AliasMap;
use crate::daemon::pidfile::Pidfile;
use crate::daemon::state::{DaemonState, ProxyEntry};
use anyhow::{Context, Result};
use ccs_proxy::store::FsStore;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;
use url::Url;

pub type Upstream = (String, String);

pub struct LifecycleConfig {
    pub state_path: PathBuf,
    pub pidfile_path: PathBuf,
    pub data_root: PathBuf,
    pub upstreams: Vec<Upstream>,
    pub foreground: bool,
}

impl LifecycleConfig {
    pub fn from_storage(
        storage: &ConfigStorage,
        foreground: bool,
        capture_official: bool,
    ) -> Result<Self> {
        let home = dirs::home_dir().context("could not find home directory")?;
        let cc_switch_dir = home.join(".cc-switch");
        std::fs::create_dir_all(&cc_switch_dir)
            .with_context(|| format!("failed to create {}", cc_switch_dir.display()))?;

        let upstreams = dedupe_upstreams(storage, capture_official);

        Ok(Self {
            state_path: cc_switch_dir.join("daemon-state.json"),
            pidfile_path: cc_switch_dir.join("daemon.pid"),
            data_root: cc_switch_dir.join("daemon-data"),
            upstreams,
            foreground,
        })
    }
}

fn dedupe_upstreams(storage: &ConfigStorage, capture_official: bool) -> Vec<Upstream> {
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
    // The implicit official Anthropic upstream is opt-in: only spawn a proxy
    // for `cc use official` when the daemon was started with
    // `--capture-official`. By default official traffic flows direct to
    // Anthropic. A user-defined alias that points at the official URL is
    // handled above and deduped here, so it always gets its proxy.
    if capture_official {
        let official = (
            "claude".to_string(),
            crate::daemon::OFFICIAL_UPSTREAM.to_string(),
        );
        if seen.insert(official.clone()) {
            result.push(official);
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

pub fn run_daemon_blocking(
    cfg: LifecycleConfig,
    log_level: Option<String>,
    verbose: u8,
) -> Result<()> {
    let env_val = std::env::var("CCS_LOG").ok();
    let level = crate::daemon::logging::resolve_log_level(
        log_level.as_deref(),
        verbose,
        env_val.as_deref(),
    );
    let mode = if cfg.foreground {
        crate::daemon::logging::LogMode::Foreground
    } else {
        crate::daemon::logging::LogMode::Background
    };

    crate::daemon::logging::cleanup_old_logs(7);
    let _guard = crate::daemon::logging::init_tracing(mode, level);

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
                tracing::warn!(upstream = %upstream_url, error = %err, "skipping invalid upstream URL");
                continue;
            }
        };

        let hash = upstream_hash(upstream_url);
        let data_dir = cfg.data_root.join(&hash);

        let mut serve_cfg = ccs_proxy::ServeConfig::new(
            ccs_proxy::ProviderKind::Claude,
            parsed_url,
            data_dir.clone(),
        );
        serve_cfg.api_server = false;

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
                tracing::error!(upstream = %upstream_url, error = %err, "failed to start proxy");
            }
        }
    }

    // Build AliasMap from current storage
    let storage = ConfigStorage::load().unwrap_or_default();
    let alias_map = Arc::new(AliasMap::from_storage(&storage));

    // Collect stores and event senders for aggregate
    let agg_stores: Vec<_> = proxy_entries
        .iter()
        .map(|entry| {
            let store = Arc::new(
                FsStore::open(entry.data_dir.clone())
                    .expect("store open should succeed — dir already created by proxy"),
            );
            (entry.upstream.clone(), store)
        })
        .collect();

    let agg_events: Vec<_> = handles
        .iter()
        .zip(proxy_entries.iter())
        .map(|(handle, entry)| (entry.upstream.clone(), handle.event_sender().clone()))
        .collect();

    // Start aggregate server (hold handle alive for daemon lifetime)
    let agg_handle = match aggregate::serve(agg_stores, agg_events, alias_map, 0).await {
        Ok(handle) => {
            tracing::info!(port = handle.port, "aggregate dashboard available");
            Some(handle)
        }
        Err(err) => {
            tracing::warn!(error = %err, "failed to start aggregate server — proxies still work");
            None
        }
    };
    let agg_port = agg_handle.as_ref().map(|h| h.port);

    let state = DaemonState {
        schema_version: 2,
        version: crate::daemon::state::CURRENT_VERSION.to_string(),
        pid: std::process::id(),
        started_at: chrono::Utc::now().to_rfc3339(),
        stopped_at: None,
        data_root: cfg.data_root.clone(),
        agg_port,
        proxies: proxy_entries.clone(),
    };
    state
        .save(&cfg.state_path)
        .context("failed to write initial daemon state")?;

    tracing::info!(
        pid = state.pid,
        proxy_count = handles.len(),
        "daemon started"
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
            tracing::info!("daemon shutting down");
        }
        _ = supervisor => {
            // supervisor_loop runs forever unless cancelled
        }
    }

    // Graceful shutdown: stop aggregate and proxy servers.
    if let Some(agg) = agg_handle {
        agg.shutdown().await;
    }
    for handle in handles {
        handle.shutdown().await;
    }

    // Write final state with stopped_at.
    let final_state = DaemonState {
        schema_version: 2,
        version: crate::daemon::state::CURRENT_VERSION.to_string(),
        pid: std::process::id(),
        started_at: state.started_at,
        stopped_at: Some(chrono::Utc::now().to_rfc3339()),
        data_root: cfg.data_root,
        agg_port: None,
        proxies: proxy_entries,
    };
    let _ = final_state.save(&cfg.state_path);

    // Remove pidfile.
    let _ = pidfile.release();

    tracing::info!("daemon stopped");
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
                tracing::warn!(upstream = %entries[i].upstream, "proxy exited unexpectedly, respawning");

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
                            schema_version: 2,
                            version: crate::daemon::state::CURRENT_VERSION.to_string(),
                            pid: std::process::id(),
                            started_at: entries.first().map_or_else(
                                || chrono::Utc::now().to_rfc3339(),
                                |e| e.started_at.clone(),
                            ),
                            stopped_at: None,
                            data_root: cfg.data_root.clone(),
                            agg_port: None,
                            proxies: entries.to_vec(),
                        };
                        let _ = state.save(&cfg.state_path);
                    }
                    Err(err) => {
                        tracing::error!(upstream = %entries[i].upstream, error = %err, "failed to respawn proxy");
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
        let result = dedupe_upstreams(&storage, false);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].1, "https://api.anthropic.com");
        assert_eq!(result[1].1, "https://other.example.com/v1");
    }

    #[test]
    fn dedupe_upstreams_skips_empty_urls() {
        let storage = make_storage(&["", "https://api.anthropic.com"]);
        let result = dedupe_upstreams(&storage, false);
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

    #[test]
    fn dedupe_upstreams_excludes_official_by_default() {
        // Default: do NOT spawn the implicit official proxy. `cc use official`
        // traffic flows direct to Anthropic unless capture is explicitly enabled.
        let result = dedupe_upstreams(&make_storage(&[]), false);
        assert!(
            !result.contains(&(
                "claude".to_string(),
                crate::daemon::OFFICIAL_UPSTREAM.to_string()
            )),
            "OFFICIAL_UPSTREAM must be absent by default, got {result:?}",
        );
    }

    #[test]
    fn dedupe_upstreams_includes_official_when_capture_enabled() {
        let result = dedupe_upstreams(&make_storage(&[]), true);
        assert!(
            result.contains(&(
                "claude".to_string(),
                crate::daemon::OFFICIAL_UPSTREAM.to_string()
            )),
            "OFFICIAL_UPSTREAM must be present when capture_official=true, got {result:?}",
        );
    }

    #[test]
    fn dedupe_upstreams_dedupes_when_user_has_official_url() {
        // Belt-and-suspenders: user shouldn't normally do this, but if they
        // configure an alias with the official URL, we must not spawn two
        // proxies for the same URL even with capture enabled.
        let result = dedupe_upstreams(&make_storage(&[crate::daemon::OFFICIAL_UPSTREAM]), true);
        let count = result
            .iter()
            .filter(|(_, url)| url == crate::daemon::OFFICIAL_UPSTREAM)
            .count();
        assert_eq!(
            count, 1,
            "OFFICIAL_UPSTREAM must appear exactly once, got {result:?}"
        );
    }

    #[test]
    fn dedupe_upstreams_keeps_user_official_alias_without_capture() {
        // A user-defined alias explicitly pointing at the official URL still
        // gets its proxy even when implicit official capture is off.
        let result = dedupe_upstreams(&make_storage(&[crate::daemon::OFFICIAL_UPSTREAM]), false);
        let count = result
            .iter()
            .filter(|(_, url)| url == crate::daemon::OFFICIAL_UPSTREAM)
            .count();
        assert_eq!(
            count, 1,
            "user-defined official alias must still spawn its proxy, got {result:?}"
        );
    }
}
