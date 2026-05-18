//! StatusLine integration module
//!
//! Provides functionality to install/uninstall a wrapper script that displays
//! the current cc-switch alias name in Claude Code's statusLine.

use anyhow::{Context, Result};
use base64::Engine;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use crate::config::types::ClaudeSettings;

/// Default statusLine command if none is configured
const DEFAULT_STATUSLINE_CMD: &str = "bunx -y ccstatusline@latest";

/// Marker comment prefix for storing original command
const MARKER_PREFIX: &str = "# CC_SWITCH_ORIGINAL_CMD: ";

/// Check if a command is available in PATH
fn is_command_available(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Detect available package manager for running ccstatusline
///
/// Priority:
/// 1. bun (bunx) - faster
/// 2. npm (npx) - fallback
///
/// Returns the command string to use, or None if neither is available.
fn detect_statusline_runner() -> Option<&'static str> {
    if is_command_available("bun") {
        Some("bunx -y ccstatusline@latest")
    } else if is_command_available("npx") {
        Some("npx -y ccstatusline@latest")
    } else {
        None
    }
}

/// Get the path to the wrapper script
fn get_wrapper_script_path() -> Result<PathBuf> {
    let config_file = crate::config::get_config_storage_path()?;
    let config_dir = config_file
        .parent()
        .context("Could not get config directory")?;
    Ok(config_dir.join("cc_auto_switch_statusline.sh"))
}

/// Generate the wrapper script content
fn generate_script(original_cmd: &str) -> String {
    let encoded = base64::engine::general_purpose::STANDARD.encode(original_cmd);
    format!(
        r#"#!/usr/bin/env bash
{marker}{encoded}
alias_name=""
# Priority: environment variable (per-session) > file (global fallback)
if [ -n "$CC_SWITCH_CURRENT_ALIAS" ]; then
  alias_name="$CC_SWITCH_CURRENT_ALIAS"
elif [ -f "$HOME/.claude/cc_auto_switch_current_alias" ]; then
  alias_name=$(cat "$HOME/.claude/cc_auto_switch_current_alias" 2>/dev/null)
fi
if [ -n "$alias_name" ]; then
  printf '[%s] ' "$alias_name"
fi
{original_cmd}
"#,
        marker = MARKER_PREFIX,
        encoded = encoded,
        original_cmd = original_cmd,
    )
}

/// Extract the original command from a wrapper script
fn extract_original_cmd(script_content: &str) -> Option<String> {
    for line in script_content.lines() {
        if let Some(encoded) = line.strip_prefix(MARKER_PREFIX)
            && let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(encoded)
        {
            return String::from_utf8(decoded).ok();
        }
    }
    None
}

/// Install the statusLine wrapper script
///
/// Reads the current statusLine command from settings.json, generates a wrapper
/// script that prepends the current alias name, and updates settings.json to
/// use the wrapper script.
///
/// If no statusLine is configured, it will detect the available package manager
/// (bun or npm) and use the appropriate command.
pub fn install(custom_dir: Option<&str>) -> Result<()> {
    let mut settings = ClaudeSettings::load(custom_dir)?;
    let wrapper_path = get_wrapper_script_path()?;

    // Get current statusLine command, or detect available runner if none configured
    let has_existing = settings
        .other
        .get("statusLine")
        .and_then(|v| v.get("command"))
        .is_some();

    let original_cmd = if has_existing {
        let current_cmd = settings
            .other
            .get("statusLine")
            .and_then(|v| v.get("command"))
            .and_then(|v| v.as_str())
            .unwrap_or(DEFAULT_STATUSLINE_CMD)
            .to_string();

        // Check if current command is the wrapper script itself (recursive installation)
        if current_cmd.contains("cc_auto_switch_statusline.sh") {
            // Try to extract original command from existing wrapper
            if wrapper_path.exists()
                && let Ok(existing) = fs::read_to_string(&wrapper_path)
                && let Some(existing_cmd) = extract_original_cmd(&existing)
            {
                existing_cmd
            } else {
                // No existing wrapper or can't extract, detect package manager
                match detect_statusline_runner() {
                    Some(cmd) => {
                        println!(
                            "Detected package manager: {}",
                            if cmd.contains("bun") { "bun" } else { "npm" }
                        );
                        cmd.to_string()
                    }
                    None => {
                        anyhow::bail!(
                            "No package manager found (bun or npm required for ccstatusline).\n\
                             Please install bun or npm, then run: cc-switch statusline install"
                        );
                    }
                }
            }
        } else {
            current_cmd
        }
    } else {
        // No existing statusLine, detect available package manager
        match detect_statusline_runner() {
            Some(cmd) => {
                println!(
                    "Detected package manager: {}",
                    if cmd.contains("bun") { "bun" } else { "npm" }
                );
                cmd.to_string()
            }
            None => {
                anyhow::bail!(
                    "No package manager found (bun or npm required for ccstatusline).\n\
                     Please install bun or npm, then run: cc-switch statusline install"
                );
            }
        }
    };

    // Check if already installed with same command
    if wrapper_path.exists()
        && let Ok(existing) = fs::read_to_string(&wrapper_path)
        && let Some(existing_cmd) = extract_original_cmd(&existing)
        && existing_cmd == original_cmd
    {
        println!("StatusLine wrapper already installed with the same command.");
        return Ok(());
    }

    // Generate wrapper script
    let script = generate_script(&original_cmd);

    // Create directory if needed
    if let Some(parent) = wrapper_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }

    // Write script
    fs::write(&wrapper_path, &script).with_context(|| {
        format!(
            "Failed to write wrapper script to {}",
            wrapper_path.display()
        )
    })?;

    // Make executable
    #[cfg(unix)]
    {
        let mut perms = fs::metadata(&wrapper_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&wrapper_path, perms)?;
    }

    // Update settings.json
    let wrapper_cmd = format!("bash {}", wrapper_path.display());

    // Build new statusLine object
    let mut status_line = serde_json::Map::new();
    status_line.insert(
        "type".to_string(),
        serde_json::Value::String("command".to_string()),
    );
    status_line.insert(
        "command".to_string(),
        serde_json::Value::String(wrapper_cmd.clone()),
    );

    // Preserve padding if it existed
    if let Some(existing) = settings.other.get("statusLine") {
        if let Some(padding) = existing.get("padding") {
            status_line.insert("padding".to_string(), padding.clone());
        }
    } else {
        status_line.insert(
            "padding".to_string(),
            serde_json::Value::Number(serde_json::Number::from(0)),
        );
    }

    settings.other.insert(
        "statusLine".to_string(),
        serde_json::Value::Object(status_line),
    );

    settings.save(custom_dir)?;

    println!("StatusLine wrapper installed successfully!");
    println!("  Script: {}", wrapper_path.display());
    println!("  Command: {}", wrapper_cmd);
    println!();
    println!("The current cc-switch alias name will now be displayed in the status line.");

    if has_existing {
        println!();
        println!("Existing statusLine configuration detected and preserved.");
    } else {
        println!();
        println!("To customize ccstatusline configuration, run one of:");
        println!("  bunx -y ccstatusline@latest --help");
        println!("  npx -y ccstatusline@latest --help");
    }

    Ok(())
}

/// Uninstall the statusLine wrapper script
///
/// Restores the original statusLine command and removes the wrapper script.
pub fn uninstall(custom_dir: Option<&str>) -> Result<()> {
    let wrapper_path = get_wrapper_script_path()?;

    if !wrapper_path.exists() {
        println!("StatusLine wrapper is not installed.");
        return Ok(());
    }

    // Read original command from wrapper
    let script_content =
        fs::read_to_string(&wrapper_path).with_context(|| "Failed to read wrapper script")?;

    let original_cmd = extract_original_cmd(&script_content);

    // Update settings.json
    let mut settings = ClaudeSettings::load(custom_dir)?;

    if let Some(cmd) = original_cmd {
        // Restore original command
        if let Some(status_line) = settings.other.get_mut("statusLine")
            && let Some(obj) = status_line.as_object_mut()
        {
            obj.insert(
                "command".to_string(),
                serde_json::Value::String(cmd.clone()),
            );
        }
        println!("Restored original statusLine command: {}", cmd);
    } else {
        // No original command found, remove statusLine entirely
        settings.other.remove("statusLine");
        println!("Removed statusLine configuration (no original command found).");
    }

    settings.save(custom_dir)?;

    // Remove wrapper script
    fs::remove_file(&wrapper_path)
        .with_context(|| format!("Failed to remove {}", wrapper_path.display()))?;

    println!("StatusLine wrapper uninstalled successfully.");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_script() {
        let cmd = "bunx -y ccstatusline@latest";
        let script = generate_script(cmd);
        assert!(script.contains("#!/usr/bin/env bash"));
        assert!(script.contains(MARKER_PREFIX));
        assert!(script.contains(cmd));
        assert!(script.contains("cc_auto_switch_current_alias"));
    }

    #[test]
    fn test_extract_original_cmd() {
        let cmd = "bunx -y ccstatusline@latest";
        let script = generate_script(cmd);
        let extracted = extract_original_cmd(&script);
        assert_eq!(extracted, Some(cmd.to_string()));
    }

    #[test]
    fn test_extract_original_cmd_missing() {
        let script = "#!/usr/bin/env bash\necho hello";
        let extracted = extract_original_cmd(script);
        assert_eq!(extracted, None);
    }
}
