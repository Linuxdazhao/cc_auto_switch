# Spec B — `cc daemon` + `cc use` auto-wrap through `ccs-proxy`

**Date**: 2026-05-28
**Status**: Approved design, ready for implementation plan
**Depends on**: Spec A (`ccs-proxy` crate — already implemented on this branch)
**Scope**: claude side only. Codex daemon support deferred (codex aliases have
no upstream URL field today).

## 1. Context & motivation

Spec A delivered the `ccs-proxy` crate: a logging reverse proxy + dashboard,
usable as a binary or as a library. Spec B is the cc-switch integration that
makes the proxy invisible day-to-day:

- User runs `cc daemon start` once.
- From then on, every `cc use <alias>` writes the proxy URL into
  `~/.claude/settings.json` instead of the alias's raw upstream URL.
- Claude Code launches and all traffic is captured in the dashboard with
  zero ceremony per call.
- If the daemon is off, `cc use` silently falls back to direct mode (with a
  single hint line to stderr) so the existing workflow is never broken.

Without Spec B, users have to either run `ccs-proxy serve` themselves and
register a separate cc-switch alias whose URL points at the proxy, or rewrite
`ANTHROPIC_BASE_URL` by hand. Both kill the appeal.

## 2. Scope & non-goals

### v1 in-scope

- New subcommands: `cc daemon start | stop | status | restart`.
- Daemon process supervises one in-process `ccs_proxy::serve()` per unique
  upstream URL from `~/.cc-switch/configurations.json` (claude aliases only).
- State file `~/.cc-switch/daemon-state.json` mapping upstream → proxy port.
- PID file `~/.cc-switch/daemon.pid` for liveness detection.
- `cc use <alias>` consults the state file and rewrites the URL it writes to
  `~/.claude/settings.json` when a matching proxy entry is found.
- Default `start` double-forks into background (Unix). `--foreground` for
  debugging.
- Anti-entropy on `start`: if pidfile points to a dead process, clean up and
  proceed; if alive, error with a helpful message.
- `cc daemon status` reports running/dead, uptime, request counts (via
  `/api/health`), and each proxy's ports + dashboard URL.
- Unix only (macOS + Linux). Windows reports unsupported, same posture as
  ccs-proxy.

### v1 explicitly out-of-scope

| Feature | Rationale |
|---|---|
| Codex daemon support | Codex aliases have no `base_url`; needs a separate cc-switch schema change first |
| Auto-respawn from `cc use` when daemon is dead | Hidden process spawning is hard to reason about; v1 user runs `cc daemon start` themselves |
| Lazy proxy spawn (per-alias on demand) | Eager is simpler; users restart daemon after adding aliases |
| Daemon hot-reload of configurations.json | Use `cc daemon restart` instead |
| Per-alias opt-out of proxying | All claude aliases with a URL get proxied; opt-out is v2 |
| `cc daemon logs` / `cc daemon tail` | Use the per-proxy log files directly (path in `status` output) |
| Windows support | Forking model is Unix; Windows needs a service wrapper — v2 |
| Aggregating multiple daemons / multi-user setup | v1 = one daemon per `$HOME` |

## 3. User-facing surface

### New CLI

```
cc-switch daemon start    [--foreground]
cc-switch daemon stop
cc-switch daemon status
cc-switch daemon restart
```

`--foreground` runs in the current TTY (logs to stdout, Ctrl-C to stop). Used
for debugging and as the inner half of the double-fork.

### Changed behavior

`cc use <alias>` and the interactive selection menu (`cc-switch` with no
args) gain one new step before writing `~/.claude/settings.json`:

```
1. Resolve the alias → Configuration (as today).
2. Read ~/.cc-switch/daemon-state.json (if exists + daemon alive).
3. If a proxy entry matches (provider="claude", upstream=alias.url):
     URL_TO_WRITE = http://127.0.0.1:<proxy_port>
   Else:
     URL_TO_WRITE = alias.url        (current behavior; emit one-line hint)
4. Write settings.json with URL_TO_WRITE.
5. Launch claude (as today).
```

The hint when falling back to direct:

```
ℹ cc daemon is not running — traffic for 'work' will NOT be captured.
  Run `cc daemon start` and re-run to enable capture.
```

Printed at most once per process invocation, only when the alias would have
been proxied if the daemon were up.

### Backward compatibility

- Daemon absent → behavior identical to v0.1.x (modulo the one-line hint).
- `cc use` with `--store env` / `--store config` continues to work; the only
  change is the URL value, not the field structure.
- Special aliases `cc` / `official` (reset to vanilla Claude) bypass the
  daemon entirely.

### Status output (machine-readable)

```
$ cc-switch daemon status
ccs-daemon: RUNNING (pid 38421, uptime 1h 22m)
  state: ~/.cc-switch/daemon-state.json
  pidfile: ~/.cc-switch/daemon.pid

Proxies (2):
  ┌────────────────────────────────────┬──────────────┬──────────────┬─────────┐
  │ upstream                           │ proxy_port   │ dashboard    │ requests│
  ├────────────────────────────────────┼──────────────┼──────────────┼─────────┤
  │ https://api.anthropic.com          │ 41001        │ :41501       │     142 │
  │ https://glm.example.com/v1         │ 41002        │ :41502       │      17 │
  └────────────────────────────────────┴──────────────┴──────────────┴─────────┘

Aliases routed through daemon:
  work, personal      → https://api.anthropic.com
  glm                 → https://glm.example.com/v1

cc-switch daemon status --json   for machine-readable output.
```

`--json` flag returns the same data as JSON.

## 4. Architecture

### Process model

```
┌────────────────────────────┐
│ cc-switch daemon start     │  user shell (foreground PID)
└────────────┬───────────────┘
             │ fork + setsid + fork (Unix double-fork)
             ▼
┌────────────────────────────┐
│ daemon process (detached)  │  pid → ~/.cc-switch/daemon.pid
│                            │
│ tokio runtime              │
│  ├── ccs_proxy::serve(A)   │  proxy_port=41001  api_port=41501
│  ├── ccs_proxy::serve(B)   │  proxy_port=41002  api_port=41502
│  └── supervisor task       │  polls health, restarts dead handles
│                            │
│ writes daemon-state.json   │  after every proxy state change
└────────────────────────────┘
```

The daemon hosts every proxy in-process as a tokio task. There are **no
child `ccs-proxy` subprocesses** — `ccs_proxy` is consumed as a library
crate (`ccs-proxy = { path = "ccs-proxy" }`).

### File layout in cc-switch crate

```
cc-switch/
├── Cargo.toml                       # adds ccs-proxy path dep + libc upgrade
├── ccs-proxy/                       # (already exists from Spec A)
├── src/
│   ├── daemon/
│   │   ├── mod.rs                   # public surface for the cli layer
│   │   ├── commands.rs              # start | stop | status | restart handlers
│   │   ├── lifecycle.rs             # daemon main loop + supervisor
│   │   ├── state.rs                 # daemon-state.json (atomic write + read)
│   │   ├── pidfile.rs               # pidfile + process liveness check
│   │   ├── fork.rs                  # double-fork (Unix)
│   │   └── status.rs                # status command formatter
│   ├── cli/
│   │   ├── cli.rs                   # add `Daemon` subcommand
│   │   └── main.rs                  # wire `Use` handler to consult state
│   └── ...
```

`fork.rs` is `#[cfg(unix)]`. The `daemon` subcommand entry point returns an
error on Windows with a clear pointer to "use `ccs-proxy serve` directly".

### Daemon ↔ cc-switch contract

The state file is the only ABI:

- `cc use` reads it to decide whether to substitute the URL.
- `cc daemon status` reads it to render the table.
- The daemon writes it after every supervised lifecycle event (proxy
  started, restarted, port re-bound, daemon stopping).

`cc use` does NOT speak to the daemon over IPC. The state file is the
contract; staleness is bounded by atomic writes and a liveness check
(pidfile + `kill -0`).

## 5. Data flow

### Daemon startup

```
cc daemon start
  │
  ├── parse args
  ├── if !--foreground:  fork-setsid-fork  →  exit parent (returns to shell)
  │
  ├── acquire ~/.cc-switch/daemon.pid (O_EXCL); if held by alive PID → error
  │
  ├── load ~/.cc-switch/configurations.json
  ├── dedupe by (provider=claude, upstream=alias.url)
  ├── for each unique upstream:
  │     handle = ccs_proxy::serve(ServeConfig {
  │       provider: ProviderKind::Claude,
  │       upstream: <url>,
  │       proxy_port: 0,  api_port: 0,    // OS picks
  │       data_dir: ~/.cc-switch/daemon-data/<sha8(upstream)>/,
  │       redact: true,
  │     }).await?
  │
  ├── write daemon-state.json atomically
  ├── install SIGTERM/SIGINT handler → graceful shutdown
  └── supervisor loop:
        every 30s for each handle:
          if dead: re-spawn, update state file
```

### `cc use` request

```
cc use work
  │
  ├── load Configuration("work") from configurations.json   (as today)
  │
  ├── try_daemon_state():
  │     if ~/.cc-switch/daemon-state.json missing → None
  │     read JSON
  │     read ~/.cc-switch/daemon.pid; kill -0 PID → if not alive → None
  │     return Some(state)
  │
  ├── url_to_write = match state {
  │     Some(s) if s.find(provider, upstream) is Some(p) => proxy_url(p),
  │     _ => { print_hint_if_first_alias_call(); config.url.clone() }
  │   }
  │
  ├── write ~/.claude/settings.json with url_to_write    (as today)
  └── exec claude                                         (as today)
```

### Daemon shutdown

```
SIGTERM/SIGINT received
  │
  ├── stop supervisor loop
  ├── for each ProxyHandle: drop (triggers graceful axum shutdown)
  ├── write final daemon-state.json with stopped_at = now
  ├── remove ~/.cc-switch/daemon.pid
  └── exit 0
```

`stop` from another shell sends SIGTERM to PID in pidfile.

## 6. State file format

`~/.cc-switch/daemon-state.json`:

```json
{
  "schema_version": 1,
  "pid": 38421,
  "started_at": "2026-05-28T19:30:12.345Z",
  "stopped_at": null,
  "data_root": "/Users/jingzhao/.cc-switch/daemon-data",
  "proxies": [
    {
      "provider": "claude",
      "upstream": "https://api.anthropic.com",
      "proxy_port": 41001,
      "api_port": 41501,
      "data_dir": "/Users/jingzhao/.cc-switch/daemon-data/8f3a2c1e",
      "started_at": "2026-05-28T19:30:12.401Z",
      "restart_count": 0
    },
    {
      "provider": "claude",
      "upstream": "https://glm.example.com/v1",
      "proxy_port": 41002,
      "api_port": 41502,
      "data_dir": "/Users/jingzhao/.cc-switch/daemon-data/c4e7b9d2",
      "started_at": "2026-05-28T19:30:12.512Z",
      "restart_count": 0
    }
  ]
}
```

**Lookup key for `cc use`**: exact string match on `provider` + `upstream`.
URL normalization (trailing slash, default port) is deferred to v2 — users
get the proxy if and only if `alias.url == proxies[].upstream` byte-for-byte.

`stopped_at` is set on graceful shutdown only. Crashed daemons leave it
null; the next `start` overwrites the file entirely after pidfile cleanup.

Permissions: `0600` on the file, `0700` on `~/.cc-switch/` (already enforced
by config_storage.rs).

### Pid file

`~/.cc-switch/daemon.pid` contains the daemon's PID as a single ASCII
integer + trailing newline. Atomically created via `O_CREAT | O_EXCL`.

## 7. Process lifecycle

### `cc daemon start`

1. `#[cfg(not(unix))]` → return `Err("cc daemon is Unix-only in v1; on Windows run `ccs-proxy serve` directly")`.
2. Parse args. Default = background; `--foreground` skips the fork.
3. Pidfile preflight (see §8).
4. If background: `unsafe { libc::fork() }`:
   - Parent: `_exit(0)` (returns control to shell).
   - Child: `libc::setsid()`, second `fork()`, parent again `_exit(0)`.
   - Grandchild: `chdir("/")`, close stdin/stdout/stderr, redirect to
     `~/.cc-switch/daemon.log` (append mode).
5. Acquire pidfile (write our PID, fail if it already exists).
6. Build tokio runtime, run daemon main:
   - load configurations.json, dedupe upstreams, spawn proxies
   - write initial state file
   - install signal handlers
   - run supervisor loop
7. On shutdown: tear down, remove pidfile, exit 0.

### `cc daemon stop`

1. Read pidfile → PID. If missing → "daemon not running" + exit 0.
2. `kill(PID, SIGTERM)`. If `ESRCH` → "daemon not running (stale pidfile)" +
   remove pidfile + exit 0.
3. Poll `kill(PID, 0)` every 100ms for up to 5s.
4. If still alive after 5s → `kill(PID, SIGKILL)` + warn + remove pidfile.
5. Else → "daemon stopped" + exit 0 (the daemon removes its own pidfile on
   clean shutdown; we delete it on SIGKILL only).

### `cc daemon status`

1. Read pidfile → PID. If missing → print "STOPPED" + exit 0.
2. `kill(PID, 0)`: if not alive → print "STOPPED (stale pidfile)" + exit 0.
3. Read state file. Render the table (see §3).
4. For each proxy, GET `http://127.0.0.1:<api_port>/api/health` with 500ms
   timeout. Augment row with request count + store-degraded flag. Unreachable
   → render row with `(unreachable)`.

### `cc daemon restart`

```
cc daemon restart
  = cc daemon stop && cc daemon start
```

Implemented as that exact sequence so any state-file inconsistency from a
crash gets cleaned up by stop's path.

## 8. Anti-entropy / liveness detection

### Pidfile invariants

| State | `cc daemon start` reaction |
|---|---|
| pidfile missing | proceed |
| pidfile present, PID alive, our process name | error: "daemon already running (PID N)" |
| pidfile present, PID alive, different process | error: "PID N occupied by other process — investigate" |
| pidfile present, PID dead | warn + remove pidfile + proceed |
| pidfile present, PID < 1 or unparseable | warn + remove pidfile + proceed |

Process-name check uses `/proc/<pid>/comm` on Linux and `ps -p <pid> -o
comm=` on macOS via `Command::new`. Failure to read process name (e.g.,
permission denied) is treated as **stale + proceed** with a warn log
(matches the practical case of cc-switch reinstalls + PID reuse).

### Port collisions

`proxy_port: 0` / `api_port: 0` lets the OS choose. After a crash and
restart, ports may differ — state file is overwritten, so `cc use` always
gets the current ports. No port-conflict cleanup needed.

### Supervisor loop

Every 30s:
- For each ProxyHandle: check the join handle's `is_finished()`.
- If finished (panic or unexpected exit):
  - Log error.
  - Re-call `ccs_proxy::serve()` with the same upstream.
  - Bump `restart_count` in state file.
  - Atomically rewrite state file.

`restart_count` is informational only. A circuit breaker on restart loops
is v2.

## 9. Error handling

| Class | Behavior |
|---|---|
| pidfile preflight fails | exit 1 with message; do not touch state file |
| ccs_proxy::serve() returns Err during start | log error, skip that upstream, continue with the others; state file entry omitted |
| configurations.json missing/empty | start with zero proxies + warn; daemon runs idle (so `cc daemon start && cc add ... && cc daemon restart` works) |
| State file unwritable | abort startup (we have no usable contract surface) |
| State file unreadable in `cc use` | treat as "daemon down"; silent fallback |
| State file stale (daemon dead but file present) | pidfile liveness check catches it; `cc use` falls back |
| Supervisor restart of dead proxy fails | log error; remove the proxy from state file; daemon continues |
| SIGTERM during a proxy spawn | spawn completes (sub-second); then shutdown |
| Daemon log file unwritable | warn to stderr (before redirection) and proceed with `/dev/null` |
| Windows | start/stop/status/restart all return an error pointing at `ccs-proxy serve` |

## 10. Security

- `~/.cc-switch/daemon.pid`, `daemon-state.json`, `daemon.log` → `0600`.
- `~/.cc-switch/daemon-data/<hash>/` → inherits ccs-proxy's own `0700`/`0600`.
- Proxies bind `127.0.0.1` only (enforced by ccs-proxy).
- Daemon never logs API tokens. State file contains URLs only.
- Process-name probing reads only `/proc/<pid>/comm` (read-only).

## 11. Testing strategy

| Layer | Coverage | Tooling |
|---|---|---|
| Unit | pidfile parsing, state file (de)serialization, dedupe logic, status formatter | `cargo test --lib` |
| Integration | start → state file populated → stop → state cleared; `cc use` substitutes URL when daemon up; falls back when down; falls back with hint when alias upstream not in state | `tests/daemon_integration.rs` (no real fork; run daemon main in a tokio task) |
| Liveness | pidfile alive vs stale vs corrupted, all branches in §8 | `tests/daemon_liveness.rs` |
| End-to-end (manual, documented) | actual double-fork + dashboard reachable + `cc use` writes correct URL | smoke checklist in PR description |
| Supervisor | kill a ProxyHandle, verify supervisor respawns it within 30s + state file updated | `tests/daemon_supervisor.rs` |

Tests run on Unix only via `#[cfg(unix)]` (matches the daemon's own
gating). Windows CI skips the daemon test module entirely.

Coverage target: ≥ 75% on `src/daemon/`.

## 12. Observability

- Daemon log: `~/.cc-switch/daemon.log` (stdout/stderr redirect from
  double-fork; tracing default `info`, `RUST_LOG=cc_switch=debug` for
  debugging).
- Per-proxy logs go to the proxy's own `<data_dir>/logs/ccs-proxy.log` as
  established by Spec A.
- `cc daemon status` is the single user-facing diagnostic.

## 13. Build & distribution

- `cc-switch` gains `ccs-proxy = { path = "ccs-proxy" }`. Both crates live
  in this branch (and ship together for v1). When ccs-proxy gets published
  to crates.io later, swap the path dep for a version dep in the same PR
  that releases that ccs-proxy version.
- libc dependency already exists in cc-switch for Unix.
- MSRV unchanged: 1.88+, edition 2024.
- Release binary size will grow (axum, reqwest, rust-embed pulled in).
  Estimated +5MB after LTO; acceptable.
- No new runtime requirements: pure Rust, no node, no system services.

## 14. Spec C+ follow-ups (separate brainstorming)

Deferred for later specs, listed here so reviewers can see they're not
forgotten:

- **Codex daemon**: add `openai_base_url` to `CodexConfiguration`; teach
  `cc codex use` to inject `OPENAI_BASE_URL`; daemon spawns codex-provider
  proxies the same way.
- **Auto-respawn from `cc use`**: if state file says daemon should be up
  but pidfile is stale, fork a new daemon transparently with a ~3s
  readiness timeout, then proceed.
- **Lazy proxy spawning**: instead of eager on `daemon start`, spawn on
  first `cc use <alias>` for that upstream. Reduces idle resource use for
  users with many aliases.
- **Per-alias opt-out**: `cc-switch add ... --no-proxy` to mark an alias as
  always-direct (useful for low-latency setups or aliases that already
  point at a proxy).
- **Daemon reload without restart**: SIGHUP triggers config re-read +
  diff-based proxy add/remove without killing in-flight requests.
- **Multi-tenancy**: one daemon per `$HOME` is fine; if a user wants
  multiple isolated daemons (different data roots), introduce
  `--state-dir`.

## 15. Approved design decisions (reference)

- Branch: extend `feat/ccs-proxy-v1` (one combined PR).
- ccs-proxy consumed as **library**, in-process (no child processes).
- One proxy **per unique upstream URL** (claude provider only); aliases
  sharing an upstream URL share a proxy.
- Eager spawn at `cc daemon start`. Adding aliases later requires
  `cc daemon restart`.
- `cc use` silent fallback to direct when daemon is off, with **one-line
  hint** to stderr.
- Anti-entropy = pidfile-based liveness; **no auto-respawn from `cc use`** in v1.
- Daemon process model: **double-fork into background** on Unix; `--foreground`
  for interactive use.
- State file is the **only** cross-process contract (no IPC socket / DBus).
- Codex deferred until codex aliases gain a URL field.
- Windows: unsupported in v1, clear error pointing at `ccs-proxy serve`.

## 16. Open work to verify before implementation

- Confirm `libc::fork` / `setsid` work as expected on the macOS CI runner
  (Linux is well-trodden). Write a tiny spike before locking in fork.rs.
- Confirm the supervisor's `JoinHandle::is_finished()` pattern catches
  axum task panics (vs needing a `tokio::spawn` + abort channel). Test
  case in `tests/daemon_supervisor.rs` will pin this down.
- Decide whether `cc daemon status` should attempt to start a tokio
  runtime just to issue the health probes — alternative is a blocking
  `ureq` call to keep status quick and dep-light. Default to ureq if
  cc-switch doesn't already pull reqwest (it doesn't); add `ureq = "2"`
  to cc-switch dev/runtime deps for this purpose.
