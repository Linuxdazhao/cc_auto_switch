use crate::cmd::config::{EnvironmentConfig, Configuration, ConfigStorage};
use anyhow::{Context, Result};
use colored::*;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute, terminal,
};
use std::io::{self, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

/// Handle interactive current command
///
/// Provides interactive menu for:
/// 1. Execute claude --dangerously-skip-permissions
/// 2. Switch configuration (lists available aliases)
/// 3. Exit
///
/// # Errors
/// Returns error if file operations fail or user input fails
pub fn handle_current_command() -> Result<()> {
    let storage = ConfigStorage::load()?;

    println!("\n{}", "Current Configuration:".green().bold());
    println!("Environment variable mode: configurations are set per-command execution");
    println!("Use 'cc-switch use <alias>' to launch Claude with specific configuration");
    println!("Use 'cc-switch use cc' to launch Claude with default settings");

    // Try to enable interactive menu with keyboard navigation
    let raw_mode_enabled = terminal::enable_raw_mode().is_ok();

    if raw_mode_enabled {
        let mut stdout = io::stdout();
        if execute!(
            stdout,
            terminal::EnterAlternateScreen,
            terminal::Clear(terminal::ClearType::All)
        )
        .is_ok()
        {
            // Full interactive mode with arrow keys for main menu
            let result = handle_main_menu_interactive(&mut stdout, &storage);

            // Always restore terminal
            let _ = execute!(stdout, terminal::LeaveAlternateScreen);
            let _ = terminal::disable_raw_mode();

            return result;
        } else {
            // Fallback to simple mode
            let _ = terminal::disable_raw_mode();
        }
    }

    // Fallback to simple numbered menu
    handle_main_menu_simple(&storage)
}

/// Handle main menu with keyboard navigation
fn handle_main_menu_interactive(
    stdout: &mut io::Stdout,
    storage: &ConfigStorage,
) -> Result<()> {
    let menu_items = [
        "Execute claude --dangerously-skip-permissions",
        "Switch configuration",
        "Exit",
    ];
    let mut selected_index = 0;

    loop {
        // Clear screen and redraw
        execute!(stdout, terminal::Clear(terminal::ClearType::All))?;
        execute!(stdout, crossterm::cursor::MoveTo(0, 0))?;

        // Header
        println!("\r{}", "╔══ Main Menu ══╗".green().bold());
        println!(
            "\r{}",
            "║ Use ↑↓ arrows, Enter to select, Esc to exit".dimmed()
        );
        println!("\r{}", "╚═══════════════╝".green().bold());
        println!();

        // Draw menu items
        for (index, item) in menu_items.iter().enumerate() {
            if index == selected_index {
                println!("\r> {} {}", "●".blue().bold(), item.blue().bold());
            } else {
                println!("\r  {} {}", "○".dimmed(), item.dimmed());
            }
        }

        // Ensure output is flushed
        stdout.flush()?;

        // Handle input
        match event::read()? {
            Event::Key(KeyEvent {
                code,
                kind: KeyEventKind::Press,
                ..
            }) => {
                match code {
                    KeyCode::Up => {
                        selected_index = selected_index.saturating_sub(1);
                    }
                    KeyCode::Down => {
                        if selected_index < menu_items.len() - 1 {
                            selected_index += 1;
                        }
                    }
                    KeyCode::Enter => {
                        // Execute terminal cleanup here
                        let _ = execute!(stdout, terminal::LeaveAlternateScreen);
                        let _ = terminal::disable_raw_mode();

                        return handle_main_menu_action(selected_index, storage);
                    }
                    KeyCode::Esc => {
                        println!("Exiting...");
                        return Ok(());
                    }
                    _ => {}
                }
            }
            Event::Key(_) => {} // Ignore key release events
            _ => {}
        }
    }
}

/// Handle main menu simple fallback
fn handle_main_menu_simple(storage: &ConfigStorage) -> Result<()> {
    loop {
        println!("\n{}", "Available Actions:".blue().bold());
        println!("1. Execute claude --dangerously-skip-permissions");
        println!("2. Switch configuration");
        println!("3. Exit");

        print!("\nPlease select an option (1-3): ");
        io::stdout().flush().context("Failed to flush stdout")?;

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .context("Failed to read input")?;

        let choice = input.trim();

        match choice {
            "1" => return handle_main_menu_action(0, storage),
            "2" => return handle_main_menu_action(1, storage),
            "3" => return handle_main_menu_action(2, storage),
            _ => {
                println!("Invalid option. Please select 1-3.");
            }
        }
    }
}

/// Handle main menu action based on selected index
fn handle_main_menu_action(
    selected_index: usize,
    storage: &ConfigStorage,
) -> Result<()> {
    match selected_index {
        0 => {
            println!("\nExecuting: claude --dangerously-skip-permissions");
            execute_claude_command(true)?;
        }
        1 => {
            // Use the interactive selection instead of simple menu
            handle_interactive_selection(storage)?;
        }
        2 => {
            println!("Exiting...");
        }
        _ => {
            println!("Invalid selection");
        }
    }
    Ok(())
}

/// Handle interactive configuration selection with real-time preview
///
/// # Arguments
/// * `storage` - Reference to configuration storage
///
/// # Errors
/// Returns error if terminal operations fail or user selection fails
pub fn handle_interactive_selection(storage: &ConfigStorage) -> Result<()> {
    if storage.configurations.is_empty() {
        println!("No configurations available. Use 'add' command to create configurations first.");
        return Ok(());
    }

    let mut configs: Vec<&Configuration> = storage.configurations.values().collect();
    configs.sort_by(|a, b| a.alias_name.cmp(&b.alias_name));

    let mut selected_index = 0;

    // Try to enable raw mode, fallback to simple menu if it fails
    let raw_mode_enabled = terminal::enable_raw_mode().is_ok();

    if raw_mode_enabled {
        let mut stdout = io::stdout();
        if execute!(
            stdout,
            terminal::EnterAlternateScreen,
            terminal::Clear(terminal::ClearType::All)
        )
        .is_ok()
        {
            // Full interactive mode with arrow keys
            let result = handle_full_interactive_menu(
                &mut stdout,
                &configs,
                &mut selected_index,
            );

            // Always restore terminal
            let _ = execute!(stdout, terminal::LeaveAlternateScreen);
            let _ = terminal::disable_raw_mode();

            return result;
        } else {
            // Fallback to simple mode
            let _ = terminal::disable_raw_mode();
        }
    }

    // Fallback to simple numbered menu
    handle_simple_interactive_menu(&configs, storage)
}

/// Handle full interactive menu with arrow key navigation
fn handle_full_interactive_menu(
    stdout: &mut io::Stdout,
    configs: &[&Configuration],
    selected_index: &mut usize,
) -> Result<()> {
    loop {
        // Clear screen and redraw
        execute!(stdout, terminal::Clear(terminal::ClearType::All))?;
        execute!(stdout, crossterm::cursor::MoveTo(0, 0))?;

        // Header with better formatting
        println!("\r{}", "╔══ Select Configuration ══╗".green().bold());
        println!(
            "\r{}",
            "║ Use ↑↓ arrows, Enter to select, Esc to cancel".dimmed()
        );
        println!("\r{}", "╚═══════════════════════════╝".green().bold());
        println!();

        // Draw menu with better alignment
        for (index, config) in configs.iter().enumerate() {
            if index == *selected_index {
                println!(
                    "\r> {} {}",
                    "●".blue().bold(),
                    config.alias_name.blue().bold()
                );
                // Show details with better formatting
                println!(
                    "\r    Token: {}",
                    format!(
                        "{}...{}",
                        &config.token[..12],
                        &config.token[config.token.len() - 8..]
                    )
                    .dimmed()
                );
                println!("\r    URL: {}", config.url.cyan());
                if let Some(model) = &config.model {
                    println!("\r    Model: {}", model.yellow());
                }
                if let Some(small_fast_model) = &config.small_fast_model {
                    println!("\r    Small Fast Model: {}", small_fast_model.yellow());
                }
                println!();
            } else {
                println!("\r  {} {}", "○".dimmed(), config.alias_name.dimmed());
            }
        }

        // Add reset option
        let reset_index = configs.len();
        if *selected_index == reset_index {
            println!(
                "\r> {} {}",
                "●".red().bold(),
                "Reset to default".red().bold()
            );
            println!("\r    Remove API configuration, use default Claude settings");
            println!();
        } else {
            println!("\r  {} {}", "○".dimmed(), "Reset to default".dimmed());
        }

        // Add exit option
        let exit_index = configs.len() + 1;
        if *selected_index == exit_index {
            println!("\r> {} {}", "●".yellow().bold(), "Exit".yellow().bold());
            println!("\r    Exit without making changes");
            println!();
        } else {
            println!("\r  {} {}", "○".dimmed(), "Exit".dimmed());
        }

        // Ensure output is flushed
        stdout.flush()?;

        // Handle input
        match event::read()? {
            Event::Key(KeyEvent {
                code,
                kind: KeyEventKind::Press,
                ..
            }) => match code {
                KeyCode::Up => {
                    *selected_index = selected_index.saturating_sub(1);
                }
                KeyCode::Down => {
                    if *selected_index < configs.len() + 1 {
                        *selected_index += 1;
                    }
                }
                KeyCode::Enter => {
                    return handle_selection_action(configs, *selected_index);
                }
                KeyCode::Esc => {
                    println!("Selection cancelled");
                    return Ok(());
                }
                _ => {}
            },
            Event::Key(_) => {} // Ignore key release events
            _ => {}
        }
    }
}

/// Handle simple interactive menu (fallback)
fn handle_simple_interactive_menu(
    configs: &[&Configuration],
    _storage: &ConfigStorage,
) -> Result<()> {
    println!("\n{}", "Available Configurations:".blue().bold());

    for (index, config) in configs.iter().enumerate() {
        println!(
            "{}. {} ({})",
            index + 1,
            config.alias_name.green(),
            format!(
                "{}...{}",
                &config.token[..12],
                &config.token[config.token.len() - 8..]
            )
            .dimmed()
        );
        println!("   URL: {}", config.url.cyan());
        if let Some(model) = &config.model {
            println!("   Model: {}", model.yellow());
        }
        if let Some(small_fast_model) = &config.small_fast_model {
            println!("   Small Fast Model: {}", small_fast_model.yellow());
        }
        println!();
    }

    println!("{}. {}", configs.len() + 1, "Reset to default".red());
    println!("{}. {}", configs.len() + 2, "Exit".yellow());

    print!("\nSelect configuration (1-{}): ", configs.len() + 2);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    match input.trim().parse::<usize>() {
        Ok(num) if num >= 1 && num <= configs.len() => {
            handle_selection_action(configs, num - 1)
        }
        Ok(num) if num == configs.len() + 1 => {
            // Reset to default
            println!("Using default Claude configuration");
            launch_claude_with_env(EnvironmentConfig::empty())
        }
        Ok(num) if num == configs.len() + 2 => {
            println!("Exiting...");
            Ok(())
        }
        _ => {
            println!("Invalid selection");
            Ok(())
        }
    }
}

/// Handle the actual selection and configuration switch
fn handle_selection_action(
    configs: &[&Configuration],
    selected_index: usize,
) -> Result<()> {
    if selected_index < configs.len() {
        // Switch to selected configuration
        let selected_config = configs[selected_index].clone();
        let env_config = EnvironmentConfig::from_config(&selected_config);

        println!(
            "Switched to configuration '{}'",
            selected_config.alias_name.green().bold()
        );
        println!(
            "Token: {}",
            format!(
                "{}...{}",
                &selected_config.token[..12],
                &selected_config.token[selected_config.token.len() - 8..]
            )
            .dimmed()
        );
        println!("URL: {}", selected_config.url.cyan());
        if let Some(model) = &selected_config.model {
            println!("Model: {}", model.yellow());
        }
        if let Some(small_fast_model) = &selected_config.small_fast_model {
            println!("Small Fast Model: {}", small_fast_model.yellow());
        }

        launch_claude_with_env(env_config)
    } else if selected_index == configs.len() {
        // Reset to default
        println!("Using default Claude configuration");
        launch_claude_with_env(EnvironmentConfig::empty())
    } else {
        // Exit
        println!("Exiting...");
        Ok(())
    }
}

/// Launch Claude CLI with environment variables
fn launch_claude_with_env(env_config: EnvironmentConfig) -> Result<()> {
    println!("\nWaiting 0.5 seconds before launching Claude...");
    thread::sleep(Duration::from_millis(500));

    println!("Launching Claude CLI...");
    let mut cmd = Command::new("claude");
    cmd.arg("--dangerously-skip-permissions");
    
    // Set environment variables
    for (key, value) in env_config.as_env_tuples() {
        cmd.env(&key, &value);
    }
    
    let mut child = cmd
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

/// Execute claude command with or without --dangerously-skip-permissions
///
/// # Arguments
/// * `skip_permissions` - Whether to add --dangerously-skip-permissions flag
fn execute_claude_command(skip_permissions: bool) -> Result<()> {
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