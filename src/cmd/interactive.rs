use crate::cmd::config::{ConfigStorage, Configuration, EnvironmentConfig};
use anyhow::{Context, Result};
use colored::*;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute, terminal,
};
use std::io::{self, Write};
use std::process::Command;
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
fn handle_main_menu_interactive(stdout: &mut io::Stdout, storage: &ConfigStorage) -> Result<()> {
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
                        // Clean up terminal before exit
                        let _ = execute!(stdout, terminal::LeaveAlternateScreen);
                        let _ = terminal::disable_raw_mode();

                        println!("\nExiting...");
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
fn handle_main_menu_action(selected_index: usize, storage: &ConfigStorage) -> Result<()> {
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
            let result = handle_full_interactive_menu(&mut stdout, &configs, &mut selected_index);

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

/// Handle full interactive menu with arrow key navigation and pagination
fn handle_full_interactive_menu(
    stdout: &mut io::Stdout,
    configs: &[&Configuration],
    selected_index: &mut usize,
) -> Result<()> {
    const PAGE_SIZE: usize = 9; // Maximum 9 configs per page
    
    // Calculate pagination info
    let total_pages = if configs.len() <= PAGE_SIZE {
        1
    } else {
        configs.len().div_ceil(PAGE_SIZE)
    };
    let mut current_page = 0;
    
    loop {
        // Calculate current page config range
        let start_idx = current_page * PAGE_SIZE;
        let end_idx = std::cmp::min(start_idx + PAGE_SIZE, configs.len());
        let page_configs = &configs[start_idx..end_idx];
        
        // Clear screen and redraw
        execute!(stdout, terminal::Clear(terminal::ClearType::All))?;
        execute!(stdout, crossterm::cursor::MoveTo(0, 0))?;

        // Header with pagination info
        println!("\r{}", "╔══ Select Configuration ══╗".green().bold());
        if total_pages > 1 {
            println!(
                "\r{}",
                format!("║ 第 {} 页，共 {} 页", current_page + 1, total_pages).dimmed()
            );
            println!(
                "\r{}",
                "║ ↑↓: 选择, Enter: 确认, N/PageDown: 下页, P/PageUp: 上页".dimmed()
            );
        } else {
            println!(
                "\r{}",
                "║ Use ↑↓ arrows, Enter to select, Esc to cancel".dimmed()
            );
        }
        println!("\r{}", "╚═══════════════════════════╝".green().bold());
        println!();

        // Add official option (always visible)
        let official_index = 0;
        if *selected_index == official_index {
            println!(
                "\r> {} {} {}",
                "●".red().bold(),
                "[R]".red().bold(),
                "official".red().bold()
            );
            println!("\r    Use official Claude API (no custom configuration)");
            println!();
        } else {
            println!(
                "\r  {} {} {}",
                "○".dimmed(),
                "[R]".dimmed(),
                "official".dimmed()
            );
        }

        // Draw current page configs with proper numbering
        for (page_index, config) in page_configs.iter().enumerate() {
            let actual_config_index = start_idx + page_index;
            let display_number = page_index + 1; // Numbers 1-9 for current page
            let actual_index = actual_config_index + 1; // +1 because official is at index 0
            let number_label = format!("[{display_number}]");
            
            if *selected_index == actual_index {
                println!(
                    "\r> {} {} {}",
                    "●".blue().bold(),
                    number_label.blue().bold(),
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
                println!(
                    "\r  {} {} {}",
                    "○".dimmed(),
                    number_label.dimmed(),
                    config.alias_name.dimmed()
                );
            }
        }

        // Add exit option (always visible)
        let exit_index = configs.len() + 1;
        if *selected_index == exit_index {
            println!(
                "\r> {} {} {}",
                "●".yellow().bold(),
                "[E]".yellow().bold(),
                "Exit".yellow().bold()
            );
            println!("\r    Exit without making changes");
            println!();
        } else {
            println!(
                "\r  {} {} {}",
                "○".dimmed(),
                "[E]".dimmed(),
                "Exit".dimmed()
            );
        }

        // Show pagination help if needed
        if total_pages > 1 {
            println!("\r{}", format!(
                "Page Navigation: [N]ext, [P]revious (第 {} 页，共 {} 页)",
                current_page + 1,
                total_pages
            ).dimmed());
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
                KeyCode::PageDown | KeyCode::Char('n') | KeyCode::Char('N') => {
                    if total_pages > 1 && current_page < total_pages - 1 {
                        current_page += 1;
                        // Reset selection to first item of new page
                        let new_page_start_idx = current_page * PAGE_SIZE;
                        *selected_index = new_page_start_idx + 1; // +1 because official is at index 0
                    }
                }
                KeyCode::PageUp | KeyCode::Char('p') | KeyCode::Char('P') => {
                    if total_pages > 1 && current_page > 0 {
                        current_page -= 1;
                        // Reset selection to first item of new page
                        let new_page_start_idx = current_page * PAGE_SIZE;
                        *selected_index = new_page_start_idx + 1; // +1 because official is at index 0
                    }
                }
                KeyCode::Enter => {
                    // Clean up terminal before processing selection
                    let _ = execute!(stdout, terminal::LeaveAlternateScreen);
                    let _ = terminal::disable_raw_mode();

                    return handle_selection_action(configs, *selected_index);
                }
                KeyCode::Esc => {
                    // Clean up terminal before exit
                    let _ = execute!(stdout, terminal::LeaveAlternateScreen);
                    let _ = terminal::disable_raw_mode();

                    println!("\nSelection cancelled");
                    return Ok(());
                }
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    let digit = c.to_digit(10).unwrap() as usize;
                    // Map digit to current page config
                    if digit >= 1 && digit <= page_configs.len() {
                        let actual_config_index = start_idx + (digit - 1);
                        let selection_index = actual_config_index + 1; // +1 because official is at index 0
                        
                        // Clean up terminal before processing selection
                        let _ = execute!(stdout, terminal::LeaveAlternateScreen);
                        let _ = terminal::disable_raw_mode();

                        return handle_selection_action(configs, selection_index);
                    }
                    // Invalid digit - ignore silently
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    // Clean up terminal before processing selection
                    let _ = execute!(stdout, terminal::LeaveAlternateScreen);
                    let _ = terminal::disable_raw_mode();

                    return handle_selection_action(configs, 0);
                }
                KeyCode::Char('e') | KeyCode::Char('E') => {
                    // Clean up terminal before processing selection
                    let _ = execute!(stdout, terminal::LeaveAlternateScreen);
                    let _ = terminal::disable_raw_mode();

                    return handle_selection_action(configs, configs.len() + 1);
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

    // Add official option (first)
    println!("1. {}", "official".red());
    println!("   Use official Claude API (no custom configuration)");
    println!();

    for (index, config) in configs.iter().enumerate() {
        println!(
            "{}. {} ({})",
            index + 2, // +2 because official is at position 1
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

    println!("{}. {}", configs.len() + 2, "Exit".yellow());

    print!("\nSelect configuration (1-{}): ", configs.len() + 2);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    match input.trim().parse::<usize>() {
        Ok(1) => {
            // Official option
            println!("Using official Claude configuration");
            launch_claude_with_env(EnvironmentConfig::empty())
        }
        Ok(num) if num >= 2 && num <= configs.len() + 1 => {
            handle_selection_action(configs, num - 2) // -2 because official is at position 1
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
fn handle_selection_action(configs: &[&Configuration], selected_index: usize) -> Result<()> {
    if selected_index == 0 {
        // Official option (reset to default)
        println!("\nUsing official Claude configuration");
        launch_claude_with_env(EnvironmentConfig::empty())
    } else if selected_index <= configs.len() {
        // Switch to selected configuration
        let config_index = selected_index - 1; // -1 because official is at index 0
        let selected_config = configs[config_index].clone();
        let env_config = EnvironmentConfig::from_config(&selected_config);

        println!(
            "\nSwitched to configuration '{}'",
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
    } else {
        // Exit
        println!("\nExiting...");
        Ok(())
    }
}

/// Launch Claude CLI with environment variables and exec to replace current process
fn launch_claude_with_env(env_config: EnvironmentConfig) -> Result<()> {
    println!("\nWaiting 0.5 seconds before launching Claude...");
    thread::sleep(Duration::from_millis(500));

    println!("Launching Claude CLI...");

    // Set environment variables for current process
    for (key, value) in env_config.as_env_tuples() {
        unsafe {
            std::env::set_var(&key, &value);
        }
    }

    // On Unix systems, use exec to replace current process
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let error = Command::new("claude")
            .arg("--dangerously-skip-permissions")
            .exec();
        // exec never returns on success, so if we get here, it failed
        anyhow::bail!("Failed to exec claude: {}", error);
    }

    // On non-Unix systems, fallback to spawn and wait
    #[cfg(not(unix))]
    {
        use std::process::Stdio;
        let mut child = Command::new("claude")
            .arg("--dangerously-skip-permissions")
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .context(
                "Failed to launch Claude CLI. Make sure 'claude' command is available in PATH",
            )?;

        let status = child.wait()?;

        if !status.success() {
            anyhow::bail!("Claude CLI exited with error status: {}", status);
        }
    }

    Ok(())
}

/// Execute claude command with or without --dangerously-skip-permissions using exec
///
/// # Arguments
/// * `skip_permissions` - Whether to add --dangerously-skip-permissions flag
fn execute_claude_command(skip_permissions: bool) -> Result<()> {
    println!("Launching Claude CLI...");

    // On Unix systems, use exec to replace current process
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let mut command = Command::new("claude");
        if skip_permissions {
            command.arg("--dangerously-skip-permissions");
        }

        let error = command.exec();
        // exec never returns on success, so if we get here, it failed
        anyhow::bail!("Failed to exec claude: {}", error);
    }

    // On non-Unix systems, fallback to spawn and wait
    #[cfg(not(unix))]
    {
        use std::process::Stdio;
        let mut command = Command::new("claude");
        if skip_permissions {
            command.arg("--dangerously-skip-permissions");
        }

        command
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        let mut child = command.spawn().context(
            "Failed to launch Claude CLI. Make sure 'claude' command is available in PATH",
        )?;

        let status = child
            .wait()
            .context("Failed to wait for Claude CLI process")?;

        if !status.success() {
            anyhow::bail!("Claude CLI exited with error status: {}", status);
        }
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
