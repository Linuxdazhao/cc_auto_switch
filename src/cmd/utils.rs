use anyhow::{Context, Result};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

/// Get the path to the configuration storage file
///
/// Returns `~/.cc_auto_switch/configurations.json`
///
/// # Errors
/// Returns error if home directory cannot be found
pub fn get_config_storage_path() -> Result<PathBuf> {
    let home_dir = dirs::home_dir().context("Could not find home directory")?;
    Ok(home_dir.join(".cc-switch").join("configurations.json"))
}

/// Get the path to the Claude settings file
///
/// Returns the path to settings.json, using custom directory if configured
/// Defaults to `~/.claude/settings.json`
///
/// # Errors
/// Returns error if home directory cannot be found or path is invalid
pub fn get_claude_settings_path(custom_dir: Option<&str>) -> Result<PathBuf> {
    if let Some(dir) = custom_dir {
        let custom_path = PathBuf::from(dir);
        if custom_path.is_absolute() {
            Ok(custom_path.join("settings.json"))
        } else {
            // If relative path, resolve from home directory
            let home_dir = dirs::home_dir().context("Could not find home directory")?;
            Ok(home_dir.join(custom_path).join("settings.json"))
        }
    } else {
        // Default path
        let home_dir = dirs::home_dir().context("Could not find home directory")?;
        Ok(home_dir.join(".claude").join("settings.json"))
    }
}

/// Read input from stdin with a prompt
///
/// # Arguments
/// * `prompt` - The prompt to display to the user
///
/// # Returns
/// The user's input as a String
pub fn read_input(prompt: &str) -> Result<String> {
    print!("{prompt}");
    io::stdout().flush().context("Failed to flush stdout")?;
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .context("Failed to read input")?;
    Ok(input.trim().to_string())
}

/// Read sensitive input (token) with a prompt (without echoing)
///
/// # Arguments
/// * `prompt` - The prompt to display to the user
///
/// # Returns
/// The user's input as a String
pub fn read_sensitive_input(prompt: &str) -> Result<String> {
    print!("{prompt}");
    io::stdout().flush().context("Failed to flush stdout")?;
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .context("Failed to read input")?;
    Ok(input.trim().to_string())
}

/// Validate alias name
///
/// # Arguments
/// * `alias_name` - The alias name to validate
///
/// # Returns
/// Ok(()) if valid, Err with message if invalid
pub fn validate_alias_name(alias_name: &str) -> Result<()> {
    if alias_name.is_empty() {
        anyhow::bail!("Alias name cannot be empty");
    }
    if alias_name == "cc" {
        anyhow::bail!("Alias name 'cc' is reserved and cannot be used");
    }
    if alias_name.chars().any(|c| c.is_whitespace()) {
        anyhow::bail!("Alias name cannot contain whitespace");
    }
    Ok(())
}

/// Execute claude command with or without --dangerously-skip-permissions
///
/// # Arguments
/// * `skip_permissions` - Whether to add --dangerously-skip-permissions flag
pub fn execute_claude_command(skip_permissions: bool) -> Result<()> {
    let mut command = Command::new("claude");
    if skip_permissions {
        command.arg("--dangerously-skip-permissions");
    }

    command
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let mut child = command.spawn().with_context(
        || "Failed to launch Claude CLI. Make sure 'claude' command is available in PATH",
    )?;

    let status = child
        .wait()
        .with_context(|| "Failed to wait for Claude CLI process")?;

    if !status.success() {
        anyhow::bail!("Claude CLI exited with error status: {}", status);
    }

    Ok(())
}

/// Launch Claude CLI with proper delay
pub fn launch_claude() -> Result<()> {
    println!("\nWaiting 0.5 seconds before launching Claude...");
    thread::sleep(Duration::from_millis(500));

    println!("Launching Claude CLI...");
    let mut child = Command::new("claude")
        .arg("--dangerously-skip-permissions")
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(
            || "Failed to launch Claude CLI. Make sure 'claude' command is available in PATH",
        )?;

    let status = child.wait()?;

    if !status.success() {
        anyhow::bail!("Claude CLI exited with error status: {}", status);
    }

    Ok(())
}