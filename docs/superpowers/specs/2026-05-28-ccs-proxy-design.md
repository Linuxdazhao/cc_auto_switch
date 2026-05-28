# Spec A — `ccs-proxy` crate (Rust port of ccglass core)

**Date**: 2026-05-28
**Status**: Approved design, ready for implementation plan
**Companion**: Spec B (cc-switch daemon integration) — separate document, follow-up

## 1. Context & motivation

cc-switch users want all `cc use <alias>` and `cc codex use <alias>` traffic
to optionally route through a local logging reverse-proxy + web dashboard,
so they can see exactly what their coding agent sends to the model — same
value proposition as the existing Node.js [ccglass](https://github.com/jianshuo/ccglass).

We decided to **reimplement the ccglass core in Rust** rather than supervise
the Node binary as a child process. Primary motivation: **deep integration
with cc-switch** — share `ConfigStorage` / `EnvironmentConfig` / provider
types, avoid env-var translation, single-binary distribution path.

This spec covers **Spec A** only: the standalone `ccs-proxy` crate. Spec B
(the `cc daemon` lifecycle, `cc use` rewiring, supervisor, state file) is a
follow-up that depends on Spec A's stable API surface.

## 2. Scope & non-goals

### v1 in-scope

- Reverse proxy for **two providers**: `claude` (Anthropic Messages,
  `POST /v1/messages`) and `codex` (OpenAI Responses + Chat Completions,
  `POST /v1/responses`, `POST /v1/chat/completions`).
- Streaming SSE forwarding **without buffering** (no perceived latency
  added vs direct connection).
- SSE response reassembly into final message + tool_calls + usage.
- Full request/response capture to disk including all response headers
  (`anthropic-request-id`, `x-request-id` must be preserved — user-flagged
  requirement).
- Per-session directory layout with per-request JSON files.
- HTTP API (`/api/sessions`, `/api/requests/:sid/:seq`, `/api/stream` SSE).
- Minimal web dashboard embedded in the binary (rust-embed).
- Library API (`serve()` returning `ProxyHandle`) so cc-switch (Spec B) can
  import as a dependency.
- Default redaction of `authorization` / `x-api-key` / `anthropic-api-key`
  / `cookie` headers when writing to disk; `--no-redact` to disable.
- `127.0.0.1` bind only on both proxy and api ports.
- Unix only (macOS + Linux); Windows reports a clear error.

### v1 explicitly out-of-scope

| Feature | Rationale |
|---|---|
| Providers beyond claude / codex (kimi, deepseek, glm, ollama, bedrock, …) | Each needs format glue + env var; v2 |
| MCP self-inspection server | Non-trivial in Rust; v2 |
| Turn-to-turn diff (UI feature) | Pure UI; v2 |
| Per-provider pricing / cost computation | Requires pricing table maintenance; v2 |
| Content-addressed (git-style) blob storage | Performance optimization; v2 |
| Export to HAR / Markdown / raw HTTP | HTTP API already exposes JSON; v2 |
| Theme switching, sparkline trends, per-model filter, cURL copy | UI polish; v2/v3 |
| Multiple sessions in one process | v1 session = one `serve()` lifetime |
| Cross-session dashboard view | v1 dashboard shows current session only |
| Subcommands `view` / `migrate` / `repack` / `rm` / `export` / `usage` | External tools can hit HTTP API; v2 |

## 3. User-facing surface

### Binary CLI

```
ccs-proxy serve [--proxy-port 0] [--api-port 0]
                --upstream <url> --provider claude|codex
                [--data-dir ~/.ccs-proxy/]
                [--no-redact]
                [--no-open]
                [--cors-allow <origin>]

ccs-proxy --version
ccs-proxy --help
```

`--port 0` (default) lets the OS pick. `--no-open` skips opening the
dashboard URL (`http://127.0.0.1:<api_port>/`) in the user's default
browser at startup; default behavior is to open it on first launch.
`--cors-allow=*` is dev-mode only and emits a warning.

### Library API (stable in v1)

```rust
// ccs_proxy

pub enum ProviderKind {
    Claude,
    Codex,
    // marked #[non_exhaustive] for v2 expansion
}

pub struct ServeConfig {
    pub provider: ProviderKind,
    pub upstream: Url,
    pub proxy_port: u16,    // 0 = auto
    pub api_port: u16,      // 0 = auto
    pub data_dir: PathBuf,
    pub redact: bool,
}

pub struct ProxyHandle {
    pub provider: ProviderKind,
    pub upstream: Url,
    pub proxy_port: u16,
    pub api_port: u16,
    // private: shutdown channels, join handles
}

/// Starts both servers, returns once both ports are bound.
/// Drop the handle to gracefully shut down.
pub async fn serve(cfg: ServeConfig) -> Result<ProxyHandle, ServeError>;
```

**API stability commitment for v1**: only the `serve()` signature and the
four public `ProxyHandle` fields above are frozen. Everything else can
shift between v1 patch releases without notice.

## 4. Architecture

```
ccs-proxy crate
├── src/lib.rs                  # pub re-exports + serve()
├── src/bin/ccs-proxy.rs        # CLI binary
│
├── src/provider/
│   ├── mod.rs                  # ProviderKind, common trait
│   ├── claude.rs               # Anthropic: path whitelist, SSE frames, usage
│   └── codex.rs                # OpenAI: same shape
│
├── src/proxy/
│   ├── mod.rs                  # axum app for proxy_port
│   ├── forward.rs              # reqwest stream forward
│   ├── sse_tap.rs              # split: tee SSE to client + reassembler
│   └── reassembler.rs          # rebuild final response per provider
│
├── src/capture/
│   ├── mod.rs                  # CaptureRecord type
│   ├── redact.rs               # mask secrets pre-write
│   └── extract.rs              # request-id, usage, model
│
├── src/store/
│   ├── mod.rs                  # Store trait
│   ├── fs.rs                   # filesystem impl
│   └── index.rs                # in-memory index (built at start)
│
├── src/api/
│   ├── mod.rs                  # axum app for api_port
│   ├── routes.rs               # /api/* handlers
│   ├── stream.rs               # broadcast → SSE for /api/stream
│   └── ui.rs                   # rust-embed mount at /
│
└── web/                        # dashboard source (embedded at build)
    ├── index.html
    ├── app.js
    └── style.css
```

### Two servers, shared state

```rust
pub struct AppState {
    pub store: Arc<dyn Store>,
    pub events: broadcast::Sender<CaptureEvent>,
    pub provider: ProviderKind,
    pub upstream: Url,
    pub session_id: SessionId,    // generated at serve() start
}
```

`serve()` spawns two axum servers in tokio tasks; returns `ProxyHandle`
only after both have successfully `listen()`-ed. `Drop` on the handle
triggers `oneshot` shutdown channels and flushes the store.

### Key dependencies (target version pins decided at impl time)

| Crate | Purpose |
|---|---|
| `axum` 0.8 | HTTP server (both ports) |
| `tokio` 1 (full) | async runtime |
| `tower` / `tower-http` | middleware, tracing, cors |
| `reqwest` (rustls, stream) | upstream client with streaming response |
| `hyper` | low-level body work |
| `tokio-util` | StreamReader, codec |
| `serde` / `serde_json` | JSON |
| `rust-embed` | embed dashboard static assets |
| `tracing` / `tracing-subscriber` | logging |
| `anyhow` (bin) / `thiserror` (lib) | error handling |
| `chrono` | timestamps |
| `url` | URL parsing |
| `uuid` | fallback request id |
| `wiremock` (dev) | mock upstream in tests |
| `criterion` (dev) | benchmark |

Single-binary release target: < 15MB.

## 5. Data flow (request lifecycle)

```
client ──①──▶ proxy_port ──②──▶ upstream
                  │
                  │ ③ stream response
                  ▼
              sse_tap split
              ├── ④ direct to client (no extra latency)
              └── ⑤ to reassembler → capture::Record
                                          │
                                          ├── store.append() → sessions/<sid>/<seq>.json
                                          └── events.send() → /api/stream subscribers
```

| # | Step |
|---|---|
| ① | Client POSTs to `proxy_port` with `stream: true` |
| ② | `reqwest` stream-forwards body to upstream (no full buffering) |
| ③ | Upstream returns SSE; reqwest yields a byte stream |
| ④ | `sse_tap` immediately forwards every chunk to the client — **no added latency** |
| ⑤ | Same chunks fed to `reassembler` which parses provider-specific events into a final message + usage |
| ⑥ | On stream end: store writes `<seq>.json` atomically, broadcast notifies dashboard |

**Critical invariants**:

- **Streaming**: proxy must never wait for the full SSE stream before
  returning to client. Tested with a benchmark (see §10).
- **request-id capture**: extracted from upstream response headers as soon
  as headers arrive (before stream end), so dashboard shows it immediately.
- **Upstream failure**: `upstream_unreachable` / 4xx / 5xx all still write
  a record (with `error` or non-2xx status).

## 6. Storage format

### Disk layout

```
~/.ccs-proxy/                    # default; overridable via --data-dir
├── sessions/
│   ├── <session-id>/            # session id = ISO8601 + 8-char random
│   │   ├── meta.json
│   │   ├── 0001.json
│   │   ├── 0002.json
│   │   └── ...
│   └── ...
└── logs/
    └── ccs-proxy.log            # tracing output, rotates at 10MB
```

Directory permissions: `0700`. File permissions: `0600`.

### `meta.json`

```json
{
  "session_id": "2026-05-28T17-12-34-000Z-a1b2c3d4",
  "provider": "claude",
  "upstream": "https://api.anthropic.com",
  "proxy_port": 41001,
  "api_port": 41501,
  "started_at": "2026-05-28T17:12:34.000Z",
  "ended_at": null,
  "request_count": 17,
  "schema_version": 1
}
```

`ended_at` filled in on graceful shutdown; left null on crash (dashboard
shows "unclean").

### `<seq>.json`

```json
{
  "seq": 1,
  "session_id": "...",
  "request_id": "req_01H8...",
  "started_at": "2026-05-28T17:12:35.123Z",
  "ended_at":   "2026-05-28T17:12:38.456Z",
  "duration_ms": 3333,
  "ttft_ms": 412,

  "request": {
    "method": "POST",
    "path": "/v1/messages",
    "headers": { "content-type": "application/json", "authorization": "<redacted>", ... },
    "body": { /* original JSON */ }
  },
  "response": {
    "status": 200,
    "headers": { "anthropic-request-id": "req_...", "content-type": "text/event-stream", ... },
    "body_reassembled": { /* reassembled final message */ },
    "raw_sse_frames_count": 142
  },
  "usage": {
    "input_tokens": 1024,
    "output_tokens": 256,
    "cache_creation_input_tokens": 0,
    "cache_read_input_tokens": 0
  },
  "model": "claude-sonnet-4-6",
  "error": null,
  "schema_version": 1
}
```

`error` shape when set:
```json
{ "kind": "upstream_unreachable" | "tls_handshake_failed" | "sse_truncated" | "reassemble_failed", "message": "..." }
```

Raw SSE frames are **not** stored in v1 (space). On `reassemble_failed`,
the first 64KB of accumulated SSE text is stored in `raw_sse_text` for
debugging.

## 7. HTTP API surface

| Method + Path | Returns |
|---|---|
| `GET /api/health` | `{status, provider, upstream, session_id, started_at, request_count, store: "ok" \| "degraded"}` |
| `GET /api/sessions` | `[{session_id, provider, upstream, started_at, ended_at, request_count, total_input_tokens, total_output_tokens}, ...]` (most recent first) |
| `GET /api/sessions/:sid` | session meta + request summary array |
| `GET /api/requests/:sid/:seq` | full `<seq>.json` |
| `GET /api/stream` | SSE; events below |
| `GET /` | embedded `index.html` |
| `GET /app.js`, `/style.css`, `/assets/*` | embedded static |

### `/api/stream` events

```
event: request_started
data: {"session_id":"...","seq":42,"started_at":"...","model":"...","request_preview":{...}}

event: request_completed
data: {"session_id":"...","seq":42,"duration_ms":3333,"status":200,"usage":{...},"request_id":"req_..."}
```

Frontend pattern: on `request_started` add a placeholder row; on
`request_completed` patch the row in place (no full re-fetch).

CORS: default same-origin (no headers needed). `--cors-allow=<origin>`
opens specific origins; emits warning log.

## 8. Dashboard MVP

Single page, vanilla JS (no build step), ≤ 800 lines JS + ≤ 200 lines CSS.
No React/Vue/Svelte/Vite. If a feature requires more than that, cut the
feature, not the constraint.

| ✅ v1 | ❌ v2+ |
|---|---|
| Header: session id, provider, upstream | Theme toggle |
| Left pane: request list for current session (time, model, status, tokens, request-id) | Cross-session navigation |
| Click row → right pane with tabs (Overview / Request / Response / Raw Headers / Usage) | Turn-to-turn diff |
| Overview: duration / TTFT / status / model / tokens / request-id (with copy button) | Copy as cURL |
| Request: system prompt / messages / tools (collapsible JSON) | tool_use ↔ tool_result paired sequence diagram |
| Response: reassembled body (content blocks, tool calls, stop_reason) | Latency sparkline |
| Raw Headers: full request + response headers (redacted) | Per-model filter |
| Live updates via `/api/stream` SSE | Error-only secondary view |
| Error rows shown with red background | Cross-session aggregate stats |

Dev mode: feature flag `dev-fs-assets` reads `web/` from disk instead of
embedded — avoids rebuilding Rust on every JS edit.

## 9. Error handling

| Class | Behavior |
|---|---|
| Upstream unreachable (DNS / TCP / TLS) | 502 to client; record with `error.kind=upstream_unreachable`; no retry |
| Upstream non-2xx | Forwarded as-is; record stores actual `status` |
| TLS handshake failure | `error.kind=tls_handshake_failed`; same as unreachable for client |
| SSE truncated mid-stream | Already-sent chunks stay; reassembler gets truncated; record marked `partial=true`, `error.kind=sse_truncated` |
| Reassembler parse error | Doesn't affect client stream; record `body_reassembled=null` + first 64KB of raw SSE in `raw_sse_text` |
| Disk write failure | Warn log; drop record; client unaffected. 10 consecutive failures → store writer task exits cleanly; `sse_tap` switches to forward-only mode (no capture) for the remainder of this `serve()` lifetime. Health endpoint reports `store: degraded`. |
| API server task panic | Tokio task isolation; proxy server unaffected; frontend SSE auto-reconnects |
| Port bind failure at startup | `Err` returned from `serve()`; caller (cc-switch daemon per Spec B) handles via spawn_failed + exponential backoff |
| `--data-dir` missing | Auto-`mkdir -p`; failure → error before bind |
| Corrupt `meta.json` / `<seq>.json` | Skipped at index load with warn; API returns 404 |

## 10. Security

- Default redaction: `authorization`, `x-api-key`, `anthropic-api-key`,
  `cookie`, `set-cookie` → `<redacted>` before disk write. `request-id`
  and other `anthropic-*` / `openai-*` metadata headers preserved.
- Both ports bind `127.0.0.1` only. **Code-level invariant** (no env-var
  override) to prevent accidental exposure.
- No auth on dashboard; relies on loopback-only bind.
- Body redaction: top-level JSON keys matching `api_key`, `apiKey`,
  `auth_token`, `authToken`, `access_token`, `accessToken`, `token`,
  `secret`, `password` (case-insensitive exact match) are replaced with
  `<redacted>` before disk write. Nested keys not scanned (LLM
  request/response bodies rarely embed secrets nested).
- Directory permissions `0700`, file permissions `0600`.
- TLS verification on upstream connection enabled (reqwest default).

## 11. Testing strategy

| Layer | Coverage | Tooling |
|---|---|---|
| Unit | reassembler (claude + codex), redact rules, ProviderKind routing, extract logic | `cargo test --lib`, SSE fixtures |
| Integration | mock upstream + `serve()` + reqwest client → record correctness, header forwarding, streaming non-delay | `tests/` with `wiremock` |
| End-to-end | Full request/response cycle, usage extraction, request-id capture | `tests/e2e.rs` |
| API | All `/api/*` endpoints | `tests/api_tests.rs` |
| Performance | 1000 concurrent streaming requests; p99 TTFT increase < 5ms vs direct | `criterion` benchmark |
| Dashboard | v1 manual smoke; no automated UI tests | — |

Coverage target: ≥ 80% on lib modules.

## 12. Observability

- `tracing` to `~/.ccs-proxy/logs/ccs-proxy.log`, default `info`.
  `RUST_LOG=ccs_proxy=debug` for debugging.
- `/api/health` exposes uptime + current request_count — used by
  cc-switch daemon (Spec B) for liveness probe.
- No Prometheus / OpenTelemetry in v1.

## 13. Build & distribution

- MSRV: Rust 1.88+, edition 2024 (matches cc-switch).
- `cargo build --release` → single `ccs-proxy` binary, < 15MB.
- No npm / yarn / node_modules / docker / submodules.
- Published to crates.io as `ccs-proxy`.

## 14. Spec B follow-up (separate brainstorming)

The cc-switch daemon integration (next spec) will cover:

- Commands: `cc daemon start | stop | status | restart` (no `reload`;
  restart instead).
- Internal: `cc daemon` maintains one `ProxyHandle` per (provider,
  upstream) pair derived from cc-switch's `configurations.json`. Eager
  spawn at daemon start. Supervises by polling task liveness; restarts
  crashed handles. Writes `~/.cc-switch/daemon-state.json` for `cc use`
  to consume.
- `cc use <alias>` / `cc codex use <alias>`: reads state file; if matched,
  rewrites `*_BASE_URL` env to `http://127.0.0.1:<proxy_port>`; falls
  back to direct mode silently (with one-line stderr hint) when daemon is
  off or upstream not registered. When PID file exists but process is
  dead: transparent re-fork of daemon with ~3s timeout, then continue.
- Anti-entropy: on daemon start, kill orphan `ccs-proxy` processes from
  prior unclean shutdowns before binding ports.
- Unix only in v1.

## 15. Approved design decisions (reference)

- Crate / binary name: **`ccs-proxy`**
- Code location: **standalone crate** (independent versioning, depended
  on by cc-switch via Cargo)
- Port topology: **two independent ports** (proxy + api/dashboard)
- Frontend distribution: **embedded** via rust-embed (single-binary)
- Tech stack: **axum + tokio + simplified per-request JSON storage** (no
  content-addressed dedup in v1)
- Session = one `serve()` lifetime
- Dashboard scope: **current session only** in v1
- No frontend build pipeline (vanilla JS / CSS)
- Disk-write circuit breaker at **10 consecutive failures**
- API stability frozen for v1: `serve()` signature + 4 public
  `ProxyHandle` fields; everything else can change
- Unix only; Windows reports unsupported

## 16. Open work to verify before implementation

- Confirm `ccglass` currently surfaces response `request-id` headers in
  its dashboard; if not, file an upstream issue (orthogonal to this spec —
  cc-switch will rely on `ccs-proxy`, not the Node binary).
- Confirm `axum 0.8` SSE handling pattern is what we expect (write a tiny
  spike before locking pin in Cargo.toml).
- Decide concrete crate version pins (deferred to implementation plan).
- Confirm `wiremock` 0.6+ supports streaming SSE responses for integration
  tests; if not, fall back to a hand-written 50-line hyper mock.
