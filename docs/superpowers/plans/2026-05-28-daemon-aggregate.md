# Daemon Aggregation Layer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a unified cross-upstream observability dashboard + API inside the daemon process, merging data from all proxies into a single view.

**Architecture:** A new `aggregate` module inside `src/daemon/` runs an axum server (one port) that subscribes to all proxy broadcast channels for real-time events, scans all proxy `FsStore` directories for historical data, and serves a combined dashboard. The existing per-proxy API server is disabled in daemon mode via a new `api_server: bool` field on `ServeConfig`.

**Tech Stack:** Rust (axum 0.8, tokio, broadcast channels, rust-embed 8), vanilla JS/CSS dashboard

---

### Task 1: Add `api_server` field to `ServeConfig` and make API server conditional

**Files:**
- Modify: `ccs-proxy/src/config.rs`
- Modify: `ccs-proxy/src/lib.rs`
- Test: `ccs-proxy/tests/e2e_serve.rs` (verify existing tests still pass)

- [ ] **Step 1: Write a test that verifies `api_server=false` skips the API listener**

Create a new integration test file:

```rust
// ccs-proxy/tests/api_server_disabled.rs
use ccs_proxy::{ProviderKind, ServeConfig};
use tempfile::TempDir;
use url::Url;

#[tokio::test]
async fn serve_with_api_server_false_returns_none_api_port() {
    let tmp = TempDir::new().unwrap();
    let mut cfg = ServeConfig::new(
        ProviderKind::Claude,
        Url::parse("https://api.anthropic.com").unwrap(),
        tmp.path().to_path_buf(),
    );
    cfg.api_server = false;

    let handle = ccs_proxy::serve(cfg).await.unwrap();
    assert!(handle.api_port.is_none(), "api_port should be None when api_server=false");
    assert!(handle.proxy_port > 0);
    handle.shutdown().await;
}
```

- [ ] **Step 2: Run test to confirm it fails**

Run: `cargo test --package ccs-proxy --test api_server_disabled -- --nocapture`
Expected: Compilation error — `api_server` field doesn't exist, `api_port` is `u16` not `Option`.

- [ ] **Step 3: Add `api_server` to `ServeConfig`**

In `ccs-proxy/src/config.rs`:

```rust
#[derive(Debug, Clone)]
pub struct ServeConfig {
    pub provider: ProviderKind,
    pub upstream: Url,
    pub proxy_port: u16,
    pub api_port: u16,
    pub data_dir: PathBuf,
    pub redact: bool,
    pub cors_allow: Option<String>,
    pub api_server: bool,  // NEW
}

impl ServeConfig {
    pub fn new(provider: ProviderKind, upstream: Url, data_dir: PathBuf) -> Self {
        Self {
            provider,
            upstream,
            proxy_port: 0,
            api_port: 0,
            data_dir,
            redact: true,
            cors_allow: None,
            api_server: true,  // default: start API server
        }
    }
}
```

- [ ] **Step 4: Change `ProxyHandle.api_port` to `Option<u16>`**

In `ccs-proxy/src/handle.rs`:

```rust
pub struct ProxyHandle {
    pub provider: ProviderKind,
    pub upstream: Url,
    pub proxy_port: u16,
    pub api_port: Option<u16>,  // CHANGED from u16
    pub(crate) shutdown_tx: Option<oneshot::Sender<()>>,
    pub(crate) join: Option<JoinHandle<()>>,
}
```

- [ ] **Step 5: Modify `ccs_proxy::serve()` to conditionally start the API server**

In `ccs-proxy/src/lib.rs`, change the `serve` function to conditionally bind and start the API listener:

```rust
pub async fn serve(cfg: ServeConfig) -> Result<ProxyHandle, ServeError> {
    if !cfg!(unix) {
        return Err(ServeError::UnsupportedPlatform(
            "only unix (macOS / Linux) is supported in v1",
        ));
    }

    std::fs::create_dir_all(&cfg.data_dir).map_err(|err| ServeError::DataDir {
        path: cfg.data_dir.clone(),
        source: err,
    })?;

    let store: Arc<dyn Store> = Arc::new(
        store::FsStore::open(cfg.data_dir.clone())
            .map_err(|err| ServeError::Internal(anyhow::Error::from(err)))?,
    );
    let session_id = SessionId::new();

    let proxy_listener = TcpListener::bind(("127.0.0.1", cfg.proxy_port))
        .await
        .map_err(ServeError::BindProxy)?;
    let proxy_addr = proxy_listener.local_addr().map_err(ServeError::BindProxy)?;

    let (api_listener, api_addr) = if cfg.api_server {
        let listener = TcpListener::bind(("127.0.0.1", cfg.api_port))
            .await
            .map_err(ServeError::BindApi)?;
        let addr = listener.local_addr().map_err(ServeError::BindApi)?;
        (Some(listener), Some(addr))
    } else {
        (None, None)
    };

    let meta = store::SessionMeta {
        session_id: session_id.to_string(),
        provider: cfg.provider.as_str().into(),
        upstream: cfg.upstream.to_string(),
        proxy_port: proxy_addr.port(),
        api_port: api_addr.map_or(0, |a| a.port()),
        started_at: chrono::Utc::now(),
        ended_at: None,
        request_count: 0,
        schema_version: 1,
    };
    if let Err(err) = store.init_session(meta).await {
        tracing::warn!(error = %err, "failed to persist initial session metadata");
    }

    let state = AppState::new(
        store.clone(),
        cfg.provider,
        cfg.upstream.clone(),
        session_id.clone(),
        cfg.redact,
    );

    let proxy_app = proxy::build_proxy_app(state.clone());

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let join = spawn_servers(
        proxy_listener,
        api_listener,
        proxy_app,
        state,
        shutdown_rx,
        store,
        session_id,
    );

    Ok(ProxyHandle {
        provider: cfg.provider,
        upstream: cfg.upstream,
        proxy_port: proxy_addr.port(),
        api_port: api_addr.map(|a| a.port()),
        shutdown_tx: Some(shutdown_tx),
        join: Some(join),
    })
}

fn spawn_servers(
    proxy_listener: TcpListener,
    api_listener: Option<TcpListener>,
    proxy_app: axum::Router,
    state: AppState,
    shutdown_rx: oneshot::Receiver<()>,
    store: Arc<dyn Store>,
    session_id: SessionId,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let proxy_fut = axum::serve(proxy_listener, proxy_app);

        if let Some(api_listener) = api_listener {
            let api_app = api::build_api_app(state);
            let api_fut = axum::serve(api_listener, api_app);
            tokio::select! {
                res = proxy_fut => {
                    if let Err(err) = res {
                        tracing::warn!(error = %err, "proxy server exited");
                    }
                }
                res = api_fut => {
                    if let Err(err) = res {
                        tracing::warn!(error = %err, "api server exited");
                    }
                }
                _ = shutdown_rx => {}
            }
        } else {
            tokio::select! {
                res = proxy_fut => {
                    if let Err(err) = res {
                        tracing::warn!(error = %err, "proxy server exited");
                    }
                }
                _ = shutdown_rx => {}
            }
        }

        if let Err(err) = store.finalize_session(session_id.as_str()).await {
            tracing::warn!(error = %err, "failed to finalize session");
        }
    })
}
```

- [ ] **Step 6: Fix all compilation errors from `api_port` type change**

In `src/daemon/lifecycle.rs`, change `entries[i].api_port = new_handle.api_port;` and the initial `ProxyEntry` construction to use `handle.api_port.unwrap_or(0)` (or change `ProxyEntry.api_port` to `Option<u16>` — see Task 3).

In `src/daemon/status.rs`, change `probe_health(entry.api_port)` to handle `Option`.

In `ccs-proxy/src/bin/ccs-proxy.rs`, adjust if needed (the standalone binary always has `api_server: true`, so unwrap is safe there).

- [ ] **Step 7: Run all tests**

Run: `cargo test --workspace`
Expected: All pass.

- [ ] **Step 8: Commit**

```bash
git add ccs-proxy/src/config.rs ccs-proxy/src/handle.rs ccs-proxy/src/lib.rs ccs-proxy/tests/api_server_disabled.rs
git commit -m "feat(ccs-proxy): add api_server flag to ServeConfig, make api_port optional"
```

---

### Task 2: Add `subscribe_events()` and `event_sender()` to `ProxyHandle`

**Files:**
- Modify: `ccs-proxy/src/handle.rs`
- Modify: `ccs-proxy/src/lib.rs` (pass events sender into handle)
- Modify: `ccs-proxy/src/state.rs` (re-export `CaptureEvent` at crate root)
- Test: `ccs-proxy/tests/api_stream.rs` (existing test uses events — verify still works)

- [ ] **Step 1: Write a test for event subscription**

```rust
// ccs-proxy/tests/subscribe_events.rs
use ccs_proxy::{ProviderKind, ServeConfig};
use tempfile::TempDir;
use url::Url;

#[tokio::test]
async fn subscribe_events_receives_broadcasts() {
    let tmp = TempDir::new().unwrap();
    let cfg = ServeConfig::new(
        ProviderKind::Claude,
        Url::parse("https://api.anthropic.com").unwrap(),
        tmp.path().to_path_buf(),
    );
    let handle = ccs_proxy::serve(cfg).await.unwrap();

    let mut rx = handle.subscribe_events();

    // Send a request to the proxy to trigger a CaptureEvent
    let proxy_url = format!("http://127.0.0.1:{}/v1/messages", handle.proxy_port);
    let client = reqwest::Client::new();
    let _ = client
        .post(&proxy_url)
        .header("x-api-key", "test")
        .header("anthropic-version", "2023-06-01")
        .json(&serde_json::json!({"model": "claude-sonnet-4-6", "max_tokens": 1, "messages": []}))
        .send()
        .await;

    // We should receive at least a RequestStarted event
    let event = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        rx.recv(),
    ).await;
    assert!(event.is_ok(), "should receive an event within timeout");

    handle.shutdown().await;
}
```

- [ ] **Step 2: Run test to confirm it fails**

Run: `cargo test --package ccs-proxy --test subscribe_events -- --nocapture`
Expected: Compilation error — `subscribe_events` method doesn't exist.

- [ ] **Step 3: Add event sender to `ProxyHandle` and public methods**

In `ccs-proxy/src/handle.rs`:

```rust
use crate::capture::CaptureEvent;
use crate::provider::ProviderKind;
use tokio::sync::{broadcast, oneshot};
use tokio::task::JoinHandle;
use url::Url;

pub struct ProxyHandle {
    pub provider: ProviderKind,
    pub upstream: Url,
    pub proxy_port: u16,
    pub api_port: Option<u16>,
    pub(crate) shutdown_tx: Option<oneshot::Sender<()>>,
    pub(crate) join: Option<JoinHandle<()>>,
    pub(crate) events: broadcast::Sender<CaptureEvent>,
}

impl ProxyHandle {
    pub fn subscribe_events(&self) -> broadcast::Receiver<CaptureEvent> {
        self.events.subscribe()
    }

    pub fn event_sender(&self) -> &broadcast::Sender<CaptureEvent> {
        &self.events
    }

    pub fn is_finished(&self) -> bool {
        self.join.as_ref().is_some_and(|j| j.is_finished())
    }

    pub async fn shutdown(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(join) = self.join.take() {
            let _ = join.await;
        }
    }
}

impl Drop for ProxyHandle {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}
```

- [ ] **Step 4: Pass the events sender from `AppState` into `ProxyHandle` in `serve()`**

In `ccs-proxy/src/lib.rs`, after constructing `state`, clone `state.events` and include it in the `ProxyHandle`:

```rust
    Ok(ProxyHandle {
        provider: cfg.provider,
        upstream: cfg.upstream,
        proxy_port: proxy_addr.port(),
        api_port: api_addr.map(|a| a.port()),
        shutdown_tx: Some(shutdown_tx),
        join: Some(join),
        events: state.events.clone(),
    })
```

- [ ] **Step 5: Re-export `CaptureEvent` from crate root**

In `ccs-proxy/src/lib.rs`, add:

```rust
pub use capture::CaptureEvent;
```

- [ ] **Step 6: Run tests**

Run: `cargo test --workspace`
Expected: All pass.

- [ ] **Step 7: Commit**

```bash
git add ccs-proxy/src/handle.rs ccs-proxy/src/lib.rs ccs-proxy/tests/subscribe_events.rs
git commit -m "feat(ccs-proxy): expose subscribe_events() and event_sender() on ProxyHandle"
```

---

### Task 3: Update `DaemonState` and `ProxyEntry` for aggregation

**Files:**
- Modify: `src/daemon/state.rs`
- Modify: `tests/daemon_integration.rs`

- [ ] **Step 1: Write a test for schema_version 2 with `agg_port`**

Add to `src/daemon/state.rs` test module:

```rust
#[test]
fn state_v2_with_agg_port_round_trips() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("state.json");
    let state = DaemonState {
        schema_version: 2,
        pid: 1234,
        started_at: "2026-05-28T00:00:00Z".to_owned(),
        stopped_at: None,
        data_root: PathBuf::from("/tmp/ccs"),
        agg_port: Some(41600),
        proxies: vec![ProxyEntry {
            provider: "claude".to_string(),
            upstream: "https://api.anthropic.com".to_string(),
            proxy_port: 41001,
            api_port: None,
            data_dir: PathBuf::from("/tmp/ccs/8f3a2c1e"),
            started_at: "2026-05-28T00:00:00Z".to_string(),
            restart_count: 0,
        }],
    };
    state.save(&path).unwrap();
    let loaded = DaemonState::load(&path).unwrap().expect("file exists");
    assert_eq!(loaded.schema_version, 2);
    assert_eq!(loaded.agg_port, Some(41600));
    assert_eq!(loaded.proxies[0].api_port, None);
}
```

- [ ] **Step 2: Run test to confirm it fails**

Run: `cargo test state_v2_with_agg_port_round_trips`
Expected: Compilation error — `agg_port` and `Option<u16>` for `api_port` don't exist.

- [ ] **Step 3: Update `DaemonState` and `ProxyEntry` structs**

In `src/daemon/state.rs`:

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ProxyEntry {
    pub provider: String,
    pub upstream: String,
    pub proxy_port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_port: Option<u16>,  // CHANGED: None in daemon mode
    pub data_dir: PathBuf,
    pub started_at: String,
    pub restart_count: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DaemonState {
    pub schema_version: u32,
    pub pid: u32,
    pub started_at: String,
    pub stopped_at: Option<String>,
    pub data_root: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agg_port: Option<u16>,  // NEW
    pub proxies: Vec<ProxyEntry>,
}
```

- [ ] **Step 4: Fix all references to `ProxyEntry.api_port` and `DaemonState`**

In `src/daemon/lifecycle.rs`:
- Change `ProxyEntry` construction: `api_port: handle.api_port,` (already `Option<u16>` from Task 1).
- Add `agg_port: None,` to `DaemonState` construction (aggregate server added in Task 6).

In `src/daemon/status.rs`:
- Change `probe_health(entry.api_port)` to `probe_health(entry.api_port)` where the function signature changes to accept `Option<u16>`:

```rust
fn probe_health(api_port: Option<u16>) -> HealthProbe {
    let Some(port) = api_port else {
        return HealthProbe { reachable: false, request_count: None, store_degraded: false };
    };
    let url = format!("http://127.0.0.1:{port}/api/health");
    // ... rest unchanged
}
```

In `tests/daemon_integration.rs`:
- Update `sample_proxy` helper to use `api_port: Some(9000)`.
- Update `sample_state` helper to include `agg_port: None`.

- [ ] **Step 5: Run tests**

Run: `cargo test --workspace`
Expected: All pass.

- [ ] **Step 6: Commit**

```bash
git add src/daemon/state.rs src/daemon/lifecycle.rs src/daemon/status.rs tests/daemon_integration.rs
git commit -m "feat(daemon): bump state schema to v2, add agg_port, make api_port optional"
```

---

### Task 4: Implement `AliasMap`

**Files:**
- Create: `src/daemon/aggregate/mod.rs`
- Create: `src/daemon/aggregate/state.rs`
- Modify: `src/daemon/mod.rs`

- [ ] **Step 1: Write unit tests for AliasMap**

```rust
// src/daemon/aggregate/state.rs (bottom of file)

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types::Configuration;
    use crate::config::types::ConfigStorage;
    use std::collections::BTreeMap;

    fn make_storage(entries: &[(&str, &str)]) -> ConfigStorage {
        let mut configurations = BTreeMap::new();
        for (alias, url) in entries {
            configurations.insert(
                alias.to_string(),
                Configuration {
                    alias_name: alias.to_string(),
                    token: "sk-test".to_string(),
                    url: url.to_string(),
                    ..Default::default()
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
    fn alias_map_groups_by_upstream() {
        let storage = make_storage(&[
            ("work", "https://api.anthropic.com"),
            ("personal", "https://api.anthropic.com"),
            ("other", "https://other.example.com"),
        ]);
        let map = AliasMap::from_storage(&storage);
        let mut aliases = map.aliases_for("https://api.anthropic.com");
        aliases.sort();
        assert_eq!(aliases, vec!["personal", "work"]);
        assert_eq!(map.aliases_for("https://other.example.com"), vec!["other"]);
    }

    #[test]
    fn alias_map_returns_empty_for_unknown() {
        let storage = make_storage(&[("work", "https://api.anthropic.com")]);
        let map = AliasMap::from_storage(&storage);
        assert!(map.aliases_for("https://unknown.example.com").is_empty());
    }
}
```

- [ ] **Step 2: Run test to confirm it fails**

Run: `cargo test alias_map_groups_by_upstream`
Expected: Module doesn't exist.

- [ ] **Step 3: Create the aggregate module and `AliasMap`**

Create `src/daemon/aggregate/mod.rs`:

```rust
pub mod state;
```

Create `src/daemon/aggregate/state.rs`:

```rust
use crate::config::types::ConfigStorage;
use std::collections::HashMap;

pub struct AliasMap {
    map: HashMap<String, Vec<String>>,
}

impl AliasMap {
    pub fn from_storage(storage: &ConfigStorage) -> Self {
        let mut map: HashMap<String, Vec<String>> = HashMap::new();
        for config in storage.configurations.values() {
            if !config.url.is_empty() {
                map.entry(config.url.clone())
                    .or_default()
                    .push(config.alias_name.clone());
            }
        }
        Self { map }
    }

    pub fn aliases_for(&self, upstream: &str) -> Vec<String> {
        self.map.get(upstream).cloned().unwrap_or_default()
    }
}
```

- [ ] **Step 4: Register the module**

In `src/daemon/mod.rs`, add:

```rust
pub mod aggregate;
```

- [ ] **Step 5: Run tests**

Run: `cargo test --lib alias_map`
Expected: All pass.

- [ ] **Step 6: Commit**

```bash
git add src/daemon/aggregate/mod.rs src/daemon/aggregate/state.rs src/daemon/mod.rs
git commit -m "feat(daemon): add aggregate module with AliasMap"
```

---

### Task 5: Implement `TaggedCaptureEvent` and the event merger task

**Files:**
- Create: `src/daemon/aggregate/stream.rs`
- Modify: `src/daemon/aggregate/mod.rs`

- [ ] **Step 1: Write unit test for event merger**

```rust
// src/daemon/aggregate/stream.rs (bottom of file)

#[cfg(test)]
mod tests {
    use super::*;
    use ccs_proxy::capture::CaptureEvent;
    use tokio::sync::broadcast;

    #[tokio::test]
    async fn merger_tags_events_with_upstream() {
        let (tx_a, _) = broadcast::channel::<CaptureEvent>(16);
        let (tx_b, _) = broadcast::channel::<CaptureEvent>(16);
        let (merged_tx, mut merged_rx) = broadcast::channel::<TaggedCaptureEvent>(64);

        let alias_map = Arc::new(AliasMap::from_entries(vec![
            ("https://a.example.com".to_string(), vec!["alias_a".to_string()]),
        ]));

        let proxy_events = vec![
            ("https://a.example.com".to_string(), tx_a.subscribe()),
            ("https://b.example.com".to_string(), tx_b.subscribe()),
        ];

        let _merger = tokio::spawn(event_merger(proxy_events, alias_map, merged_tx));

        // Give merger time to start
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        tx_a.send(CaptureEvent::RequestStarted {
            session_id: "sess1".to_string(),
            seq: 1,
            started_at: chrono::Utc::now(),
            model: Some("claude-sonnet-4-6".to_string()),
        }).unwrap();

        let tagged = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            merged_rx.recv(),
        ).await.unwrap().unwrap();

        assert_eq!(tagged.upstream, "https://a.example.com");
        assert_eq!(tagged.aliases, vec!["alias_a"]);
    }
}
```

- [ ] **Step 2: Run test to confirm it fails**

Run: `cargo test merger_tags_events`
Expected: Module doesn't exist.

- [ ] **Step 3: Implement `TaggedCaptureEvent` and `event_merger`**

Create `src/daemon/aggregate/stream.rs`:

```rust
use crate::daemon::aggregate::state::AliasMap;
use ccs_proxy::CaptureEvent;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize)]
pub struct TaggedCaptureEvent {
    pub upstream: String,
    pub aliases: Vec<String>,
    #[serde(flatten)]
    pub inner: CaptureEvent,
}

pub async fn event_merger(
    proxy_events: Vec<(String, broadcast::Receiver<CaptureEvent>)>,
    alias_map: Arc<AliasMap>,
    merged_tx: broadcast::Sender<TaggedCaptureEvent>,
) {
    use tokio_stream::StreamExt;
    use tokio_stream::wrappers::BroadcastStream;

    let streams: Vec<_> = proxy_events
        .into_iter()
        .map(|(upstream, rx)| {
            let upstream = upstream.clone();
            BroadcastStream::new(rx).filter_map(move |res| {
                res.ok().map(|ev| (upstream.clone(), ev))
            })
        })
        .collect();

    let mut merged = futures::stream::select_all(streams);

    while let Some((upstream, event)) = merged.next().await {
        let aliases = alias_map.aliases_for(&upstream);
        let tagged = TaggedCaptureEvent {
            upstream,
            aliases,
            inner: event,
        };
        let _ = merged_tx.send(tagged);
    }
}
```

- [ ] **Step 4: Add helper constructor to `AliasMap` for testing**

In `src/daemon/aggregate/state.rs`, add:

```rust
impl AliasMap {
    pub fn from_entries(entries: Vec<(String, Vec<String>)>) -> Self {
        Self {
            map: entries.into_iter().collect(),
        }
    }
}
```

- [ ] **Step 5: Update `src/daemon/aggregate/mod.rs`**

```rust
pub mod state;
pub mod stream;
```

- [ ] **Step 6: Add dependencies to workspace `Cargo.toml`**

In the root `Cargo.toml`, add `futures = "0.3"` and `tokio-stream = { version = "0.1", features = ["sync"] }` to `[dependencies]` (these are already dependencies of ccs-proxy, but the cc-switch binary needs them too for the aggregate module).

- [ ] **Step 7: Run tests**

Run: `cargo test --lib merger_tags_events`
Expected: Pass.

- [ ] **Step 8: Commit**

```bash
git add src/daemon/aggregate/stream.rs src/daemon/aggregate/state.rs src/daemon/aggregate/mod.rs Cargo.toml
git commit -m "feat(daemon): implement TaggedCaptureEvent and event merger task"
```

---

### Task 6: Implement aggregate API routes

**Files:**
- Create: `src/daemon/aggregate/routes.rs`
- Modify: `src/daemon/aggregate/mod.rs`
- Modify: `src/daemon/aggregate/state.rs`

- [ ] **Step 1: Define `AggregateState` shared state**

In `src/daemon/aggregate/state.rs`, add:

```rust
use ccs_proxy::store::FsStore;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::broadcast;
use super::stream::TaggedCaptureEvent;

pub struct AggregateState {
    pub stores: Vec<(String, Arc<FsStore>)>,
    pub merged_events: broadcast::Sender<TaggedCaptureEvent>,
    pub alias_map: Arc<AliasMap>,
    pub started_at: DateTime<Utc>,
}
```

- [ ] **Step 2: Write a test for `/api/health` aggregate endpoint**

```rust
// tests/daemon_aggregate.rs
#[cfg(unix)]
mod daemon_aggregate {
    use ccs_proxy::store::FsStore;
    use std::sync::Arc;
    use tempfile::TempDir;

    #[tokio::test]
    async fn aggregate_health_returns_proxy_count() {
        let tmp_a = TempDir::new().unwrap();
        let tmp_b = TempDir::new().unwrap();
        let store_a = Arc::new(FsStore::open(tmp_a.path().to_path_buf()).unwrap());
        let store_b = Arc::new(FsStore::open(tmp_b.path().to_path_buf()).unwrap());

        let stores = vec![
            ("https://a.example.com".to_string(), store_a),
            ("https://b.example.com".to_string(), store_b),
        ];

        let alias_map = Arc::new(
            cc_switch::daemon::aggregate::state::AliasMap::from_entries(vec![
                ("https://a.example.com".to_string(), vec!["alias_a".to_string()]),
            ]),
        );

        let handle = cc_switch::daemon::aggregate::serve(stores, vec![], alias_map, 0)
            .await
            .unwrap();

        let resp = reqwest::get(format!("http://127.0.0.1:{}/api/health", handle.port))
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["proxy_count"], 2);
        assert_eq!(body["status"], "ok");
    }
}
```

- [ ] **Step 3: Run test to confirm it fails**

Run: `cargo test --test daemon_aggregate -- --nocapture`
Expected: Module/function doesn't exist.

- [ ] **Step 4: Implement aggregate routes**

Create `src/daemon/aggregate/routes.rs`:

```rust
use super::state::AggregateState;
use super::stream::TaggedCaptureEvent;
use axum::extract::{Query, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::{Json, Router, routing::get};
use futures::stream::Stream;
use serde::Deserialize;
use serde_json::{Value, json};
use std::convert::Infallible;
use std::sync::Arc;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

type SharedState = Arc<AggregateState>;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/sessions", get(list_sessions))
        .route("/api/sessions/{sid}", get(get_session))
        .route("/api/requests/{sid}/{seq}", get(get_request))
        .route("/api/stats", get(stats))
        .route("/api/stream", get(stream))
}

#[derive(Deserialize, Default)]
struct SessionsQuery {
    upstream: Option<String>,
    alias: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
}

#[derive(Deserialize, Default)]
struct StatsQuery {
    since: Option<String>,
}

async fn health(State(state): State<SharedState>) -> Json<Value> {
    let uptime_s = (chrono::Utc::now() - state.started_at).num_seconds();
    let mut total_requests: u64 = 0;
    let mut proxies = Vec::new();

    for (upstream, store) in &state.stores {
        let sessions = store.list_sessions().await.unwrap_or_default();
        let req_count: u64 = sessions.iter().map(|s| s.request_count).sum();
        total_requests += req_count;
        let aliases = state.alias_map.aliases_for(upstream);
        proxies.push(json!({
            "upstream": upstream,
            "request_count": req_count,
            "aliases": aliases,
        }));
    }

    Json(json!({
        "status": "ok",
        "pid": std::process::id(),
        "uptime_s": uptime_s,
        "proxy_count": state.stores.len(),
        "agg_port": state.started_at, // will be set by caller
        "total_requests": total_requests,
        "proxies": proxies,
    }))
}

async fn list_sessions(
    State(state): State<SharedState>,
    Query(params): Query<SessionsQuery>,
) -> Json<Value> {
    let mut all_sessions = Vec::new();

    for (upstream, store) in &state.stores {
        let sessions = store.list_sessions().await.unwrap_or_default();
        let aliases = state.alias_map.aliases_for(upstream);
        for session in sessions {
            all_sessions.push(json!({
                "upstream": upstream,
                "aliases": aliases,
                "session_id": session.session_id,
                "provider": session.provider,
                "started_at": session.started_at,
                "ended_at": session.ended_at,
                "request_count": session.request_count,
            }));
        }
    }

    // Sort by started_at desc
    all_sessions.sort_by(|a, b| {
        let a_time = a["started_at"].as_str().unwrap_or("");
        let b_time = b["started_at"].as_str().unwrap_or("");
        b_time.cmp(a_time)
    });

    // Filter by upstream
    if let Some(ref upstream_filter) = params.upstream {
        all_sessions.retain(|s| s["upstream"].as_str() == Some(upstream_filter.as_str()));
    }

    // Filter by alias
    if let Some(ref alias_filter) = params.alias {
        all_sessions.retain(|s| {
            s["aliases"]
                .as_array()
                .is_some_and(|arr| arr.iter().any(|a| a.as_str() == Some(alias_filter.as_str())))
        });
    }

    // Pagination
    let offset = params.offset.unwrap_or(0);
    let limit = params.limit.unwrap_or(100);
    let paginated: Vec<_> = all_sessions.into_iter().skip(offset).take(limit).collect();

    Json(json!(paginated))
}

async fn get_session(
    State(state): State<SharedState>,
    axum::extract::Path(sid): axum::extract::Path<String>,
) -> axum::response::Response {
    for (upstream, store) in &state.stores {
        let sessions = store.list_sessions().await.unwrap_or_default();
        if let Some(meta) = sessions.into_iter().find(|s| s.session_id == sid) {
            let requests = store.list_requests(&sid).await.unwrap_or_default();
            let aliases = state.alias_map.aliases_for(upstream);
            return Json(json!({
                "upstream": upstream,
                "aliases": aliases,
                "meta": meta,
                "requests": requests,
            }))
            .into_response();
        }
    }
    (axum::http::StatusCode::NOT_FOUND, "session not found").into_response()
}

async fn get_request(
    State(state): State<SharedState>,
    axum::extract::Path((sid, seq)): axum::extract::Path<(String, u64)>,
) -> axum::response::Response {
    for (_upstream, store) in &state.stores {
        if let Ok(Some(rec)) = store.get_request(&sid, seq).await {
            return match serde_json::to_value(&rec) {
                Ok(val) => Json(val).into_response(),
                Err(_) => {
                    (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "serialize error")
                        .into_response()
                }
            };
        }
    }
    (axum::http::StatusCode::NOT_FOUND, "request not found").into_response()
}

async fn stats(
    State(state): State<SharedState>,
    Query(params): Query<StatsQuery>,
) -> Json<Value> {
    let since = params
        .since
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc));

    let mut upstreams_stats = Vec::new();
    let mut totals_requests: u64 = 0;
    let mut totals_input: u64 = 0;
    let mut totals_output: u64 = 0;
    let mut totals_duration: u64 = 0;
    let mut totals_duration_count: u64 = 0;
    let mut totals_errors: u64 = 0;

    for (upstream, store) in &state.stores {
        let sessions = store.list_sessions().await.unwrap_or_default();
        let aliases = state.alias_map.aliases_for(upstream);
        let mut total_req: u64 = 0;
        let mut input_tokens: u64 = 0;
        let mut output_tokens: u64 = 0;
        let mut duration_sum: u64 = 0;
        let mut duration_count: u64 = 0;
        let mut ttft_sum: u64 = 0;
        let mut ttft_count: u64 = 0;
        let mut error_count: u64 = 0;
        let session_count = sessions.len() as u64;

        for session in &sessions {
            let requests = store.list_requests(&session.session_id).await.unwrap_or_default();
            for req in &requests {
                if let Some(since_dt) = since {
                    if req.started_at < since_dt {
                        continue;
                    }
                }
                total_req += 1;
                if let Some(inp) = req.input_tokens {
                    input_tokens += inp;
                }
                if let Some(outp) = req.output_tokens {
                    output_tokens += outp;
                }
                if let Some(dur) = req.duration_ms {
                    duration_sum += dur;
                    duration_count += 1;
                }
                if req.has_error {
                    error_count += 1;
                }
            }
        }

        let avg_duration = if duration_count > 0 {
            duration_sum / duration_count
        } else {
            0
        };
        let avg_ttft = if ttft_count > 0 {
            ttft_sum / ttft_count
        } else {
            0
        };
        let error_rate = if total_req > 0 {
            error_count as f64 / total_req as f64
        } else {
            0.0
        };

        totals_requests += total_req;
        totals_input += input_tokens;
        totals_output += output_tokens;
        totals_duration += duration_sum;
        totals_duration_count += duration_count;
        totals_errors += error_count;

        upstreams_stats.push(json!({
            "upstream": upstream,
            "aliases": aliases,
            "total_requests": total_req,
            "total_input_tokens": input_tokens,
            "total_output_tokens": output_tokens,
            "avg_duration_ms": avg_duration,
            "error_count": error_count,
            "error_rate": error_rate,
            "session_count": session_count,
        }));
    }

    let totals_avg_duration = if totals_duration_count > 0 {
        totals_duration / totals_duration_count
    } else {
        0
    };

    Json(json!({
        "upstreams": upstreams_stats,
        "totals": {
            "total_requests": totals_requests,
            "total_input_tokens": totals_input,
            "total_output_tokens": totals_output,
            "avg_duration_ms": totals_avg_duration,
            "error_count": totals_errors,
        },
        "since": since,
    }))
}

async fn stream(
    State(state): State<SharedState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.merged_events.subscribe();
    let sse_stream = BroadcastStream::new(rx).filter_map(|res| match res {
        Ok(tagged) => {
            let event_name = match &tagged.inner {
                ccs_proxy::CaptureEvent::RequestStarted { .. } => "request_started",
                ccs_proxy::CaptureEvent::RequestCompleted { .. } => "request_completed",
            };
            let data = serde_json::to_string(&tagged).unwrap_or_default();
            Some(Ok(Event::default().event(event_name).data(data)))
        }
        Err(_) => None,
    });
    Sse::new(sse_stream).keep_alive(KeepAlive::default())
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test --test daemon_aggregate -- --nocapture`
Expected: Still fails (need `serve()` function — Task 7).

- [ ] **Step 6: Commit**

```bash
git add src/daemon/aggregate/routes.rs src/daemon/aggregate/state.rs
git commit -m "feat(daemon): implement aggregate API routes (health, sessions, requests, stats, stream)"
```

---

### Task 7: Implement `aggregate::serve()` and `AggregateHandle`

**Files:**
- Modify: `src/daemon/aggregate/mod.rs`
- Create: `web-aggregate/index.html` (minimal placeholder for now)
- Create: `web-aggregate/app.js` (placeholder)
- Create: `web-aggregate/style.css` (placeholder)

- [ ] **Step 1: Implement `serve()` function and `AggregateHandle`**

In `src/daemon/aggregate/mod.rs`:

```rust
pub mod routes;
pub mod state;
pub mod stream;

use ccs_proxy::CaptureEvent;
use ccs_proxy::store::FsStore;
use state::{AggregateState, AliasMap};
use std::sync::Arc;
use stream::TaggedCaptureEvent;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

pub struct AggregateHandle {
    pub port: u16,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    join: Option<JoinHandle<()>>,
}

impl AggregateHandle {
    pub async fn shutdown(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(join) = self.join.take() {
            let _ = join.await;
        }
    }
}

impl Drop for AggregateHandle {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

pub async fn serve(
    stores: Vec<(String, Arc<FsStore>)>,
    proxy_events: Vec<(String, broadcast::Sender<CaptureEvent>)>,
    alias_map: Arc<AliasMap>,
    port: u16,
) -> anyhow::Result<AggregateHandle> {
    let listener = TcpListener::bind(("127.0.0.1", port)).await?;
    let bound_port = listener.local_addr()?.port();

    let (merged_tx, _) = broadcast::channel::<TaggedCaptureEvent>(2048);

    // Start event merger task
    let receivers: Vec<_> = proxy_events
        .iter()
        .map(|(upstream, sender)| (upstream.clone(), sender.subscribe()))
        .collect();
    let merger_alias_map = alias_map.clone();
    let merger_tx = merged_tx.clone();
    tokio::spawn(stream::event_merger(receivers, merger_alias_map, merger_tx));

    let agg_state = Arc::new(AggregateState {
        stores,
        merged_events: merged_tx,
        alias_map,
        started_at: chrono::Utc::now(),
    });

    let app = axum::Router::new()
        .merge(routes::router())
        .merge(ui_router())
        .with_state(agg_state);

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let join = tokio::spawn(async move {
        let server = axum::serve(listener, app);
        tokio::select! {
            res = server => {
                if let Err(err) = res {
                    tracing::warn!(error = %err, "aggregate server exited");
                }
            }
            _ = shutdown_rx => {}
        }
    });

    tracing::info!(port = bound_port, "aggregate server started");

    Ok(AggregateHandle {
        port: bound_port,
        shutdown_tx: Some(shutdown_tx),
        join: Some(join),
    })
}

// Embedded dashboard assets
use axum::Router;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "web-aggregate/"]
struct AggWebAsset;

fn ui_router() -> Router<Arc<AggregateState>> {
    Router::new()
        .route("/", get(|| async { serve_asset("index.html") }))
        .route("/index.html", get(|| async { serve_asset("index.html") }))
        .route("/app.js", get(|| async { serve_asset("app.js") }))
        .route("/style.css", get(|| async { serve_asset("style.css") }))
}

fn serve_asset(name: &str) -> Response {
    match AggWebAsset::get(name) {
        Some(asset) => {
            let mime = match std::path::Path::new(name)
                .extension()
                .and_then(|x| x.to_str())
            {
                Some("html") => "text/html; charset=utf-8",
                Some("js") => "application/javascript; charset=utf-8",
                Some("css") => "text/css; charset=utf-8",
                _ => "application/octet-stream",
            };
            (StatusCode::OK, [(header::CONTENT_TYPE, mime)], asset.data.into_owned()).into_response()
        }
        None => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}
```

- [ ] **Step 2: Create placeholder web assets**

Create `web-aggregate/index.html`:

```html
<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>ccs-daemon aggregate</title>
<link rel="stylesheet" href="/style.css">
</head>
<body>
<header id="header">ccs-daemon aggregate — loading...</header>
<main id="main">
  <aside id="sidebar"></aside>
  <section id="content"></section>
</main>
<script src="/app.js"></script>
</body>
</html>
```

Create `web-aggregate/app.js`:

```javascript
// Placeholder — full dashboard in Task 9
document.getElementById('header').textContent = 'ccs-daemon aggregate — connected';
```

Create `web-aggregate/style.css`:

```css
/* Placeholder — full styles in Task 9 */
* { margin: 0; padding: 0; box-sizing: border-box; }
body { font-family: monospace; }
```

- [ ] **Step 3: Add `rust-embed` dependency to workspace root `Cargo.toml`**

Add to `[dependencies]`:

```toml
rust-embed = "8"
```

- [ ] **Step 4: Run integration test**

Run: `cargo test --test daemon_aggregate -- --nocapture`
Expected: Pass — `/api/health` returns proxy_count=2.

- [ ] **Step 5: Commit**

```bash
git add src/daemon/aggregate/mod.rs web-aggregate/ Cargo.toml
git commit -m "feat(daemon): implement aggregate::serve() with embedded dashboard skeleton"
```

---

### Task 8: Wire aggregate server into daemon lifecycle

**Files:**
- Modify: `src/daemon/lifecycle.rs`
- Modify: `src/daemon/state.rs` (use `agg_port` in state writes)
- Modify: `src/daemon/commands.rs` (show agg URL in status)
- Modify: `src/daemon/status.rs` (display agg dashboard URL)

- [ ] **Step 1: Modify `run_daemon_async` to start aggregate server after proxies**

In `src/daemon/lifecycle.rs`, after spawning all proxies and before writing state:

```rust
use crate::daemon::aggregate;
use crate::daemon::aggregate::state::AliasMap;
use std::sync::Arc;

async fn run_daemon_async(cfg: LifecycleConfig) -> Result<()> {
    // ... existing pidfile + data_root setup ...
    // ... existing proxy spawn loop (add api_server: false to ServeConfig) ...

    // Build AliasMap from storage
    let storage = crate::config::ConfigStorage::load().unwrap_or_default();
    let alias_map = Arc::new(AliasMap::from_storage(&storage));

    // Collect stores and event senders for aggregate
    let agg_stores: Vec<_> = handles
        .iter()
        .zip(proxy_entries.iter())
        .map(|(handle, entry)| {
            let store = Arc::new(
                ccs_proxy::store::FsStore::open(entry.data_dir.clone())
                    .expect("store open should succeed — dir already exists"),
            );
            (entry.upstream.clone(), store)
        })
        .collect();

    let agg_events: Vec<_> = handles
        .iter()
        .zip(proxy_entries.iter())
        .map(|(handle, entry)| (entry.upstream.clone(), handle.event_sender().clone()))
        .collect();

    // Start aggregate server (non-fatal on failure)
    let agg_port = match aggregate::serve(agg_stores, agg_events, alias_map, 0).await {
        Ok(agg_handle) => {
            tracing::info!(port = agg_handle.port, "aggregate dashboard available");
            Some(agg_handle.port)
        }
        Err(err) => {
            tracing::warn!(error = %err, "failed to start aggregate server — proxies still work");
            None
        }
    };

    let state = DaemonState {
        schema_version: 2,
        pid: std::process::id(),
        started_at: chrono::Utc::now().to_rfc3339(),
        stopped_at: None,
        data_root: cfg.data_root.clone(),
        agg_port,
        proxies: proxy_entries.clone(),
    };
    // ... rest of lifecycle unchanged ...
}
```

- [ ] **Step 2: Pass `api_server: false` when spawning proxies in daemon mode**

In the proxy spawn loop in `lifecycle.rs`:

```rust
let mut serve_cfg = ccs_proxy::ServeConfig::new(
    ccs_proxy::ProviderKind::Claude,
    parsed_url,
    data_dir.clone(),
);
serve_cfg.api_server = false;

match ccs_proxy::serve(serve_cfg).await {
    // ...
}
```

And update `ProxyEntry` construction:

```rust
proxy_entries.push(ProxyEntry {
    provider: "claude".to_string(),
    upstream: upstream_url.clone(),
    proxy_port: handle.proxy_port,
    api_port: handle.api_port,  // will be None
    data_dir,
    started_at: chrono::Utc::now().to_rfc3339(),
    restart_count: 0,
});
```

- [ ] **Step 3: Add aggregate URL to status output**

In `src/daemon/status.rs`, modify `format_status_text` to show the aggregate dashboard:

```rust
pub fn format_status_text(
    state: &DaemonState,
    statuses: &[ProxyStatus],
    aliases_per_upstream: &AliasesByUpstream,
) -> String {
    let mut out = String::new();
    // ... existing header ...

    if let Some(agg_port) = state.agg_port {
        out.push_str(&format!("  dashboard: http://127.0.0.1:{}\n\n", agg_port));
    }

    // ... rest unchanged ...
}
```

And in `format_status_json`:

```rust
serde_json::json!({
    "status": "running",
    "pid": state.pid,
    "started_at": state.started_at,
    "data_root": state.data_root.display().to_string(),
    "agg_port": state.agg_port,
    "proxies": proxies,
})
```

- [ ] **Step 4: Run all tests**

Run: `cargo test --workspace`
Expected: All pass.

- [ ] **Step 5: Commit**

```bash
git add src/daemon/lifecycle.rs src/daemon/state.rs src/daemon/commands.rs src/daemon/status.rs
git commit -m "feat(daemon): wire aggregate server into lifecycle, show dashboard URL in status"
```

---

### Task 9: Implement the aggregate dashboard frontend

**Files:**
- Modify: `web-aggregate/index.html`
- Modify: `web-aggregate/app.js`
- Modify: `web-aggregate/style.css`

- [ ] **Step 1: Write `web-aggregate/index.html`**

```html
<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>ccs-daemon aggregate</title>
<link rel="stylesheet" href="/style.css">
</head>
<body>
<header id="header">
  <span class="brand">ccs-daemon</span>
  <span id="meta"></span>
</header>
<main id="main">
  <aside id="sidebar">
    <section id="filter-section">
      <h3>Upstreams</h3>
      <div id="upstream-filters"></div>
      <h3>Time</h3>
      <div id="time-filters">
        <button class="time-btn active" data-window="1h">1h</button>
        <button class="time-btn" data-window="24h">24h</button>
        <button class="time-btn" data-window="7d">7d</button>
        <button class="time-btn" data-window="all">all</button>
      </div>
    </section>
    <section id="stats-section">
      <h3>Stats</h3>
      <div id="stats-cards"></div>
    </section>
  </aside>
  <section id="content">
    <table id="request-table">
      <thead>
        <tr>
          <th>Time</th>
          <th>Upstream</th>
          <th>Model</th>
          <th>Tokens</th>
          <th>Status</th>
          <th>Duration</th>
          <th>Request-ID</th>
        </tr>
      </thead>
      <tbody id="request-list"></tbody>
    </table>
    <div id="pagination"></div>
    <div id="detail-panel" class="hidden">
      <div id="detail-tabs">
        <button class="tab active" data-tab="overview">Overview</button>
        <button class="tab" data-tab="request">Request</button>
        <button class="tab" data-tab="response">Response</button>
        <button class="tab" data-tab="headers">Headers</button>
        <button class="tab" data-tab="usage">Usage</button>
      </div>
      <div id="detail-content"></div>
    </div>
  </section>
</main>
<script src="/app.js"></script>
</body>
</html>
```

- [ ] **Step 2: Write `web-aggregate/style.css`**

Full styles (~250 lines): monospace, dark-on-light, flexbox two-pane layout (sidebar 280px, content flex-1), table styling, color-coded upstream borders, tab styling, detail panel, pagination buttons. Follow the same conventions as `ccs-proxy/web/style.css`.

- [ ] **Step 3: Write `web-aggregate/app.js`**

Full JS (~800 lines) implementing:
- `init()`: fetch `/api/health`, populate header (uptime, proxy count, total requests, port).
- `loadUpstreamFilters()`: from health data, render checkbox list with color-coded labels.
- `loadStats()`: fetch `/api/stats?since=<window>`, render per-upstream cards.
- `loadRequests()`: fetch `/api/sessions` with filters, build request list from all sessions' requests. Support pagination.
- `connectStream()`: `EventSource` on `/api/stream`, prepend new rows to table.
- `selectRow(sid, seq)`: fetch `/api/requests/:sid/:seq`, populate detail panel tabs.
- Time window filter: button click changes `since` parameter.
- Upstream filter: checkbox toggles filter by upstream.
- Color palette: assign colors to upstreams for left-border on rows.

- [ ] **Step 4: Manual smoke test**

Start daemon in foreground mode: `cargo run -- daemon start --foreground`
Open `http://127.0.0.1:<agg_port>` in browser.
Verify: header shows stats, sidebar has upstream checkboxes, time filter buttons work, request list populates when traffic flows.

- [ ] **Step 5: Commit**

```bash
git add web-aggregate/
git commit -m "feat(daemon): implement aggregate dashboard frontend"
```

---

### Task 10: Integration test — full daemon with aggregate

**Files:**
- Create: `tests/daemon_aggregate.rs` (expand from Task 6 placeholder)

- [ ] **Step 1: Write comprehensive integration test**

```rust
// tests/daemon_aggregate.rs
#[cfg(unix)]
mod daemon_aggregate {
    use ccs_proxy::store::{FsStore, SessionMeta, Store};
    use ccs_proxy::capture::{CaptureRecord, RequestPart, ResponsePart, Usage};
    use std::sync::Arc;
    use std::collections::BTreeMap;
    use tempfile::TempDir;

    #[tokio::test]
    async fn aggregate_sessions_merges_across_stores() {
        let tmp_a = TempDir::new().unwrap();
        let tmp_b = TempDir::new().unwrap();
        let store_a = Arc::new(FsStore::open(tmp_a.path().to_path_buf()).unwrap());
        let store_b = Arc::new(FsStore::open(tmp_b.path().to_path_buf()).unwrap());

        // Populate store_a with a session
        store_a.init_session(SessionMeta {
            session_id: "sess_a".to_string(),
            provider: "claude".to_string(),
            upstream: "https://a.example.com".to_string(),
            proxy_port: 0,
            api_port: 0,
            started_at: chrono::Utc::now(),
            ended_at: None,
            request_count: 1,
            schema_version: 1,
        }).await.unwrap();

        // Populate store_b with a session
        store_b.init_session(SessionMeta {
            session_id: "sess_b".to_string(),
            provider: "claude".to_string(),
            upstream: "https://b.example.com".to_string(),
            proxy_port: 0,
            api_port: 0,
            started_at: chrono::Utc::now() - chrono::Duration::hours(1),
            ended_at: None,
            request_count: 2,
            schema_version: 1,
        }).await.unwrap();

        let stores = vec![
            ("https://a.example.com".to_string(), store_a),
            ("https://b.example.com".to_string(), store_b),
        ];

        let alias_map = Arc::new(
            cc_switch::daemon::aggregate::state::AliasMap::from_entries(vec![
                ("https://a.example.com".to_string(), vec!["work".to_string()]),
                ("https://b.example.com".to_string(), vec!["personal".to_string()]),
            ]),
        );

        let handle = cc_switch::daemon::aggregate::serve(stores, vec![], alias_map, 0)
            .await
            .unwrap();

        // Test /api/sessions returns both sessions
        let resp = reqwest::get(format!("http://127.0.0.1:{}/api/sessions", handle.port))
            .await.unwrap();
        let sessions: Vec<serde_json::Value> = resp.json().await.unwrap();
        assert_eq!(sessions.len(), 2);
        // First should be most recent (sess_a)
        assert_eq!(sessions[0]["session_id"], "sess_a");
        assert_eq!(sessions[0]["upstream"], "https://a.example.com");
        assert_eq!(sessions[0]["aliases"][0], "work");

        // Test filter by upstream
        let resp = reqwest::get(format!(
            "http://127.0.0.1:{}/api/sessions?upstream=https://b.example.com",
            handle.port
        )).await.unwrap();
        let sessions: Vec<serde_json::Value> = resp.json().await.unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0]["session_id"], "sess_b");

        // Test filter by alias
        let resp = reqwest::get(format!(
            "http://127.0.0.1:{}/api/sessions?alias=work",
            handle.port
        )).await.unwrap();
        let sessions: Vec<serde_json::Value> = resp.json().await.unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0]["session_id"], "sess_a");

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn aggregate_stats_computes_per_upstream() {
        let tmp_a = TempDir::new().unwrap();
        let store_a = Arc::new(FsStore::open(tmp_a.path().to_path_buf()).unwrap());

        store_a.init_session(SessionMeta {
            session_id: "sess1".to_string(),
            provider: "claude".to_string(),
            upstream: "https://a.example.com".to_string(),
            proxy_port: 0, api_port: 0,
            started_at: chrono::Utc::now(),
            ended_at: None, request_count: 2, schema_version: 1,
        }).await.unwrap();

        store_a.append(CaptureRecord {
            seq: 1,
            session_id: "sess1".to_string(),
            request_id: Some("req_1".to_string()),
            started_at: chrono::Utc::now(),
            ended_at: Some(chrono::Utc::now()),
            duration_ms: Some(1000),
            ttft_ms: Some(200),
            request: RequestPart { method: "POST".into(), path: "/v1/messages".into(), headers: BTreeMap::new(), body: serde_json::Value::Null },
            response: Some(ResponsePart { status: 200, headers: BTreeMap::new(), body_reassembled: None, raw_sse_text: None, raw_sse_frames_count: 0 }),
            usage: Some(Usage { input_tokens: 100, output_tokens: 50, cache_creation_input_tokens: 0, cache_read_input_tokens: 0 }),
            model: Some("claude-sonnet-4-6".to_string()),
            error: None, partial: false, schema_version: 1,
        }).await.unwrap();

        let stores = vec![("https://a.example.com".to_string(), store_a)];
        let alias_map = Arc::new(cc_switch::daemon::aggregate::state::AliasMap::from_entries(vec![]));
        let handle = cc_switch::daemon::aggregate::serve(stores, vec![], alias_map, 0).await.unwrap();

        let resp = reqwest::get(format!("http://127.0.0.1:{}/api/stats", handle.port)).await.unwrap();
        let stats: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(stats["upstreams"][0]["total_requests"], 1);
        assert_eq!(stats["upstreams"][0]["total_input_tokens"], 100);
        assert_eq!(stats["totals"]["total_requests"], 1);

        handle.shutdown().await;
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --test daemon_aggregate -- --nocapture`
Expected: All pass.

- [ ] **Step 3: Commit**

```bash
git add tests/daemon_aggregate.rs
git commit -m "test(daemon): add integration tests for aggregate API"
```

---

### Task 11: Final verification and cleanup

**Files:**
- All modified files from Tasks 1-10

- [ ] **Step 1: Run full test suite**

Run: `cargo test --workspace`
Expected: All pass.

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: No warnings.

- [ ] **Step 3: Check binary compiles in release mode**

Run: `cargo build --release`
Expected: Builds successfully.

- [ ] **Step 4: Manual end-to-end test**

```bash
# Start daemon in foreground
cargo run -- daemon start --foreground
# In another terminal, check status
cargo run -- daemon status
# Verify agg_port is shown
# Open http://127.0.0.1:<agg_port> in browser
# Send a test request through a proxy port
# Verify it appears in the aggregate dashboard
```

- [ ] **Step 5: Final commit (if any cleanup needed)**

```bash
git add -A
git commit -m "chore: cleanup and verify daemon aggregate layer"
```
