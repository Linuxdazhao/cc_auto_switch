//! Daemon subsystem: in-process supervision of one ccs-proxy per upstream URL.
//!
//! See docs/superpowers/specs/2026-05-28-cc-switch-daemon-design.md for design.

pub mod aggregate;
pub mod commands;
pub mod pidfile;
pub mod state;
pub mod status;

#[cfg(unix)]
pub mod fork;
#[cfg(unix)]
pub mod lifecycle;
#[cfg(unix)]
pub mod logging;

pub use commands::{DaemonAction, handle_daemon_command};

use crate::daemon::pidfile::{Pidfile, process_alive};
use crate::daemon::state::{CURRENT_VERSION, DaemonState};

/// If the daemon is alive but was started by a `cc-switch` binary whose version
/// differs from this one, return a warning message. Returns `None` when the
/// daemon is stopped or its version matches the current binary.
///
/// A stale daemon may expose different proxy/api ports or capture semantics, so
/// the user should restart it after upgrading `cc-switch`.
pub fn version_mismatch_warning() -> Option<String> {
    let home = dirs::home_dir()?;
    let cc_switch_dir = home.join(".cc-switch");
    let state = DaemonState::load(&cc_switch_dir.join("daemon-state.json")).ok()??;

    // Only warn when a daemon is actually running.
    let pidfile = Pidfile::new(cc_switch_dir.join("daemon.pid"));
    let pid = pidfile.read().ok()??;
    if !matches!(process_alive(pid), Ok(true)) {
        return None;
    }

    if !state.version_mismatch() {
        return None;
    }

    let running = if state.version.is_empty() {
        "unknown (pre-version)".to_string()
    } else {
        state.version.clone()
    };
    Some(format!(
        "cc daemon is running an outdated version (daemon {running}, CLI {CURRENT_VERSION}) — proxy ports/capture may be stale. Run `cc-switch daemon restart`."
    ))
}

/// Print the version-mismatch warning in red, if one applies. No-op otherwise.
pub fn print_version_mismatch_warning() {
    use colored::Colorize;
    if let Some(msg) = version_mismatch_warning() {
        eprintln!("{}", format!("\u{26a0} {msg}").red().bold());
    }
}

/// Result of attempting to resolve a proxy URL for a given upstream.
pub enum ProxyResolution {
    /// Daemon is running and has a matching proxy.
    Proxied { proxy_url: String },
    /// Daemon is not running or has no match; use direct URL.
    Direct,
}

/// Check whether the daemon is alive and has a proxy for the given upstream URL.
/// Returns the proxy URL (http://127.0.0.1:<port>) if available, otherwise Direct.
pub fn try_resolve_proxy(upstream: &str) -> ProxyResolution {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return ProxyResolution::Direct,
    };
    let cc_switch_dir = home.join(".cc-switch");
    let state_path = cc_switch_dir.join("daemon-state.json");
    let pidfile_path = cc_switch_dir.join("daemon.pid");

    try_resolve_proxy_from_paths(upstream, &state_path, &pidfile_path)
}

/// Path-injected core of [`try_resolve_proxy`]. Exposed to integration tests so
/// they can exercise resolution against a temp `~/.cc-switch` instead of the
/// real one (which may have a live daemon — see the daemon_integration tests).
pub fn try_resolve_proxy_from_paths(
    upstream: &str,
    state_path: &std::path::Path,
    pidfile_path: &std::path::Path,
) -> ProxyResolution {
    let state = match DaemonState::load(state_path) {
        Ok(Some(s)) => s,
        _ => return ProxyResolution::Direct,
    };

    let pidfile = Pidfile::new(pidfile_path.to_path_buf());
    let pid = match pidfile.read() {
        Ok(Some(pid)) => pid,
        _ => return ProxyResolution::Direct,
    };

    match process_alive(pid) {
        Ok(true) => {}
        _ => return ProxyResolution::Direct,
    }

    match state.find_proxy("claude", upstream) {
        Some(entry) => ProxyResolution::Proxied {
            proxy_url: format!("http://127.0.0.1:{}", entry.proxy_port),
        },
        None => ProxyResolution::Direct,
    }
}

/// Official Anthropic upstream URL. The daemon spawns a ccs-proxy for this
/// URL **only** when started with `--capture-official`; by default `cc use
/// official` traffic flows direct to Anthropic and is not captured.
///
/// MUST stay byte-identical to Claude CLI's default `ANTHROPIC_BASE_URL`, since
/// `find_proxy` does literal string match.
pub const OFFICIAL_UPSTREAM: &str = "https://api.anthropic.com";

/// Build the `EnvironmentConfig` for the "official" launch path, printing
/// user-facing status in blue. Returns an env with `CC_SWITCH_CURRENT_ALIAS`
/// set to `"official"`, plus `ANTHROPIC_BASE_URL` pointing at the daemon proxy
/// if it's running. Never sets `ANTHROPIC_AUTH_TOKEN` — Claude CLI's OAuth
/// credentials must flow through unchanged.
pub fn build_official_env() -> crate::config::config::EnvironmentConfig {
    use crate::config::config::EnvironmentConfig;
    use colored::Colorize;
    let env = EnvironmentConfig::empty().with_alias("official");
    match try_resolve_proxy(OFFICIAL_UPSTREAM) {
        ProxyResolution::Proxied { proxy_url } => {
            eprintln!(
                "{}",
                format!("\u{2139} Routing official traffic through cc daemon proxy at {proxy_url}")
                    .blue()
            );
            env.with_base_url(proxy_url)
        }
        ProxyResolution::Direct => {
            eprintln!(
                "{}",
                "\u{2139} official traffic is going direct to Anthropic (not captured).".blue()
            );
            eprintln!(
                "{}",
                "  Start the daemon with `cc-switch daemon start --capture-official` to capture it."
                    .blue()
            );
            env
        }
    }
}
