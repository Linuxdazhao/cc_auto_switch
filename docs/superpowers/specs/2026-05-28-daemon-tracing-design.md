# Daemon Tracing — Design Spec

**Date**: 2026-05-28  
**Status**: Draft  
**Scope**: Replace `eprintln!` in `src/daemon/` with structured `tracing` logs; unify daemon + embedded ccs-proxy under a single subscriber with daily rotation and automatic cleanup.

---

## 1. Goal

Provide production-grade observability for the cc-switch daemon:

- Structured, leveled logging (trace/debug/info/warn/error)
- Daily file rotation with 7-day retention
- Unified output for daemon supervisor + all embedded ccs-proxy instances
- User-controllable verbosity via CLI flags and environment variable

## 2. Architecture

```
cc daemon start [--log-level <level> | -v/-vv/-vvv]
        │
        ▼
┌─────────────────────────────────────────────┐
│  init_tracing(mode, level)                  │
│                                             │
│  foreground: stderr layer + file layer      │
│  background: file layer only                │
│                                             │
│  file layer: daily rolling appender         │
│    dir:  ~/.cc-switch/logs/                 │
│    name: daemon-YYYY-MM-DD.log              │
│                                             │
│  cleanup: delete files > 7 days old         │
└─────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────┐
│  run_daemon_async()                         │
│    ├── tracing::info!("daemon started")     │
│    ├── ccs_proxy::serve()  ← uses global    │
│    │     subscriber (no init of its own)    │
│    └── supervisor_loop()                    │
└─────────────────────────────────────────────┘
```

## 3. Subscriber Configuration

### 3.1 Initialization

A new function `src/daemon/logging.rs::init_tracing(mode, level)`:

```rust
pub enum LogMode { Foreground, Background }

pub fn init_tracing(mode: LogMode, level: LevelFilter) -> tracing_appender::non_blocking::WorkerGuard;
```

- Returns a `WorkerGuard` that the caller must hold for the daemon's lifetime (ensures flush on exit).
- Must be called before any `tracing::*` macro fires.

### 3.2 Layers

| Layer | When | Format | Target |
|-------|------|--------|--------|
| File | Always | `{timestamp} {level} {target}: {message} {fields}` (compact) | `~/.cc-switch/logs/daemon-YYYY-MM-DD.log` |
| Stderr | Foreground only | Pretty with ANSI colors | stderr |

### 3.3 Level Resolution

Priority (highest wins):

1. `CCS_LOG` env var — full `EnvFilter` syntax (e.g. `ccs_proxy=trace,cc_switch::daemon=debug`)
2. `--log-level <level>` CLI flag — single level for all targets
3. `-v` / `-vv` / `-vvv` flags — maps to info / debug / trace
4. Default: `info`

### 3.4 ccs-proxy as Library

When `ccs_proxy::serve()` is called from the daemon, it must NOT initialize its own subscriber. The current `src/bin/ccs-proxy.rs` init is fine (it only runs for the standalone binary). The library code already uses `tracing::warn!` etc. — those will just work once the daemon sets a global subscriber.

## 4. Log Rotation

### 4.1 Daily Rolling

Use `tracing_appender::rolling::daily("~/.cc-switch/logs/", "daemon")` which produces files like:

```
daemon.2026-05-28.log
daemon.2026-05-29.log
```

### 4.2 Retention Cleanup

On daemon startup, scan `~/.cc-switch/logs/` and delete any `daemon.*.log` file whose embedded date is >7 days ago. Implementation:

```rust
fn cleanup_old_logs(log_dir: &Path, retention_days: u32) {
    // Parse date from filename: "daemon.YYYY-MM-DD.log"
    // Delete if (today - file_date) > retention_days
}
```

This runs once at startup, synchronously, before the tracing subscriber is active (so it uses `eprintln!` for any cleanup errors — acceptable since it's a one-shot operation before the main loop).

## 5. Changes to Existing Code

### 5.1 New File: `src/daemon/logging.rs`

- `init_tracing(mode, level) -> WorkerGuard`
- `cleanup_old_logs(log_dir, retention_days)`
- `resolve_log_level(cli_level, cli_verbose, env_var) -> LevelFilter`

### 5.2 Modified: `src/daemon/lifecycle.rs`

- `run_daemon_blocking()` calls `init_tracing()` first, holds guard
- All `eprintln!` → `tracing::info!` / `tracing::warn!` / `tracing::error!`

### 5.3 Modified: `src/daemon/commands.rs`

- `eprintln!` in start/stop/status handlers → keep as `eprintln!` for CLI user feedback (these run in the CLI process, not the daemon)
- Exception: messages inside `handle_start()` after fork point → `tracing::*`

### 5.4 Modified: `src/cli/cli.rs`

- Add `--log-level` option and `-v` verbosity flag to `DaemonCommands::Start` and `DaemonCommands::Restart`

### 5.5 Modified: `Cargo.toml` (workspace root)

Add dependencies:
```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-appender = "0.2"
```

### 5.6 NOT Modified: `ccs-proxy/src/bin/ccs-proxy.rs`

Standalone binary keeps its own subscriber init — no change needed.

## 6. CLI Interface

```
cc-switch daemon start [OPTIONS]

Options:
  --log-level <LEVEL>   Log level: error, warn, info, debug, trace [default: info]
  -v, -vv, -vvv        Increase verbosity (info, debug, trace)
  --foreground          Run in foreground (also logs to stderr)

Environment:
  CCS_LOG               Override log level filter (RUST_LOG syntax, highest priority)
```

## 7. Log Examples

```
2026-05-28T19:30:01.234Z  INFO cc_switch::daemon::lifecycle: daemon started pid=78188 proxies=4
2026-05-28T19:30:01.250Z  INFO cc_switch::daemon::lifecycle: proxy started upstream="https://api.anthropic.com" port=41001 api_port=41501
2026-05-28T19:30:01.251Z  INFO cc_switch::daemon::lifecycle: proxy started upstream="https://glm.example.com/v1" port=41002 api_port=41502
2026-05-28T19:30:31.234Z DEBUG cc_switch::daemon::lifecycle: supervisor tick all_healthy=true
2026-05-28T19:31:01.234Z  WARN ccs_proxy::proxy::forward: upstream request failed kind="upstream_timeout" upstream="https://glm.example.com/v1"
2026-05-28T20:00:00.000Z  INFO cc_switch::daemon::lifecycle: daemon shutting down signal=SIGTERM
```

## 8. Testing

- Unit test: `resolve_log_level()` priority logic (env > flag > verbose > default)
- Unit test: `cleanup_old_logs()` with tempdir containing old + new files
- Integration: daemon foreground mode produces expected log output to stderr (capture with `assert_cmd` or manual)

## 9. Out of Scope

- JSON structured output (deferred to aggregation dashboard spec)
- Remote log shipping
- Per-proxy separate log files (decided against in approach selection)
- Log level changes at runtime (hot reload)
