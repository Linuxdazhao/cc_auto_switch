//! Cross-platform helpers for binary resolution and terminal capability detection.
//!
//! `resolve_npm_cli` exists because npm-installed CLIs on Windows ship as `.cmd` or
//! `.ps1` shims rather than `.exe`, and `std::process::Command::new("name")` does
//! not always pick them up depending on how PATHEXT is configured.
//!
//! `unicode_support_enabled` centralizes the heuristic used by the interactive UI
//! to decide between Unicode box-drawing and ASCII fallback.

use std::path::PathBuf;

/// Resolve a Node/npm-style CLI name to an executable path.
///
/// Resolution order:
/// 1. Env var `<NAME_UPPERCASE>_BINARY` (e.g. `CLAUDE_BINARY`, `CODEX_BINARY`) —
///    if set, returned verbatim. Lets users pin a specific binary.
/// 2. On Windows: probe `<name>.exe`, then `<name>.cmd`, then `<name>.ps1` via
///    the `which` crate (which respects PATH and is independent of cmd.exe's
///    PATHEXT handling).
/// 3. Fallback: return `PathBuf::from(name)` so `Command::new()` behaves
///    identically to the pre-resolver code on platforms where probing finds
///    nothing.
pub fn resolve_npm_cli(name: &str) -> PathBuf {
    let override_var = format!("{}_BINARY", name.to_uppercase());
    if let Ok(path) = std::env::var(&override_var) {
        return PathBuf::from(path);
    }

    #[cfg(windows)]
    {
        for ext in ["exe", "cmd", "ps1"] {
            let candidate = format!("{name}.{ext}");
            if let Ok(path) = which::which(&candidate) {
                return path;
            }
        }
    }

    PathBuf::from(name)
}

/// Decide whether the terminal supports Unicode box-drawing characters.
///
/// Precedence (highest first):
/// 1. `CC_SWITCH_ASCII=1` → force ASCII (escape hatch for any terminal).
/// 2. `CC_SWITCH_UNICODE=1` → force Unicode (escape hatch for Windows users
///    on terminals our heuristic can't detect).
/// 3. On Windows: only enable Unicode if `WT_SESSION` is set (Windows
///    Terminal). Default to ASCII to avoid mojibake on legacy conhost / CMD
///    where the default codepage is not UTF-8.
/// 4. On non-Windows: enabled by default. Modern Linux and macOS terminals
///    universally support UTF-8; the `CC_SWITCH_ASCII=1` escape hatch above
///    covers exceptions.
pub fn unicode_support_enabled() -> bool {
    if std::env::var("CC_SWITCH_ASCII").is_ok_and(|v| v == "1") {
        return false;
    }
    if std::env::var("CC_SWITCH_UNICODE").is_ok_and(|v| v == "1") {
        return true;
    }

    #[cfg(windows)]
    {
        return std::env::var("WT_SESSION").is_ok();
    }

    #[cfg(not(windows))]
    {
        // Default to enabled — modern *nix terminals (Linux, macOS) universally
        // support UTF-8. The CC_SWITCH_ASCII=1 escape hatch above covers exceptions.
        true
    }
}
