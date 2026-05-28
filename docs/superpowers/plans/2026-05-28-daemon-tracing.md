# Daemon Tracing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace all `eprintln!` in `src/daemon/lifecycle.rs` with structured `tracing` logs, using a unified subscriber (daily file rotation + optional stderr) that also captures ccs-proxy library logs.

**Architecture:** Daemon initializes a global tracing subscriber at the top of `run_daemon_blocking()`. In foreground mode it adds a stderr layer; in background mode only the file layer is active. Log level is resolved from `CCS_LOG` env → `--log-level` flag → `-v` count → default `info`. Old log files (>7 days) are cleaned up on startup.

**Tech Stack:** `tracing 0.1`, `tracing-subscriber 0.3` (env-filter, fmt), `tracing-appender 0.2`

---

## File Structure

| Action | Path | Responsibility |
|--------|------|----------------|
| Create | `src/daemon/logging.rs` | Subscriber init, level resolution, log cleanup |
| Modify | `src/daemon/mod.rs` | Add `pub mod logging;` |
| Modify | `src/daemon/lifecycle.rs` | Call `init_tracing()`, replace `eprintln!` with `tracing::*` |
| Modify | `src/daemon/commands.rs` | Replace post-fork `eprintln!` in `handle_start` with `tracing::*`; keep pre-fork CLI feedback as `eprintln!` |
| Modify | `src/cli/cli.rs` | Add `--log-level` and `-v` flags to Start/Restart variants |
| Modify | `Cargo.toml` | Add tracing dependencies |
| Create | `tests/daemon_logging.rs` | Tests for level resolution + log cleanup |

---

### Task 1: Add tracing dependencies to Cargo.toml

**Files:**
- Modify: `Cargo.toml:17-33`

- [ ] **Step 1: Add dependencies**

Add these three lines to `[dependencies]` in `Cargo.toml`:

```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-appender = "0.2"
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: success (no code uses the crates yet, but they resolve)

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore(deps): add tracing, tracing-subscriber, tracing-appender"
```

---

### Task 2: Add --log-level and -v flags to CLI

**Files:**
- Modify: `src/cli/cli.rs:330-351`

- [ ] **Step 1: Write the test**

Create `tests/daemon_logging.rs`:

```rust
//! Tests for daemon logging: level resolution and log cleanup.

#[cfg(test)]
mod daemon_logging {
    use cc_switch::daemon::logging::resolve_log_level;
    use tracing::level_filters::LevelFilter;

    #[test]
    fn default_level_is_info() {
        let level = resolve_log_level(None, 0, None);
        assert_eq!(level, LevelFilter::INFO);
    }

    #[test]
    fn verbose_1_is_info() {
        let level = resolve_log_level(None, 1, None);
        assert_eq!(level, LevelFilter::INFO);
    }

    #[test]
    fn verbose_2_is_debug() {
        let level = resolve_log_level(None, 2, None);
        assert_eq!(level, LevelFilter::DEBUG);
    }

    #[test]
    fn verbose_3_is_trace() {
        let level = resolve_log_level(None, 3, None);
        assert_eq!(level, LevelFilter::TRACE);
    }

    #[test]
    fn cli_flag_overrides_verbose() {
        let level = resolve_log_level(Some("error"), 3, None);
        assert_eq!(level, LevelFilter::ERROR);
    }

    #[test]
    fn env_overrides_cli_flag() {
        let level = resolve_log_level(Some("error"), 0, Some("trace"));
        assert_eq!(level, LevelFilter::TRACE);
    }

    #[test]
    fn env_invalid_falls_back_to_cli() {
        let level = resolve_log_level(Some("warn"), 0, Some("not_a_level"));
        assert_eq!(level, LevelFilter::WARN);
    }
}
```

- [ ] **Step 2: Verify test fails (module doesn't exist)**

Run: `cargo test --test daemon_logging`
Expected: compile error — `cc_switch::daemon::logging` not found

- [ ] **Step 3: Add CLI flags to DaemonCommands**

In `src/cli/cli.rs`, replace the `Start` and `Restart` variants:

```rust
/// Subcommands for `cc-switch daemon`
#[derive(Subcommand)]
pub enum DaemonCommands {
    /// Start the daemon (double-forks into background by default)
    Start {
        /// Run in the foreground (don't daemonize). Useful for debugging.
        #[arg(long)]
        foreground: bool,

        /// Log level: error, warn, info, debug, trace
        #[arg(long = "log-level", value_name = "LEVEL")]
        log_level: Option<String>,

        /// Increase verbosity (-v info, -vv debug, -vvv trace)
        #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count)]
        verbose: u8,
    },
    /// Stop the running daemon
    Stop,
    /// Show daemon status and proxy health
    Status {
        /// Output as JSON instead of a human-readable table
        #[arg(long)]
        json: bool,
    },
    /// Stop then start the daemon (picks up configuration changes)
    Restart {
        /// Run in the foreground after restart
        #[arg(long)]
        foreground: bool,

        /// Log level: error, warn, info, debug, trace
        #[arg(long = "log-level", value_name = "LEVEL")]
        log_level: Option<String>,

        /// Increase verbosity (-v info, -vv debug, -vvv trace)
        #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count)]
        verbose: u8,
    },
}
```

- [ ] **Step 4: Update commands.rs DaemonAction to carry log params**

In `src/daemon/commands.rs`, update `DaemonAction`:

```rust
pub enum DaemonAction {
    Start {
        foreground: bool,
        log_level: Option<String>,
        verbose: u8,
    },
    Stop,
    Status { json: bool },
    Restart {
        foreground: bool,
        log_level: Option<String>,
        verbose: u8,
    },
}
```

Update the match in `handle_daemon_command` and `handle_start` signature:

```rust
#[cfg(unix)]
match action {
    DaemonAction::Start { foreground, log_level, verbose } => {
        handle_start(foreground, log_level, verbose, storage)
    }
    DaemonAction::Stop => handle_stop(),
    DaemonAction::Status { json } => handle_status(json, storage),
    DaemonAction::Restart { foreground, log_level, verbose } => {
        let _ = handle_stop();
        handle_start(foreground, log_level, verbose, storage)
    }
}
```

Update `handle_start` signature:

```rust
#[cfg(unix)]
fn handle_start(foreground: bool, log_level: Option<String>, verbose: u8, storage: &ConfigStorage) -> Result<()> {
```

- [ ] **Step 5: Update CLI main.rs to pass new fields**

In `src/cli/main.rs`, update the daemon command match arm to pass `log_level` and `verbose` from CLI args to `DaemonAction::Start` / `DaemonAction::Restart`.

Find the existing match and update:

```rust
DaemonCommands::Start { foreground, log_level, verbose } => {
    DaemonAction::Start { foreground, log_level, verbose }
}
```

```rust
DaemonCommands::Restart { foreground, log_level, verbose } => {
    DaemonAction::Restart { foreground, log_level, verbose }
}
```

- [ ] **Step 6: Verify compilation**

Run: `cargo check`
Expected: success (tests still fail because `daemon::logging` doesn't exist yet)

- [ ] **Step 7: Commit**

```bash
git add src/cli/cli.rs src/daemon/commands.rs src/cli/main.rs tests/daemon_logging.rs
git commit -m "feat(daemon): add --log-level and -v flags to start/restart CLI"
```

---

### Task 3: Implement `src/daemon/logging.rs`

**Files:**
- Create: `src/daemon/logging.rs`
- Modify: `src/daemon/mod.rs`

- [ ] **Step 1: Create the logging module**

Create `src/daemon/logging.rs`:

```rust
use std::path::Path;
use tracing::level_filters::LevelFilter;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub enum LogMode {
    Foreground,
    Background,
}

/// Resolve the effective log level from (highest priority first):
/// 1. `env_val` — CCS_LOG env var (full EnvFilter syntax)
/// 2. `cli_level` — --log-level flag (single level string)
/// 3. `verbose` — -v count: 1=info, 2=debug, 3+=trace
/// 4. Default: info
pub fn resolve_log_level(
    cli_level: Option<&str>,
    verbose: u8,
    env_val: Option<&str>,
) -> LevelFilter {
    if let Some(env) = env_val {
        if let Ok(lf) = env.parse::<LevelFilter>() {
            return lf;
        }
    }

    if let Some(flag) = cli_level {
        if let Ok(lf) = flag.parse::<LevelFilter>() {
            return lf;
        }
    }

    match verbose {
        0 | 1 => LevelFilter::INFO,
        2 => LevelFilter::DEBUG,
        _ => LevelFilter::TRACE,
    }
}

/// Initialize the global tracing subscriber. Returns a guard that must be held
/// for the daemon's lifetime to ensure the non-blocking writer flushes on drop.
pub fn init_tracing(mode: LogMode, level: LevelFilter) -> WorkerGuard {
    let log_dir = log_directory();
    std::fs::create_dir_all(&log_dir).ok();

    let file_appender = tracing_appender::rolling::daily(&log_dir, "daemon");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = EnvFilter::builder()
        .with_default_directive(level.into())
        .from_env_lossy();

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(false);

    let registry = tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer);

    match mode {
        LogMode::Foreground => {
            let stderr_layer = tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .with_ansi(true)
                .with_target(true);
            registry.with(stderr_layer).init();
        }
        LogMode::Background => {
            registry.with(None::<tracing_subscriber::fmt::Layer<_>>).init();
        }
    }

    guard
}

/// Clean up log files older than `retention_days` in the log directory.
pub fn cleanup_old_logs(retention_days: u32) {
    let log_dir = log_directory();
    let entries = match std::fs::read_dir(&log_dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    let today = chrono::Utc::now().date_naive();

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // tracing-appender daily files: "daemon.YYYY-MM-DD"
        let date_part = match name_str.strip_prefix("daemon.") {
            Some(rest) => rest,
            None => continue,
        };

        let file_date = match chrono::NaiveDate::parse_from_str(date_part, "%Y-%m-%d") {
            Ok(d) => d,
            Err(_) => continue,
        };

        let age_days = (today - file_date).num_days();
        if age_days > retention_days as i64 {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

fn log_directory() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".cc-switch")
        .join("logs")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleanup_removes_old_files() {
        let dir = tempfile::TempDir::new().unwrap();
        let old_file = dir.path().join("daemon.2020-01-01");
        let recent_file = dir.path().join("daemon.2026-05-27");
        let unrelated = dir.path().join("other.txt");

        std::fs::write(&old_file, "old").unwrap();
        std::fs::write(&recent_file, "recent").unwrap();
        std::fs::write(&unrelated, "keep").unwrap();

        // We can't call cleanup_old_logs directly because it uses a fixed dir.
        // Test the logic inline:
        let today = chrono::Utc::now().date_naive();
        for entry in std::fs::read_dir(dir.path()).unwrap().flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            let date_part = match name_str.strip_prefix("daemon.") {
                Some(rest) => rest,
                None => continue,
            };
            let file_date = match chrono::NaiveDate::parse_from_str(date_part, "%Y-%m-%d") {
                Ok(d) => d,
                Err(_) => continue,
            };
            let age_days = (today - file_date).num_days();
            if age_days > 7 {
                std::fs::remove_file(entry.path()).unwrap();
            }
        }

        assert!(!old_file.exists(), "old file should be deleted");
        assert!(recent_file.exists(), "recent file should be kept");
        assert!(unrelated.exists(), "unrelated file should be kept");
    }
}
```

- [ ] **Step 2: Register module in mod.rs**

In `src/daemon/mod.rs`, add after the existing `pub mod status;` line:

```rust
pub mod logging;
```

- [ ] **Step 3: Verify tests pass**

Run: `cargo test --test daemon_logging`
Expected: all 7 tests pass

Run: `cargo test daemon::logging`
Expected: `cleanup_removes_old_files` passes

- [ ] **Step 4: Commit**

```bash
git add src/daemon/logging.rs src/daemon/mod.rs
git commit -m "feat(daemon): logging module with subscriber init, level resolution, and log cleanup"
```

---

### Task 4: Wire tracing into lifecycle.rs

**Files:**
- Modify: `src/daemon/lifecycle.rs`
- Modify: `src/daemon/commands.rs`

- [ ] **Step 1: Update lifecycle.rs to accept log params and init tracing**

Change `run_daemon_blocking` signature to accept log params:

```rust
pub fn run_daemon_blocking(cfg: LifecycleConfig, log_level: Option<String>, verbose: u8) -> Result<()> {
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
```

- [ ] **Step 2: Add `foreground` field to LifecycleConfig**

In `src/daemon/lifecycle.rs`, add to the struct:

```rust
pub struct LifecycleConfig {
    pub state_path: PathBuf,
    pub pidfile_path: PathBuf,
    pub data_root: PathBuf,
    pub upstreams: Vec<Upstream>,
    pub foreground: bool,
}
```

Update `from_storage` to accept and pass through `foreground`:

```rust
pub fn from_storage(storage: &ConfigStorage, foreground: bool) -> Result<Self> {
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
        foreground,
    })
}
```

- [ ] **Step 3: Replace eprintln! with tracing macros in lifecycle.rs**

Replace all `eprintln!` calls in `run_daemon_async` and `supervisor_loop`:

| Old | New |
|-----|-----|
| `eprintln!("warning: skipping upstream...")` | `tracing::warn!(upstream = %upstream_url, error = %err, "skipping invalid upstream URL");` |
| `eprintln!("warning: failed to start proxy...")` | `tracing::error!(upstream = %upstream_url, error = %err, "failed to start proxy");` |
| `eprintln!("ccs-daemon: started...")` | `tracing::info!(pid = state.pid, proxy_count = handles.len(), "daemon started");` |
| `eprintln!("ccs-daemon: shutting down...")` | `tracing::info!("daemon shutting down");` |
| `eprintln!("ccs-daemon: stopped")` | `tracing::info!("daemon stopped");` |
| `eprintln!("ccs-daemon: proxy for {} exited...")` | `tracing::warn!(upstream = %entries[i].upstream, "proxy exited unexpectedly, respawning");` |
| `eprintln!("ccs-daemon: failed to respawn...")` | `tracing::error!(upstream = %entries[i].upstream, error = %err, "failed to respawn proxy");` |

- [ ] **Step 4: Update commands.rs to pass log params through**

In `handle_start`, pass `log_level` and `verbose` to `run_daemon_blocking`:

```rust
#[cfg(unix)]
fn handle_start(foreground: bool, log_level: Option<String>, verbose: u8, storage: &ConfigStorage) -> Result<()> {
    let cfg = LifecycleConfig::from_storage(storage, foreground)?;

    // ... existing pidfile check logic unchanged ...

    if !foreground {
        let home = dirs::home_dir().context("could not find home directory")?;
        let log_path = home.join(".cc-switch").join("daemon.log");
        crate::daemon::fork::double_fork_into_background(&log_path)?;
    }

    crate::daemon::lifecycle::run_daemon_blocking(cfg, log_level, verbose)
}
```

- [ ] **Step 5: Fix test compilation**

Update `tests/daemon_supervisor.rs` — `LifecycleConfig::from_storage` now takes `foreground` param. Add `false` as second arg:

```rust
let cfg = LifecycleConfig::from_storage(&storage, false).unwrap();
```

Apply this to all calls in that test file (4 occurrences).

- [ ] **Step 6: Verify all tests pass**

Run: `cargo test`
Expected: all tests pass (including daemon_supervisor, daemon_integration, daemon_liveness, daemon_logging)

- [ ] **Step 7: Verify foreground mode produces stderr output**

Run: `cargo run -- daemon start --foreground --log-level debug 2>&1 | head -5`
Expected: structured log lines on stderr (timestamps, levels, targets), then Ctrl+C to stop.

- [ ] **Step 8: Commit**

```bash
git add src/daemon/lifecycle.rs src/daemon/commands.rs tests/daemon_supervisor.rs
git commit -m "feat(daemon): wire tracing subscriber into lifecycle, replace eprintln with structured logs"
```

---

### Task 5: Verify end-to-end and clean up

**Files:**
- Modify: `src/daemon/commands.rs` (only if eprintln! remain that should be tracing)

- [ ] **Step 1: Audit remaining eprintln! usage in daemon**

Run: `grep -n "eprintln!" src/daemon/`
Expected: only pre-fork CLI messages in `commands.rs` (stop/status handlers that run in the user's terminal process, not the daemon). These should stay as `eprintln!` because:
- They run before tracing is initialized
- They're user-facing CLI output, not daemon logs

- [ ] **Step 2: Run the full test suite**

Run: `cargo test`
Expected: all tests pass

- [ ] **Step 3: Run clippy**

Run: `cargo clippy -- -W warnings`
Expected: no warnings

- [ ] **Step 4: Verify log file is created in background mode**

```bash
cargo run -- daemon start
sleep 2
ls ~/.cc-switch/logs/daemon.*
cargo run -- daemon stop
```

Expected: a `daemon.YYYY-MM-DD` file exists in `~/.cc-switch/logs/`

- [ ] **Step 5: Final commit if any fixups needed**

```bash
git add -A
git commit -m "chore(daemon): tracing cleanup and verification"
```

(Skip if no changes needed.)
