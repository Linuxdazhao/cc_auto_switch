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

# Clean up orphaned alias files for dead processes (runs in background to avoid latency)
cleanup_orphans() {{
  for f in "$HOME/.claude/cc_auto_switch_alias_"*; do
    [ -f "$f" ] || continue
    local pid="${{f##*_}}"
    if ! kill -0 "$pid" 2>/dev/null; then
      rm -f "$f"
    fi
  done
}}
cleanup_orphans &

# Traverse parent process chain to find the real Claude process
# Claude Code spawns statusLine via an intermediate process, so $PPID is not
# the Claude main process. We walk up the process tree to find a process
# whose name contains 'claude' or 'node' and has a per-PID alias file.
find_claude_pid() {{
  local current_pid=$PPID
  local max_depth=10
  local depth=0

  while [ $depth -lt $max_depth ] && [ $current_pid -gt 1 ]; do
    local proc_info=$(ps -p $current_pid -o pid,ppid,comm 2>/dev/null | tail -1)
    if [ -z "$proc_info" ]; then
      break
    fi

    local pid=$(echo "$proc_info" | awk '{{print $1}}')
    local ppid=$(echo "$proc_info" | awk '{{print $2}}')
    local comm=$(echo "$proc_info" | awk '{{print $3}}')

    # Check if this is claude or node (Claude Code runs on Node.js)
    if [[ "$comm" == *"claude"* ]] || [[ "$comm" == *"node"* ]]; then
      if [ -f "$HOME/.claude/cc_auto_switch_alias_${{pid}}" ]; then
        echo $pid
        return 0
      fi
    fi

    current_pid=$ppid
    depth=$((depth + 1))
  done
  return 1
}}

alias_name=""
# Priority: env var (per-session, most reliable) > per-PID file (per-session)
# The env var CC_SWITCH_CURRENT_ALIAS is set by cc-switch when launching Claude and inherited
# by all child processes. It is the most reliable source because it is per-session and cannot
# be contaminated by other sessions. The per-PID file is a fallback for sessions where the
# env var is not available. The global file is NOT used as a fallback because it is shared
# across all sessions and overwritten by every `cs use` invocation, which would cause
# cross-session alias contamination.
if [ -n "$CC_SWITCH_CURRENT_ALIAS" ]; then
  alias_name="$CC_SWITCH_CURRENT_ALIAS"
else
  claude_pid=$(find_claude_pid)
  if [ -n "$claude_pid" ] && [ -f "$HOME/.claude/cc_auto_switch_alias_${{claude_pid}}" ]; then
    alias_name=$(cat "$HOME/.claude/cc_auto_switch_alias_${{claude_pid}}" 2>/dev/null)
  fi
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
        assert!(script.contains("CC_SWITCH_CURRENT_ALIAS"));
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

    #[test]
    fn test_env_var_has_highest_priority_in_script() {
        // The $CC_SWITCH_CURRENT_ALIAS env var is set per-session by cs use and
        // inherited by the statusline subprocess. It must be checked BEFORE the
        // per-PID file and global file lookups, because:
        // 1. Per-PID files don't exist for sessions launched before the feature
        // 2. find_claude_pid() can match the wrong claude process when many
        //    sessions are running
        // 3. The global file is overwritten by every cs use in any terminal,
        //    which would change the alias shown in ALL running sessions
        let cmd = "bunx -y ccstatusline@latest";
        let script = generate_script(cmd);

        // Find the position of the env var check and the per-PID file check
        let env_var_pos = script
            .find("CC_SWITCH_CURRENT_ALIAS")
            .expect("Script must reference CC_SWITCH_CURRENT_ALIAS");
        let find_claude_pid_call_pos = script
            .find("$(find_claude_pid)")
            .expect("Script must call find_claude_pid");

        // The env var check must come BEFORE the find_claude_pid() call
        assert!(
            env_var_pos < find_claude_pid_call_pos,
            "CC_SWITCH_CURRENT_ALIAS env var must be checked before find_claude_pid() \
             to prevent cross-session alias contamination"
        );
    }

    #[test]
    fn test_global_file_not_used_as_fallback() {
        // The global file cc_auto_switch_current_alias is shared across all sessions
        // and overwritten by every `cs use` invocation. Using it as a fallback would
        // cause cross-session alias contamination: running `cs use xxx` in one terminal
        // would change the alias displayed in ALL running sessions.
        // The script must NOT read from the global file.
        let cmd = "bunx -y ccstatusline@latest";
        let script = generate_script(cmd);

        // The alias detection logic (after find_claude_pid function) must not reference
        // the global file. The find_claude_pid function body references per-PID files
        // (cc_auto_switch_alias_${pid}) which is fine.
        let alias_detection_section = script
            .split("alias_name=\"\"")
            .nth(1)
            .expect("Script must have alias detection section");

        assert!(
            !alias_detection_section.contains("cc_auto_switch_current_alias"),
            "The alias detection section must NOT use the global file \
             (cc_auto_switch_current_alias) as a fallback, because it is shared \
             across all sessions and causes cross-session contamination"
        );
    }
}
