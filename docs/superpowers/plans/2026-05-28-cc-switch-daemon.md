# Implementation Plan — Spec B: `cc daemon` + `cc use` auto-wrap

**Date**: 2026-05-28
**Spec**: `docs/superpowers/specs/2026-05-28-cc-switch-daemon-design.md`
**Branch**: `feat/ccs-proxy-v1` (extend the existing branch — same PR)
**Execution mode**: subagent-driven-development (one implementer + two reviewers per task)

## Conventions for every task

- **Working dir** for all subagents: `/Users/jingzhao/Work/github/cc_auto_switch/.claude/worktrees/ccs-proxy-v1/` (cc-switch root). When a task touches the ccs-proxy crate, use `ccs-proxy/` subdirectory.
- **Run tests** at the end of every task. Commit only when `cargo test --workspace` passes.
- **clippy.toml rules** that already bite us elsewhere on this repo — apply preemptively:
  - `HashMap` / `HashSet` disallowed → use `BTreeMap` / `BTreeSet`.
  - `min-ident-chars-threshold = 2` (allowlist `i,j,x,y,z,w,n,id`).
  - `too-many-lines-threshold = 50` → split long functions early.
  - `arithmetic-side-effects-allowed = []` → use checked / saturating arith.
  - `disallowed-names = ["test", "temp", "dummy", ".."]` → name variables for what they hold.
- **Don't break existing tests.** cc-switch's existing 170+ tests must stay green.
- **Commit format**: Conventional Commits. `feat(daemon): ...`, `fix(daemon): ...`, `test(daemon): ...`, `docs(daemon): ...`. Scope `daemon` for everything in `src/daemon/`; scope `cli` for changes to `src/cli/main.rs` that wire `cc use`.
- **No `--no-verify`**. Pre-commit hook runs the full suite.

## Task list (12 tasks)

Tasks form a dependency chain. Run them strictly in order — each downstream task assumes the prior one's commit is on HEAD.

---

### Task 1 — Wire the `ccs-proxy` path dep into cc-switch

**Goal**: Add `ccs-proxy = { path = "ccs-proxy" }` to cc-switch's `Cargo.toml` so the daemon code can call `ccs_proxy::serve()`. Also add the few runtime deps the daemon will need that cc-switch doesn't have yet: `tokio` (full features, matching ccs-proxy's pin), `ureq` for blocking status probes (keeps the `status` command free of an async runtime requirement), and `chrono` (matches ccs-proxy's timestamp format).

**Why a separate task**: Adding deps changes build time substantially. Keeping it isolated makes the cause obvious if CI suddenly slows.

**Acceptance**:
- `Cargo.toml` updated.
- `cargo build` succeeds at workspace root.
- `cargo test --workspace` still passes (we haven't added behavior yet).
- No existing tests altered.

**Files touched**: `Cargo.toml`.

**Implementer should NOT**: create the `src/daemon/` module yet. That's Task 2.

**Commit**: `chore(deps): add ccs-proxy path dep + tokio/ureq/chrono for daemon`

---

### Task 2 — Skeleton `src/daemon/` module + Windows guard

**Goal**: Create the daemon module structure with empty stubs and the Windows error. No real logic yet — this gives downstream tasks a stable module surface.

**Files to create**:

```
src/daemon/mod.rs           # pub uses; re-export commands::handle_daemon_command
src/daemon/commands.rs      # enum DaemonAction { Start, Stop, Status, Restart }; handle() match
src/daemon/state.rs         # struct DaemonState, ProxyEntry; load/save (atomic write tmp+rename)
src/daemon/pidfile.rs       # acquire/release/read/check_alive — stubs that return placeholder errors
src/daemon/lifecycle.rs     # async fn run_daemon() -> Result<()> — stub returning Ok(())
src/daemon/fork.rs          # #[cfg(unix)] double_fork() — stub returning Ok(())
                            # #[cfg(not(unix))] returns Err with the Windows message
src/daemon/status.rs        # fn format_status(state, runtime_info) -> String — stub
```

**Acceptance**:
- `cargo check` passes.
- `mod daemon;` added to `src/lib.rs`.
- `pub use daemon::handle_daemon_command;` re-exported.
- Each file ≤ 30 LoC (skeletons only).
- On Windows, `handle_daemon_command(DaemonAction::Start, _)` returns `Err("cc daemon is Unix-only in v1 — run `ccs-proxy serve` directly")`. Same error for all four actions.

**Files touched**: `src/lib.rs`, `src/daemon/**`.

**Commit**: `feat(daemon): scaffold module structure + Windows unsupported guard`

---

### Task 3 — `DaemonState` + `ProxyEntry` types and atomic IO

**Goal**: Implement the data types matching Spec §6 plus atomic load/save. TDD: write the round-trip tests first.

**Schema** (matches §6 exactly):

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DaemonState {
    pub schema_version: u32,        // always 1
    pub pid: u32,
    pub started_at: String,         // RFC3339
    pub stopped_at: Option<String>, // RFC3339
    pub data_root: PathBuf,
    pub proxies: Vec<ProxyEntry>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ProxyEntry {
    pub provider: String,           // "claude" for v1
    pub upstream: String,           // full URL string
    pub proxy_port: u16,
    pub api_port: u16,
    pub data_dir: PathBuf,
    pub started_at: String,
    pub restart_count: u32,
}
```

**Behavior**:

- `DaemonState::load(path)` → `Result<Option<DaemonState>>`. Missing file → `Ok(None)`. Corrupt JSON → `Err` with file path in the message.
- `DaemonState::save(&self, path)` → `Result<()>`. Atomic: write `path.tmp`, fsync, rename to `path`. `0600` perms on Unix.
- `DaemonState::find_proxy(&self, provider, upstream) -> Option<&ProxyEntry>` — exact-match lookup. No URL normalization.
- Public on the module.

**Tests** (write FIRST, watch them fail, then implement):
1. round-trip: build a state, save, load, assert equal.
2. missing file → `Ok(None)`.
3. corrupt JSON → `Err` mentioning the path.
4. `find_proxy` returns `Some` for exact match, `None` otherwise.
5. `find_proxy` for wrong provider returns `None` even if upstream matches.
6. On Unix: saved file has mode `0o600` (use `std::os::unix::fs::PermissionsExt`).

**Files touched**: `src/daemon/state.rs`.

**Commit**: `feat(daemon): DaemonState type + atomic load/save (0600 perms)`

---

### Task 4 — Pidfile acquire / release / liveness check

**Goal**: Implement the pidfile primitives from Spec §8.

**API**:

```rust
pub struct Pidfile { path: PathBuf }

impl Pidfile {
    pub fn new(path: PathBuf) -> Self;

    /// Atomically create the pidfile and write our PID into it.
    /// Returns Err if the file already exists (caller should preflight first).
    pub fn acquire(&self) -> Result<()>;

    /// Best-effort removal. Idempotent.
    pub fn release(&self) -> Result<()>;

    /// Returns the PID if the file exists and parses, else Ok(None).
    pub fn read(&self) -> Result<Option<u32>>;
}

/// Returns true if a process with this PID is running and visible to us.
/// On Unix: kill(pid, 0). EPERM also counts as alive (we don't own it).
/// On non-Unix: returns Ok(false) (the caller never reaches here, but keep
///   the function compilable on Windows).
pub fn process_alive(pid: u32) -> Result<bool>;

/// Returns the comm name for the PID, or None if it cannot be determined.
/// Linux: /proc/<pid>/comm. macOS: `ps -p <pid> -o comm=`. Used by the
/// preflight check to distinguish "our prior daemon" from "PID reused by
/// some other process".
pub fn process_name(pid: u32) -> Option<String>;
```

**Tests** (TDD):
1. `acquire` writes the file with our PID.
2. `acquire` errors when file already exists.
3. `release` removes the file. `release` again is `Ok(())`.
4. `read` returns `None` for missing file, `Some(pid)` for valid, `Err` for unparseable content.
5. `process_alive(std::process::id())` returns true.
6. `process_alive(1)` returns true on Unix (init exists).
7. `process_alive(0xFFFF_FFFE)` returns false (no PID this high).
8. File permissions on the pidfile after acquire are `0o600`.

**Files touched**: `src/daemon/pidfile.rs`.

**Commit**: `feat(daemon): pidfile + process liveness primitives`

---

### Task 5 — Preflight: pidfile state matrix from Spec §8

**Goal**: Implement the decision table from Spec §8 as a single function the start command will call before doing anything else.

**API**:

```rust
pub enum PidfilePreflight {
    Proceed,                                      // no pidfile, or stale
    DaemonAlreadyRunning { pid: u32 },            // alive, our process name
    PortOccupied      { pid: u32, name: String }, // alive, different name
}

pub fn preflight_pidfile(path: &Path) -> Result<PidfilePreflight>;
```

Behavior matches the §8 table:
- Missing → `Proceed`.
- Present + alive + name == `"cc-switch"` (or process_name returns None due to permission error) → `DaemonAlreadyRunning` UNLESS the name lookup explicitly returned a different name. (Be lenient on unknown — treat as ours.)
- Present + alive + name is something else (`bash`, `vim`, etc.) → `PortOccupied`.
- Present + dead → log warn, remove file, `Proceed`.
- Present + unparseable → log warn, remove file, `Proceed`.

**Tests** (TDD, in `tests/daemon_preflight.rs`):
1. missing pidfile → Proceed.
2. pidfile with our own PID → DaemonAlreadyRunning.
3. pidfile pointing at PID 1 with name `init`/`launchd` → PortOccupied (depends on host; use a mocking layer or skip on platforms where PID 1 is `cc-switch`, which is none).
4. pidfile with bogus PID (e.g., `99999999`) → file removed, Proceed.
5. pidfile with unparseable content (`"hello"`) → file removed, Proceed.

**Files touched**: `src/daemon/pidfile.rs` (add this above the helpers from Task 4).

**Commit**: `feat(daemon): pidfile preflight decision table`

---

### Task 6 — Double-fork (Unix only) + foreground path

**Goal**: Implement the `fork.rs` daemonization. This is the riskiest task — write very small, test by spike-running.

**API**:

```rust
/// Fork twice, setsid, chdir("/"), close stdio, redirect to `log_path`.
/// Returns Ok(()) in the grandchild that should run the daemon main.
/// Parent / first child call _exit(0) and never return.
///
/// On non-Unix this is a compile error to call. The commands.rs caller
/// is also #[cfg(unix)]; this function should be #[cfg(unix)] too.
#[cfg(unix)]
pub fn double_fork_into_background(log_path: &Path) -> Result<()>;
```

**Implementation outline** (write small, test obsessively):

1. Open `log_path` (`O_WRONLY | O_CREAT | O_APPEND`, mode 0o600). Hold the raw fd.
2. `unsafe { libc::fork() }`. Parent: `libc::_exit(0)`. Child: continue.
3. `libc::setsid()`. On error: bail.
4. `unsafe { libc::fork() }` again. Parent: `_exit(0)`. Grandchild: continue.
5. `libc::chdir(c"/")`.
6. `libc::dup2(log_fd, 1)` (stdout). `libc::dup2(log_fd, 2)` (stderr).
7. Open `/dev/null` and `dup2` to fd 0 (stdin).
8. Close the original log_fd if it isn't already 1 or 2.
9. Return Ok(()).

**Tests** (very minimal — testing fork in unit tests is fragile):

- `tests/daemon_fork.rs`, gated `#[cfg(unix)]`:
  - Spawn a subprocess that calls a tiny helper bin (or uses `cargo run --example`) which calls `double_fork_into_background` then writes a marker to a tempfile and `_exit(0)`. The parent test verifies the marker appears within 5s and the original subprocess returned 0.
- If implementing the helper-bin is too heavy, ship one TODO comment and rely on the e2e smoke test for fork correctness. **Note this in the report.**

**Files touched**: `src/daemon/fork.rs`, possibly `examples/daemon_fork_smoke.rs`.

**Commit**: `feat(daemon): double-fork into background with log redirection`

---

### Task 7 — Daemon lifecycle main (no real proxies yet)

**Goal**: Wire the tokio runtime + signal handling + state file lifecycle, but **without** actually spawning ccs-proxy yet. Spawn placeholder "would-spawn" log lines for each unique upstream. This isolates fork+signal+state correctness from proxy correctness.

**API**:

```rust
pub struct LifecycleConfig {
    pub state_path: PathBuf,
    pub pidfile_path: PathBuf,
    pub data_root: PathBuf,
    pub upstreams: Vec<(String /* provider */, String /* upstream URL */)>,
}

/// Owns the tokio runtime. Blocks until shutdown signal. Cleans up pidfile
/// + writes final state (stopped_at set) on shutdown.
pub fn run_daemon_blocking(cfg: LifecycleConfig) -> Result<()>;
```

**Behavior**:

1. Acquire pidfile.
2. Build tokio current-thread runtime (`Runtime::new()`).
3. Inside the runtime:
   - For each upstream: log `would spawn proxy for {provider} {url}` (no actual serve yet).
   - Build a `DaemonState` with `proxies = []` for now (Task 8 will wire the real list).
   - Save state file.
   - `tokio::select!` between `signal::ctrl_c()` and `signal::unix::signal(SIGTERM)`.
4. On signal: log "shutting down", update `stopped_at`, save state file, release pidfile, return Ok.

**Tests** (`tests/daemon_lifecycle.rs`):
1. Run `run_daemon_blocking` in a thread; after 200ms, send SIGTERM to ourselves; assert it returns within 2s. Assert pidfile is gone. Assert state file's `stopped_at` is set.
2. Run twice in sequence — first acquires, releases; second acquires successfully.

**Files touched**: `src/daemon/lifecycle.rs`.

**Commit**: `feat(daemon): lifecycle main with signal handling + state file IO (no proxies yet)`

---

### Task 8 — Spawn real ccs-proxy handles + supervisor

**Goal**: Replace the "would spawn" stub from Task 7 with real `ccs_proxy::serve()` calls and a 30s supervisor loop. Update state file after every lifecycle event.

**Implementation**:

1. Drop the `tokio::runtime::Runtime::new()` (current-thread) approach if needed and switch to `Runtime::new_multi_thread()` so axum handlers from ccs-proxy can run alongside the supervisor.
2. For each `(provider, upstream)` in `cfg.upstreams`:
   - Build `ccs_proxy::ServeConfig { provider: Claude, upstream: parsed, proxy_port: 0, api_port: 0, data_dir: cfg.data_root.join(sha8(&upstream)), redact: true }`.
   - `ccs_proxy::serve(scfg).await` → store the returned ProxyHandle in a `Vec<ProxyHandle>`.
   - Build a `ProxyEntry` from the handle's ports + the data_dir.
3. Save initial state file with the populated `proxies`.
4. Spawn supervisor task: every 30s, check each handle. If a handle reports its task as finished/panicked, call `serve()` again with the same config, bump `restart_count`, atomically rewrite state file.
5. On shutdown: drop each ProxyHandle (graceful shutdown is part of ccs-proxy's API), set `stopped_at`, save, release pidfile.

**Dedupe helper**: `fn dedupe_upstreams(configs: &Configurations) -> Vec<(String, String)>` — iterates claude aliases, collects `(provider, alias.url)` into a `BTreeSet` to preserve order + dedupe. Lives in `src/daemon/lifecycle.rs`.

**Tests** (`tests/daemon_supervisor.rs`):
1. Spawn the daemon with two distinct upstreams pointed at a wiremock; assert state file lists both with non-zero ports.
2. Spawn with three claude aliases sharing two upstream URLs; assert dedupe → only two `ProxyEntry` items.
3. Manually drop one ProxyHandle from outside (or kill the task via the handle's underlying join handle); poll for up to 35s; assert `restart_count` went 0 → 1 in the state file. If 35s is too slow for CI, expose a `supervisor_interval` knob on `LifecycleConfig` (default 30s) and use 200ms in tests.

**Files touched**: `src/daemon/lifecycle.rs`, `tests/daemon_supervisor.rs`.

**Commit**: `feat(daemon): spawn ccs-proxy handles + 30s supervisor with restart_count`

---

### Task 9 — `cc daemon start | stop | restart` command handlers

**Goal**: Wire the three lifecycle commands through `commands.rs`. `start` integrates fork + preflight + lifecycle. `stop` integrates pidfile read + SIGTERM + poll. `restart` is `stop` then `start`.

**API**:

```rust
pub enum DaemonAction { Start { foreground: bool }, Stop, Status { json: bool }, Restart { foreground: bool } }

pub fn handle_daemon_command(action: DaemonAction, storage: &ConfigStorage) -> Result<()>;
```

**Per-action behavior** (matches Spec §7):

- `Start`:
  1. Preflight pidfile. If `DaemonAlreadyRunning` → exit 1 with "daemon already running (pid N)". If `PortOccupied` → exit 1 with "pid N is occupied by another process (name: X) — investigate".
  2. Compute `data_root = config_root.join("daemon-data")`. Mkdir 0o700.
  3. Build LifecycleConfig from storage.
  4. If `!foreground`: `double_fork_into_background(log_path)` — grandchild proceeds.
  5. Call `run_daemon_blocking(cfg)`.

- `Stop`:
  1. Read pidfile. Missing → print "cc-switch daemon: not running" + Ok.
  2. `kill(pid, SIGTERM)`. ESRCH → print "stale pidfile, removed" + remove pidfile + Ok.
  3. Poll `process_alive(pid)` every 100ms for up to 5s.
  4. Still alive → `kill(pid, SIGKILL)` + warn to stderr + remove pidfile + Ok (exit 0).
  5. Else → print "cc-switch daemon: stopped" + Ok.

- `Restart { foreground }`:
  1. Run Stop (ignore "not running" outcome).
  2. Run Start { foreground }.

**Tests** — manual smoke for start because of forking. Stop has unit tests:

- `tests/daemon_commands.rs`:
  - Stop with no pidfile → Ok + correct message.
  - Stop with pidfile pointing at dead pid → removes file + Ok.
  - (Start covered by lifecycle tests via `--foreground` path.)

**Files touched**: `src/daemon/commands.rs`, `tests/daemon_commands.rs`.

**Commit**: `feat(daemon): start | stop | restart command handlers`

---

### Task 10 — `cc daemon status` (with `--json`)

**Goal**: Render the table from Spec §3. Use `ureq` for the per-proxy health probe so the status command doesn't need a tokio runtime.

**API** in `src/daemon/status.rs`:

```rust
pub struct ProxyStatus {
    pub entry: ProxyEntry,
    pub reachable: bool,
    pub request_count: Option<u64>,
    pub store_degraded: bool,
}

pub fn collect_status(state: &DaemonState) -> Vec<ProxyStatus>;
pub fn format_status_text(state: &DaemonState, statuses: &[ProxyStatus], aliases_per_upstream: &BTreeMap<String, Vec<String>>) -> String;
pub fn format_status_json(state: &DaemonState, statuses: &[ProxyStatus]) -> serde_json::Value;
```

**Behavior**:

1. Read state file. Missing or daemon not alive (per pidfile) → print `cc-switch daemon: STOPPED` (text) or `{"status":"stopped"}` (json), exit 0.
2. For each ProxyEntry: `ureq::get(http://127.0.0.1:{api_port}/api/health).timeout_connect(500ms).timeout(500ms).call()`. Parse JSON. Failure → `reachable=false`.
3. Build alias-per-upstream map from storage.
4. Format.

**Tests** (`tests/daemon_status.rs`):
1. `format_status_text` snapshot for a state with two proxies.
2. `format_status_text` for STOPPED state.
3. `format_status_json` schema check (top-level keys present).
4. Don't test the live `/api/health` probe — covered by integration in Task 11.

**Files touched**: `src/daemon/status.rs`, `tests/daemon_status.rs`.

**Commit**: `feat(daemon): status command with ureq health probes + --json`

---

### Task 11 — Wire `cc-switch daemon` into the clap CLI

**Goal**: Add the `Daemon` subcommand variant in `src/cli/cli.rs` and dispatch in `src/cli/main.rs`. Update completion + help.

**Spec for clap**:

```rust
/// Manage the cc-switch capture daemon
#[command(subcommand)]
Daemon {
    #[command(subcommand)]
    command: DaemonCommands,
},

#[derive(Subcommand)]
pub enum DaemonCommands {
    /// Start the daemon (double-forks into background by default)
    Start {
        /// Run in the foreground instead of forking
        #[arg(long, short = 'F')]
        foreground: bool,
    },
    /// Stop the daemon gracefully (SIGTERM, then SIGKILL after 5s)
    Stop,
    /// Restart the daemon
    Restart {
        #[arg(long, short = 'F')]
        foreground: bool,
    },
    /// Show daemon status and configured proxies
    Status {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}
```

In `src/cli/main.rs`:
- Add `Commands::Daemon { command }` match arm calling `daemon::handle_daemon_command(action, &storage)`.
- Update help long_about EXAMPLES section with `cc-switch daemon start` example.
- Update `src/cli/completion.rs` — `cc-switch daemon` should complete `start | stop | restart | status`.

**Tests** (`tests/daemon_cli.rs`):
1. `cargo run -- daemon --help` exits 0 (verified via `assert_cmd` if available, else `Command::new(env!("CARGO_BIN_EXE_cc-switch"))`).
2. `cargo run -- daemon stop` with no pidfile → exits 0, stdout contains "not running".
3. `cargo run -- daemon status` with no state file → exits 0, stdout contains "STOPPED".

**Files touched**: `src/cli/cli.rs`, `src/cli/main.rs`, `src/cli/completion.rs`, `tests/daemon_cli.rs`.

**Commit**: `feat(cli): wire `cc-switch daemon` subcommand`

---

### Task 12 — `cc use` consults the daemon state file

**Goal**: The actual auto-wrap. Modify the `Commands::Use` handler in `src/cli/main.rs` (and the interactive selection path) to check the daemon state file and substitute the URL.

**Implementation**:

1. Add a helper in `src/daemon/state.rs`:

   ```rust
   /// Returns Some(proxy_url) if the daemon is alive and has a proxy for
   /// (provider, upstream). Returns None otherwise.
   pub fn resolve_proxy_url(
       state_path: &Path,
       pidfile_path: &Path,
       provider: &str,
       upstream: &str,
   ) -> Option<String>;
   ```

   It returns `None` (not Err) for every failure mode — missing state file, dead daemon, corrupt JSON, upstream not registered. The caller treats `None` as "fall back to direct".

2. In `Commands::Use` (`src/cli/main.rs` line ~852):
   - After cloning the Configuration but BEFORE building `EnvironmentConfig`:
     ```rust
     let effective_url = match daemon::state::resolve_proxy_url(
         &state_path, &pidfile_path, "claude", &config.url,
     ) {
         Some(proxy_url) => proxy_url,
         None => {
             if daemon_pidfile_exists_but_dead(&pidfile_path) || true {
                 // print the one-line hint exactly once (use OnceLock)
                 eprintln!("ℹ cc daemon is not running — traffic for '{alias_name}' will NOT be captured.");
                 eprintln!("  Run `cc daemon start` and re-run to enable capture.");
             }
             config.url.clone()
         }
     };
     ```
   - Build `EnvironmentConfig` from a Configuration clone with `url` overridden to `effective_url`. (Don't mutate the storage copy.)
   - Then call `switch_to_config_with_mode` etc. exactly as today.

3. Hint policy:
   - Print only when `state_path` is missing OR pidfile is missing OR pidfile points to dead PID.
   - **Do NOT** print when the daemon is up but this specific upstream isn't registered (user has the daemon, just hasn't added this alias yet — different ergonomic case). For v1 simplicity, log this case to stderr only at `--verbose` or never; we'll choose "never" for v1 to keep noise down.
   - Use a `OnceLock<()>` to ensure at most one hint per process.

4. Apply the same substitution path to the interactive selection (`handle_interactive_selection`). It calls into similar code — refactor common URL-resolution into a helper `fn resolve_effective_url(alias_name, config) -> (String /* url */, bool /* via_daemon */)` to avoid duplicating the lookup logic across the two call sites.

**Tests** (`tests/daemon_use_integration.rs`):
1. With no daemon: `cc-switch use someAlias` writes settings.json with `config.url`. Stderr contains the hint.
2. With a mock state file pointing at a fake proxy port + a live PID (use `std::process::id()` since we're the live process): writes settings.json with `http://127.0.0.1:<port>`. No hint on stderr.
3. With state file present but pidfile pointing at a dead PID: behaves as case 1.
4. Special alias `cc` / `official` bypass the daemon entirely.

**Files touched**: `src/daemon/state.rs`, `src/cli/main.rs`, `src/interactive/interactive.rs`, `tests/daemon_use_integration.rs`.

**Commit**: `feat(cli): cc use consults daemon state file + emits hint on fallback`

---

## After all 12 tasks: final review

After Task 12 commits:

1. **Dispatch a final code-reviewer subagent** with the whole diff range (`BASE_SHA = a0a2747`, the pre-Spec-B commit; `HEAD_SHA = current`). Use the standard requesting-code-review template. Ask for:
   - Spec compliance against `2026-05-28-cc-switch-daemon-design.md`.
   - Coupling / leakage between `daemon/`, `cli/`, and `ccs-proxy`.
   - Concurrency / signal-safety issues in `lifecycle.rs` and `fork.rs`.
   - Any test that mocks rather than exercises real behavior (per project memory: integration tests should hit real components).

2. **Run the full smoke checklist manually** in the controller turn:
   - `cargo build --release`
   - `cc-switch daemon start` (background) → returns to shell immediately
   - `cc-switch daemon status` → shows RUNNING + proxies
   - `cc-switch use <some-claude-alias>` → settings.json has proxy URL; claude launches; capture appears in `http://127.0.0.1:<api_port>/`
   - `cc-switch daemon stop` → status returns STOPPED
   - `cc-switch use <some-claude-alias>` → settings.json has direct URL; stderr has the hint

3. **Update `MEMORY.md`** if anything load-bearing was learned that wasn't in the spec (per memory rules: only non-obvious things; "we use double-fork" is in the spec already, no memory needed).

4. **Hand the controller back to `superpowers:finishing-a-development-branch`** to drive merge / PR decision.

## Risk register

| Risk | Mitigation |
|---|---|
| Double-fork misbehaves on macOS CI | Task 6 ships a helper bin + integration test; if it's fragile, demote to e2e-only and document the limitation in PR. |
| `tokio::JoinHandle::is_finished()` doesn't catch axum panics cleanly | Task 8 supervisor test pins this down; alternative is an explicit `tokio::spawn` + watch channel from inside ccs-proxy (out of scope for Spec B — would require Spec A API change). |
| `ureq` adds a noticeable binary size | We're already pulling reqwest via ccs-proxy; ureq adds ~150KB of TLS-less HTTP. Acceptable. |
| `cc use` slowdown due to state file IO + pidfile check | The whole check is two `fs::read` + a `kill(pid, 0)`. Sub-millisecond. No mitigation needed. |
| Stale state file after crash leads `cc use` to a bogus port | Pidfile liveness check catches it; if pidfile somehow survives but daemon doesn't, the proxy port is closed and reqwest from claude will fail loudly. User runs `cc daemon restart`. |
| Adding deps to cc-switch bloats release binary | Strip + LTO already on; accept the growth (~5MB). Document in the PR. |

## Out of scope for this branch (do NOT do during Spec B)

- Anything codex-related (separate spec).
- Auto-respawning the daemon from `cc use`.
- Lazy proxy spawning.
- URL normalization for the lookup key.
- Per-alias `--no-proxy` flag.
- SIGHUP reload.
- Windows daemon support.
- Pre-publishing ccs-proxy to crates.io (path dep is fine for the v1 release; that's a follow-up).
