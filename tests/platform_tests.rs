//! Tests for the platform resolver helper.
//!
//! These tests are platform-independent — Windows-only PATHEXT probing is
//! exercised by the CI `shell-smoke` job, not here.

use cc_switch::platform::{resolve_npm_cli, unicode_support_enabled};
use std::path::PathBuf;

#[test]
fn env_override_takes_precedence() {
    // SAFETY: tests do not run in parallel within a single test binary by default
    //         only when using cargo nextest. The env-var name is unique to this test.
    unsafe {
        std::env::set_var("FOOTESTBIN_BINARY", "/custom/path/to/foo");
    }
    let resolved = resolve_npm_cli("footestbin");
    unsafe {
        std::env::remove_var("FOOTESTBIN_BINARY");
    }
    assert_eq!(resolved, PathBuf::from("/custom/path/to/foo"));
}

#[test]
fn no_override_no_path_match_falls_back_to_bare_name() {
    let unique = "definitely_nonexistent_binary_xyz123abc";
    let resolved = resolve_npm_cli(unique);
    assert_eq!(resolved, PathBuf::from(unique));
}

#[test]
fn ascii_env_override_forces_ascii() {
    unsafe {
        std::env::set_var("CC_SWITCH_ASCII", "1");
    }
    let supported = unicode_support_enabled();
    unsafe {
        std::env::remove_var("CC_SWITCH_ASCII");
    }
    assert!(!supported, "CC_SWITCH_ASCII=1 must force ASCII fallback");
}
