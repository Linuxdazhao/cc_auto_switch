# Spec C — Daemon Aggregation Layer

**Date**: 2026-05-28  
**Status**: Draft, pending review  
**Depends on**: Spec A (ccs-proxy crate), Spec B (cc daemon lifecycle)  
**Scope**: Unified cross-upstream observability dashboard + API, hosted inside the daemon process.

## 1. Context & motivation

With Spec B, the daemon supervises one `ccs_proxy::serve()` per unique upstream URL. Each proxy stores captured requests independently in `~/.cc-switch/daemon-data/<hash>/sessions/`. Currently, the only way to observe traffic is to visit each per-proxy dashboard individually — there is no cross-upstream view.

This spec adds an **aggregation layer**: a single API server (one port) inside the daemon that merges data from all proxies and presents a unified dashboard. Users get:

1. **Data isolation preserved** — each proxy's store remains independent. One crashing doesn't affect another's history.
2. **Cross-upstream comparison** — latency, token cost, error rates across all upstreams in one view.
3. **Unified filtering** — filter by upstream, alias, or time window from a single page.
4. **Zero-config discovery** — the aggregation server automatically discovers all proxy data directories.
5. **Backward compatible** — per-proxy dashboards remain available when running `ccs-proxy` standalone. The aggregation layer is additive.

## 2. Scope & non-goals

### v1 in-scope

- Aggregation API server as a tokio task inside the daemon process (single port, OS-assigned).
- Unified `/api/sessions`, `/api/requests/:sid/:seq`, `/api/stats`, `/api/stream` across all proxies.
- Real-time event merging via direct `broadcast::Receiver` subscription to each proxy's event channel.
- Historical data read by scanning all `daemon-data/*/sessions/` directories.
- Alias resolution: map upstream URLs back to alias names via `configurations.json`.
- New embedded dashboard (`web-aggregate/`) — vanilla JS/CSS, no build step.
- `ServeConfig.api_server: bool` option in ccs-proxy to skip the per-proxy API server in daemon mode.
- `ProxyHandle::subscribe_events()` public method for event subscription.
- `daemon-state.json` extended with `agg_port` field.
- `cc daemon status` displays the aggregation dashboard URL.
- Pagination on list endpoints (`?limit=&offset=`).

### v1 explicitly out-of-scope

| Feature | Rationale |
|---|---|
| In-memory index / caching for historical queries | v1 full-scan is acceptable at expected scale (<10k requests); v2 optimization |
| Per-upstream pricing / cost computation | Requires pricing table; v2 |
| Export (CSV, JSON dump, HAR) | API already returns JSON; v2 |
| Hot-reload of alias map when configurations.json changes | Use `cc daemon restart`; v2 |
| Authentication on aggregate dashboard | Loopback-only bind; same posture as per-proxy |
| Aggregate across multiple machines / daemons | v1 = one daemon per `$HOME` |
| Sparkline trends / charts | v2 UI polish |
| Custom time range (date picker) | v1 has preset windows (1h/24h/7d/all); v2 |

## 3. Architecture

### Process model

```
┌──────────────────────────────────────────────────────────┐
│ daemon process (tokio runtime)                            │
│                                                          │
│  ├── ccs_proxy::serve(A, api_server=false)               │
│  │     proxy_port=41001 (forwarding only)                │
│  │                                                       │
│  ├── ccs_proxy::serve(B, api_server=false)               │
│  │     proxy_port=41002 (forwarding only)                │
│  │                                                       │
│  ├── supervisor task                                     │
│  │                                                       │
│  └── aggregate server (agg_port=41600)                   │
│       ├── subscribes to A.events + B.events              │
│       ├── reads daemon-data/*/sessions/ for history      │
│       └── serves unified dashboard + API                 │
└──────────────────────────────────────────────────────────┘
```

In daemon mode, per-proxy `api_port` is **not started**. All observability goes through the single aggregate port. The standalone `ccs-proxy` binary is unaffected (always starts its own API server).

### File layout

```
cc-switch/
├── src/daemon/
│   ├── aggregate/
│   │   ├── mod.rs          # AggregateHandle, serve(), public API
│   │   ├── routes.rs       # /api/* handlers
│   │   ├── stream.rs       # merged SSE endpoint (select over all proxy channels)
│   │   └── state.rs        # AggregateState, AliasMap, TaggedCaptureEvent
│   └── lifecycle.rs        # modified: spawn aggregate server after proxies
│
├── web-aggregate/          # new dashboard (embedded via rust-embed)
│   ├── index.html
│   ├── app.js
│   └── style.css
│
└── Cargo.toml              # add rust-embed dependency
```

### Key types

```rust
// src/daemon/aggregate/state.rs

pub struct AggregateState {
    /// Per-proxy store references (upstream_url, FsStore) — read-only
    pub stores: Vec<(String, Arc<ccs_proxy::store::FsStore>)>,
    /// Merged event channel for SSE subscribers
    pub merged_events: broadcast::Sender<TaggedCaptureEvent>,
    /// Alias → upstream mapping (loaded from configurations.json at start)
    pub alias_map: Arc<AliasMap>,
    /// Daemon start time
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaggedCaptureEvent {
    pub upstream: String,
    pub aliases: Vec<String>,
    #[serde(flatten)]
    pub inner: CaptureEvent,
}

pub struct AliasMap {
    /// upstream_url → Vec<alias_name>
    map: HashMap<String, Vec<String>>,
}

impl AliasMap {
    pub fn from_storage(storage: &ConfigStorage) -> Self;
    pub fn aliases_for(&self, upstream: &str) -> Vec<String>;
}
```

```rust
// src/daemon/aggregate/mod.rs

pub struct AggregateHandle {
    pub port: u16,
    // private: shutdown channel, join handle
}

pub async fn serve(
    stores: Vec<(String, Arc<FsStore>)>,
    proxy_events: Vec<(String, broadcast::Sender<CaptureEvent>)>,
    alias_map: Arc<AliasMap>,
    port: u16,  // 0 = OS picks
) -> Result<AggregateHandle>;
```

## 4. ccs-proxy library changes

### ServeConfig

```rust
pub struct ServeConfig {
    pub provider: ProviderKind,
    pub upstream: Url,
    pub proxy_port: u16,
    pub api_port: u16,
    pub data_dir: PathBuf,
    pub redact: bool,
    pub api_server: bool,  // NEW: default true; false skips API/dashboard server
}
```

### ProxyHandle

```rust
pub struct ProxyHandle {
    pub provider: ProviderKind,
    pub upstream: Url,
    pub proxy_port: u16,
    pub api_port: Option<u16>,  // CHANGED: None when api_server=false
    // private fields...
}

impl ProxyHandle {
    /// Subscribe to live capture events for this proxy.
    /// Returns a broadcast receiver tagged with this proxy's upstream.
    pub fn subscribe_events(&self) -> broadcast::Receiver<CaptureEvent>;  // NEW

    /// Access the underlying event sender (for aggregate subscription setup).
    pub fn event_sender(&self) -> &broadcast::Sender<CaptureEvent>;  // NEW
}
```

The `ProxyHandle` internally holds an `Arc` reference to the `AppState.events` sender.

## 5. Aggregate API surface

All endpoints bind to `127.0.0.1:<agg_port>`.

| Method + Path | Returns |
|---|---|
| `GET /api/health` | `{status, pid, uptime_s, proxy_count, agg_port, total_requests, proxies: [{upstream, proxy_port, request_count, aliases}]}` |
| `GET /api/sessions` | All sessions across all proxies, each with `upstream` and `aliases` fields. Sorted by `started_at` desc. Supports `?upstream=`, `?alias=`, `?limit=`, `?offset=` |
| `GET /api/sessions/:sid` | Session meta + request summary list (same as per-proxy but works across all stores) |
| `GET /api/requests/:sid/:seq` | Full `CaptureRecord` |
| `GET /api/stats` | Per-upstream aggregate statistics (see §5.1). Supports `?since=<ISO8601>` |
| `GET /api/stream` | SSE: merged `TaggedCaptureEvent` from all proxies |
| `GET /` | Aggregate dashboard (embedded HTML/JS/CSS) |
| `GET /app.js`, `/style.css` | Embedded static assets |

### 5.1 `/api/stats` response

```json
{
  "upstreams": [
    {
      "upstream": "https://api.anthropic.com",
      "aliases": ["work", "personal"],
      "total_requests": 342,
      "total_input_tokens": 1200000,
      "total_output_tokens": 450000,
      "avg_duration_ms": 2800,
      "avg_ttft_ms": 380,
      "error_count": 5,
      "error_rate": 0.015,
      "session_count": 12
    }
  ],
  "totals": {
    "total_requests": 500,
    "total_input_tokens": 1800000,
    "total_output_tokens": 700000,
    "avg_duration_ms": 3100,
    "error_count": 8
  },
  "since": null
}
```

### 5.2 `/api/stream` events

```
event: request_started
data: {"upstream":"https://api.anthropic.com","aliases":["work","personal"],"type":"request_started","session_id":"...","seq":42,"started_at":"...","model":"claude-sonnet-4-6"}

event: request_completed
data: {"upstream":"https://api.anthropic.com","aliases":["work","personal"],"type":"request_completed","session_id":"...","seq":42,"duration_ms":3333,"status":200,"usage":{"input_tokens":1024,"output_tokens":256},"request_id":"req_01H8...","has_error":false}
```

## 6. Data flow

### Startup

```
cc daemon start
  │
  ├── load configurations.json, dedupe upstreams
  ├── build AliasMap from configurations.json
  ├── for each upstream:
  │     handle = ccs_proxy::serve(ServeConfig { api_server: false, ... })
  │
  ├── collect stores: for each handle, open read-only FsStore at its data_dir
  ├── collect event senders: for each handle, handle.event_sender()
  │
  ├── agg_handle = aggregate::serve(stores, event_senders, alias_map, port=0)
  │
  ├── write daemon-state.json (includes agg_port)
  └── supervisor loop
```

### Real-time event merge

A dedicated tokio task subscribes to all proxy event senders:

```rust
async fn event_merger(
    proxy_events: Vec<(String, broadcast::Receiver<CaptureEvent>)>,
    alias_map: Arc<AliasMap>,
    merged_tx: broadcast::Sender<TaggedCaptureEvent>,
) {
    // Use tokio::select! over all receivers
    // On event: tag with upstream + aliases, send to merged_tx
    // On channel closed: log warn, remove from set
}
```

When the supervisor restarts a proxy, it notifies the merger task via a `watch` channel containing the updated list of event senders.

### Historical query

On API request (e.g., `GET /api/sessions`):
1. For each store in `stores`: call `store.list_sessions().await`
2. Merge all results, annotate each `SessionMeta` with `upstream` and `aliases`
3. Sort by `started_at` desc
4. Apply filters (`?upstream=`, `?alias=`, `?limit=`, `?offset=`)
5. Return JSON

For `GET /api/requests/:sid/:seq`:
1. Try each store until one returns `Some(record)`
2. Return the record (session_id is globally unique due to timestamp + random suffix)

## 7. State file changes

`~/.cc-switch/daemon-state.json` gains one field:

```json
{
  "schema_version": 2,
  "pid": 38421,
  "started_at": "...",
  "stopped_at": null,
  "data_root": "/Users/jingzhao/.cc-switch/daemon-data",
  "agg_port": 41600,
  "proxies": [
    {
      "provider": "claude",
      "upstream": "https://api.anthropic.com",
      "proxy_port": 41001,
      "api_port": null,
      "data_dir": "/Users/jingzhao/.cc-switch/daemon-data/8f3a2c1e",
      "started_at": "...",
      "restart_count": 0
    }
  ]
}
```

Changes:
- `schema_version` bumped to `2`
- `agg_port: u16` added at top level
- `proxies[].api_port` becomes `Option<u16>` (always `null` in daemon mode)

Backward compat: `cc use` only reads `proxies[].proxy_port` and `proxies[].upstream` — unaffected.

## 8. Dashboard MVP

New frontend `web-aggregate/`, vanilla JS/CSS, embedded via `rust-embed`. Constraints: ≤ 1000 lines JS + ≤ 300 lines CSS. No React/Vue/build step.

### Layout

```
┌─────────────────────────────────────────────────────────────┐
│ Header: daemon uptime │ proxy count │ total requests │ port │
├────────────┬────────────────────────────────────────────────┤
│ Filter     │ Main area                                      │
│            │                                                │
│ Upstreams: │ Request list table:                            │
│ ☑ api.anth │  Time | Upstream | Model | Tokens | Status |  │
│ ☑ glm.exam │  Duration | Request-ID                        │
│            │                                                │
│ Time:      │ ─────────────────────────────────────────────  │
│ [1h] [24h] │ Click row → expand detail panel:              │
│ [7d] [all] │   Tabs: Overview / Request / Response /       │
│            │         Headers / Usage                        │
│ Stats:     │                                                │
│ per-upstream│                                               │
│ tokens/err │                                                │
└────────────┴────────────────────────────────────────────────┘
```

### Features

| ✅ v1 | ❌ v2+ |
|---|---|
| Header with daemon stats | Charts / sparklines |
| Upstream checkbox filter (color-coded rows) | Custom date range picker |
| Preset time windows (1h/24h/7d/all) | Per-model filter |
| Per-upstream stats cards (tokens, errors, avg latency) | Export / download |
| Combined request list sorted by time | Cross-session diff |
| Click-to-expand detail (Overview/Request/Response/Headers/Usage) | Theme toggle |
| Live updates via `/api/stream` SSE | Copy as cURL |
| Error rows highlighted | Cost estimation |
| Pagination (100 per page) | Search / full-text |

### Color coding

Each upstream gets a distinct left-border color in the request list (auto-assigned from a small palette). The filter sidebar shows the same color next to the upstream checkbox.

## 9. Error handling

| Case | Behavior |
|---|---|
| Aggregate port bind failure | Daemon starts normally (proxies work). Warn log. `cc daemon status` shows "aggregate: unavailable". |
| One proxy's data_dir unreadable | Skip that store. Others normal. Warn log. |
| Proxy restart → channel closed | Event merger removes dead channel, re-subscribes on new handle (via watch notification). |
| Empty data dirs | API returns empty arrays / zero stats. Dashboard shows "no data yet". |
| Corrupt meta.json / seq.json | Skipped with warn (same behavior as per-proxy FsStore). |
| >10k historical requests (slow scan) | Accept latency in v1. Pagination mitigates UI impact. |
| configurations.json not found | AliasMap is empty; upstream URLs shown without alias names. |

## 10. Security

- Aggregate port binds `127.0.0.1` only (code-level invariant, same as proxy ports).
- No auth on dashboard (relies on loopback bind).
- No additional secrets in aggregate state. URLs only.
- Dashboard never displays raw API tokens (redaction happens at capture time in ccs-proxy).
- File permissions inherited from daemon-data (`0700` dirs, `0600` files).

## 11. Testing strategy

| Layer | Coverage | Tooling |
|---|---|---|
| Unit | AliasMap construction + lookup, stats aggregation math, TaggedCaptureEvent serialization | `cargo test --lib` |
| Integration | Spawn daemon (foreground) with 2 upstreams (mock) → send requests → verify aggregate API returns merged data with correct upstream tags | `tests/daemon_aggregate.rs` with `wiremock` |
| Event merge | Create multiple broadcast senders → verify merger task produces tagged events in order | Unit test in `aggregate/stream.rs` |
| Session lookup | Pre-populate two data dirs → verify `/api/sessions` merges + sorts correctly | Integration test |
| Dashboard | Manual smoke test | Documented checklist |

Coverage target: ≥ 75% on `src/daemon/aggregate/`.

## 12. Observability

- Aggregate server logs via the daemon's unified tracing subscriber (from Daemon Tracing spec).
- Access logs at `debug` level: `aggregate: GET /api/sessions 200 12ms`.
- Error logs at `warn`: store scan failures, channel disconnects.
- `/api/health` serves as the diagnostic endpoint for the aggregate layer.

## 13. Build & distribution

- `rust-embed` added as a dependency to cc-switch (not just ccs-proxy) for embedding `web-aggregate/`.
- No new binaries. The aggregate server is part of the daemon process.
- Binary size impact: +200KB estimated (rust-embed assets + aggregate routes).
- MSRV unchanged: 1.88+, edition 2024.

## 14. Changes to existing code

| File | Change |
|---|---|
| `ccs-proxy/src/lib.rs` | `ServeConfig` add `api_server: bool` (default `true`). `ProxyHandle.api_port` → `Option<u16>`. Add `ProxyHandle::subscribe_events()` and `event_sender()`. |
| `ccs-proxy/src/bin/ccs-proxy.rs` | No change (always `api_server: true`). |
| `src/daemon/lifecycle.rs` | Pass `api_server: false` to `ccs_proxy::serve()`. After proxy spawn, start aggregate server. Pass event senders + stores to aggregate. |
| `src/daemon/state.rs` | `DaemonState` add `agg_port: u16`. `ProxyEntry.api_port` → `Option<u16>`. Bump `schema_version` to 2. |
| `src/daemon/commands.rs` | `status` output includes aggregate dashboard URL. |
| `src/daemon/mod.rs` | Add `pub mod aggregate;` |
| `Cargo.toml` (workspace root) | Add `rust-embed` dependency. |
| `src/daemon/aggregate/` | New module (~400 lines Rust). |
| `web-aggregate/` | New frontend (~1300 lines HTML/JS/CSS). |

## 15. Approved design decisions (reference)

- Aggregation server runs **inside the daemon process** (one additional axum task).
- Daemon mode **does not start per-proxy api_port** — all UI through aggregate port.
- Real-time events via **direct broadcast channel subscription** (no HTTP polling between processes).
- Historical reads via **direct FsStore filesystem scan** (no separate database).
- New **standalone frontend** (`web-aggregate/`), not an extension of per-proxy dashboard.
- v1 **full-scan** for stats/sessions queries (no caching/indexing); pagination to mitigate.
- Alias resolution from `configurations.json` snapshot at daemon start (no hot-reload).
- Aggregate port failure is **non-fatal** — proxies continue to work.
- `schema_version` bumped to 2 in state file.

## 16. Open work to verify before implementation

- Confirm `broadcast::Sender<CaptureEvent>` can be subscribed from outside the ccs-proxy crate (may need to make `CaptureEvent` public at crate root — verify current re-exports).
- Decide whether `rust-embed` in cc-switch should use the same version as ccs-proxy or if they can diverge.
- Verify that reading another process's `FsStore` while it's actively writing is safe (atomic write via rename should guarantee this — confirm no partial reads possible).
- Spike: confirm `tokio::select!` over a dynamic vec of broadcast receivers scales to 10+ without issues (alternative: `StreamMap`).
