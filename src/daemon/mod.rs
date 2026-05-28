//! Daemon subsystem: in-process supervision of one ccs-proxy per upstream URL.
//!
//! See docs/superpowers/specs/2026-05-28-cc-switch-daemon-design.md for design.

pub mod commands;
pub mod lifecycle;
pub mod pidfile;
pub mod state;
pub mod status;

#[cfg(unix)]
pub mod fork;

pub use commands::{DaemonAction, handle_daemon_command};
