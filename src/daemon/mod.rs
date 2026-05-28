//! Daemon subsystem: in-process supervision of one ccs-proxy per upstream URL.
//!
//! See docs/superpowers/specs/2026-05-28-cc-switch-daemon-design.md for design.

pub mod aggregate;
pub mod commands;
pub mod lifecycle;
pub mod logging;
pub mod pidfile;
pub mod state;
pub mod status;

#[cfg(unix)]
pub mod fork;

pub use commands::{DaemonAction, handle_daemon_command};

use crate::daemon::pidfile::{Pidfile, process_alive};
use crate::daemon::state::DaemonState;

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

fn try_resolve_proxy_from_paths(
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
