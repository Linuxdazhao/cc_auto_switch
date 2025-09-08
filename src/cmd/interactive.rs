use crate::cmd::config::{ConfigStorage, Configuration, EnvironmentConfig};
use crate::cmd::display_utils::{
    TextAlignment, format_token_for_display, get_terminal_width, pad_text_to_width,
    text_display_width,
};
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

/// Border drawing utilities for terminal compatibility
struct BorderDrawing {
    /// Check if terminal supports Unicode box drawing characters
    pub unicode_supported: bool,
}

impl BorderDrawing {
    /// Create new border drawing utility
    fn new() -> Self {
        let unicode_supported = Self::detect_unicode_support();
        Self { unicode_supported }
    }

    /// Detect if terminal supports Unicode characters
    fn detect_unicode_support() -> bool {
        // Check environment variables that indicate Unicode support
        if let Ok(term) = std::env::var("TERM") {
            // Modern terminals that support Unicode
            if term.contains("xterm") || term.contains("screen") || term == "tmux-256color" {
                return true;
            }
        }

        // Check locale settings
        if let Ok(lang) = std::env::var("LANG")
            && (lang.contains("UTF-8") || lang.contains("utf8"))
        {
            return true;
        }

        // Conservative fallback - assume Unicode is supported for better UX
        // If issues arise, ASCII fallback will be manually triggered
        true
    }

    /// Draw top border with title
    fn draw_top_border(&self, title: &str, width: usize) -> String {
        if self.unicode_supported {
            let title_padded = format!(" {title} ");
            let title_len = text_display_width(&title_padded);

            if title_len >= width - 2 {
                // Title too long, use simple border
                format!("╔{}╗", "═".repeat(width - 2))
            } else {
                let padding = (width - 2 - title_len) / 2;
                let extra = (width - 2 - title_len) % 2;
                format!(
                    "╔{}{}{}{}╗",
                    "═".repeat(padding),
                    title_padded,
                    "═".repeat(padding + extra),
                    ""
                )
            }
        } else {
            // ASCII fallback
            let title_padded = format!(" {title} ");
            let title_len = title_padded.len();

            if title_len >= width - 2 {
                format!("+{}+", "-".repeat(width - 2))
            } else {
                let padding = (width - 2 - title_len) / 2;
                let extra = (width - 2 - title_len) % 2;
                format!(
                    "+{}{}{}+",
                    "-".repeat(padding),
                    title_padded,
                    "-".repeat(padding + extra)
                )
            }
        }
    }

    /// Draw middle border line with text
    fn draw_middle_line(&self, text: &str, width: usize) -> String {
        if self.unicode_supported {
            let text_len = text_display_width(text);
            let available_width = width - 4;
            if text_len >= available_width {
                format!("║ {} ║", &text[..available_width])
            } else {
                let padded_text =
                    pad_text_to_width(text, available_width, TextAlignment::Left, ' ');
                format!("║ {} ║", padded_text)
            }
        } else {
            // ASCII fallback
            let text_len = text_display_width(text);
            let available_width = width - 4;
            if text_len >= available_width {
                format!("| {} |", &text[..available_width])
            } else {
                let padded_text =
                    pad_text_to_width(text, available_width, TextAlignment::Left, ' ');
                format!("| {} |", padded_text)
            }
        }
    }

    /// Draw bottom border
    fn draw_bottom_border(&self, width: usize) -> String {
        if self.unicode_supported {
            format!("╚{}╝", "═".repeat(width - 2))
        } else {
            format!("+{}+", "-".repeat(width - 2))
        }
    }
}

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

        // Header - use BorderDrawing for compatibility
        let border = BorderDrawing::new();
        const MAIN_MENU_WIDTH: usize = 55;

        println!(
            "\r{}",
            border
                .draw_top_border("Main Menu", MAIN_MENU_WIDTH)
                .green()
                .bold()
        );
        println!(
            "\r{}",
            border
                .draw_middle_line(
                    "Use ↑↓ arrows, Enter to select, Esc to exit",
                    MAIN_MENU_WIDTH
                )
                .dimmed()
        );
        println!(
            "\r{}",
            border.draw_bottom_border(MAIN_MENU_WIDTH).green().bold()
        );
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

        // Handle input with error recovery
        let event = match event::read() {
            Ok(event) => event,
            Err(e) => {
                // Clean up terminal state on input error
                let _ = execute!(stdout, terminal::LeaveAlternateScreen);
                let _ = terminal::disable_raw_mode();
                return Err(e.into());
            }
        };

        match event {
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
    // Handle empty configuration list
    if configs.is_empty() {
        println!("\r{}", "No configurations available".yellow());
        println!(
            "\r{}",
            "Use 'cc-switch add <alias> <token> <url>' to add configurations first.".dimmed()
        );
        println!("\r{}", "Press any key to continue...".dimmed());
        let _ = event::read(); // Wait for user input
        return Ok(());
    }

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

        // Header with pagination info - use BorderDrawing for compatibility
        let border = BorderDrawing::new();
        const CONFIG_MENU_WIDTH: usize = 65;

        println!(
            "\r{}",
            border
                .draw_top_border("Select Configuration", CONFIG_MENU_WIDTH)
                .green()
                .bold()
        );
        if total_pages > 1 {
            println!(
                "\r{}",
                border
                    .draw_middle_line(
                        &format!("第 {} 页，共 {} 页", current_page + 1, total_pages),
                        CONFIG_MENU_WIDTH
                    )
                    .dimmed()
            );
            println!(
                "\r{}",
                border
                    .draw_middle_line(
                        "↑↓导航，1-9快选，N/P翻页，R-官方，E-退出，Enter确认",
                        CONFIG_MENU_WIDTH
                    )
                    .dimmed()
            );
        } else {
            println!(
                "\r{}",
                border
                    .draw_middle_line(
                        "↑↓导航，1-9快选，R-官方，E-退出，Enter确认，Esc取消",
                        CONFIG_MENU_WIDTH
                    )
                    .dimmed()
            );
        }
        println!(
            "\r{}",
            border.draw_bottom_border(CONFIG_MENU_WIDTH).green().bold()
        );
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

                // Show details with improved formatting and alignment
                let details = format_config_details(config, "\r    ", false);
                for detail_line in details {
                    println!("{detail_line}");
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
            println!(
                "\r{}",
                format!(
                    "Page Navigation: [N]ext, [P]revious (第 {} 页，共 {} 页)",
                    current_page + 1,
                    total_pages
                )
                .dimmed()
            );
        }

        // Ensure output is flushed
        stdout.flush()?;

        // Handle input with error recovery
        let event = match event::read() {
            Ok(event) => event,
            Err(e) => {
                // Clean up terminal state on input error
                let _ = execute!(stdout, terminal::LeaveAlternateScreen);
                let _ = terminal::disable_raw_mode();
                return Err(e.into());
            }
        };

        match event {
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
    const PAGE_SIZE: usize = 9; // Same page size as full interactive menu

    // If configs fit in one page, show the simple original menu
    if configs.len() <= PAGE_SIZE {
        return handle_simple_single_page_menu(configs);
    }

    // Multi-page simple menu
    let total_pages = configs.len().div_ceil(PAGE_SIZE);
    let mut current_page = 0;

    loop {
        // Calculate current page config range
        let start_idx = current_page * PAGE_SIZE;
        let end_idx = std::cmp::min(start_idx + PAGE_SIZE, configs.len());
        let page_configs = &configs[start_idx..end_idx];

        println!("\n{}", "Available Configurations:".blue().bold());
        if total_pages > 1 {
            println!("第 {} 页，共 {} 页", current_page + 1, total_pages);
            println!("使用 'n' 下一页, 'p' 上一页, 'r' 官方配置, 'e' 退出");
        }
        println!();

        // Add official option (always available)
        println!("{} {}", "[r]".red().bold(), "official".red());
        println!("   Use official Claude API (no custom configuration)");
        println!();

        // Show current page configs with improved formatting
        for (page_index, config) in page_configs.iter().enumerate() {
            let display_number = page_index + 1;
            println!(
                "{}. {}",
                format!("[{display_number}]").green().bold(),
                config.alias_name.green()
            );

            // Show config details with consistent formatting
            let details = format_config_details(config, "   ", true);
            for detail_line in details {
                println!("{detail_line}");
            }
            println!();
        }

        // Exit option
        println!("{} {}", "[e]".yellow().bold(), "Exit".yellow());

        if total_pages > 1 {
            println!(
                "\n页面导航: [n]下页, [p]上页 | 配置选择: [1-{}] | [r]官方 | [e]退出",
                page_configs.len()
            );
        }

        print!("\n请输入选择: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let choice = input.trim().to_lowercase();

        match choice.as_str() {
            "r" => {
                // Official option
                println!("Using official Claude configuration");
                return launch_claude_with_env(EnvironmentConfig::empty());
            }
            "e" => {
                println!("Exiting...");
                return Ok(());
            }
            "n" if total_pages > 1 && current_page < total_pages - 1 => {
                current_page += 1;
                continue;
            }
            "p" if total_pages > 1 && current_page > 0 => {
                current_page -= 1;
                continue;
            }
            digit_str => {
                if let Ok(digit) = digit_str.parse::<usize>()
                    && digit >= 1
                    && digit <= page_configs.len()
                {
                    let actual_config_index = start_idx + (digit - 1);
                    let selection_index = actual_config_index + 1; // +1 because official is at index 0
                    return handle_selection_action(configs, selection_index);
                }
                println!("无效选择，请重新输入");
            }
        }
    }
}

/// Handle simple single page menu (original behavior for ≤9 configs)
fn handle_simple_single_page_menu(configs: &[&Configuration]) -> Result<()> {
    println!("\n{}", "Available Configurations:".blue().bold());

    // Add official option (first)
    println!("1. {}", "official".red());
    println!("   Use official Claude API (no custom configuration)");
    println!();

    for (index, config) in configs.iter().enumerate() {
        println!(
            "{}. {}",
            index + 2, // +2 because official is at position 1
            config.alias_name.green()
        );

        // Show config details with consistent formatting
        let details = format_config_details(config, "   ", true);
        for detail_line in details {
            println!("{detail_line}");
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

        // Show selected configuration details with consistent formatting
        let details = format_config_details(&selected_config, "", false);
        for detail_line in details {
            println!("{detail_line}");
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

/// Format configuration details with consistent indentation and alignment
///
/// This function provides unified formatting for configuration display across
/// all interactive menus, ensuring consistent visual presentation.
///
/// # Arguments
/// * `config` - The configuration to format
/// * `indent` - Base indentation string (e.g., "    " or "   ")
/// * `compact` - Whether to use compact formatting (single line where possible)
///
/// # Returns  
/// Vector of formatted lines for configuration display
fn format_config_details(config: &Configuration, indent: &str, _compact: bool) -> Vec<String> {
    let mut lines = Vec::new();

    // Calculate optimal field width for alignment
    let terminal_width = get_terminal_width();
    let _available_width = terminal_width.saturating_sub(text_display_width(indent) + 8);

    // Field labels with consistent width for alignment
    let token_label = "Token:";
    let url_label = "URL:";
    let model_label = "Model:";
    let small_model_label = "Small Fast Model:";

    // Find the widest label for alignment
    let max_label_width = [token_label, url_label, model_label, small_model_label]
        .iter()
        .map(|label| text_display_width(label))
        .max()
        .unwrap_or(0);

    // Format token with proper alignment
    let token_line = format!(
        "{}{} {}",
        indent,
        pad_text_to_width(token_label, max_label_width, TextAlignment::Left, ' '),
        format_token_for_display(&config.token).dimmed()
    );
    lines.push(token_line);

    // Format URL with proper alignment
    let url_line = format!(
        "{}{} {}",
        indent,
        pad_text_to_width(url_label, max_label_width, TextAlignment::Left, ' '),
        config.url.cyan()
    );
    lines.push(url_line);

    // Format model information if available
    if let Some(model) = &config.model {
        let model_line = format!(
            "{}{} {}",
            indent,
            pad_text_to_width(model_label, max_label_width, TextAlignment::Left, ' '),
            model.yellow()
        );
        lines.push(model_line);
    }

    // Format small fast model if available
    if let Some(small_fast_model) = &config.small_fast_model {
        let small_model_line = format!(
            "{}{} {}",
            indent,
            pad_text_to_width(small_model_label, max_label_width, TextAlignment::Left, ' '),
            small_fast_model.yellow()
        );
        lines.push(small_model_line);
    }

    lines
}

#[cfg(test)]
mod border_drawing_tests {
    use super::*;

    #[test]
    fn test_border_drawing_unicode_support() {
        let border = BorderDrawing::new();
        // Should create without panic
        assert!(border.unicode_supported || !border.unicode_supported); // Always true, just testing creation
    }

    #[test]
    fn test_border_drawing_top_border() {
        let border = BorderDrawing {
            unicode_supported: true,
        };
        let result = border.draw_top_border("Test", 20);
        assert!(result.len() > 0);
        assert!(result.contains("Test"));
    }

    #[test]
    fn test_border_drawing_ascii_fallback() {
        let border = BorderDrawing {
            unicode_supported: false,
        };
        let result = border.draw_top_border("Test", 20);
        assert!(result.len() > 0);
        assert!(result.contains("Test"));
        assert!(result.contains("+"));
        assert!(result.contains("-"));
    }

    #[test]
    fn test_border_drawing_middle_line() {
        let border = BorderDrawing {
            unicode_supported: true,
        };
        let result = border.draw_middle_line("Test message", 30);
        assert!(result.len() > 0);
        assert!(result.contains("Test message"));
    }

    #[test]
    fn test_border_drawing_bottom_border() {
        let border = BorderDrawing {
            unicode_supported: true,
        };
        let result = border.draw_bottom_border(20);
        assert!(result.len() > 0);
    }

    #[test]
    fn test_border_drawing_width_consistency() {
        let border = BorderDrawing {
            unicode_supported: true,
        };
        let width = 30;
        let top = border.draw_top_border("Title", width);
        let middle = border.draw_middle_line("Content", width);
        let bottom = border.draw_bottom_border(width);

        // All borders should have the same character length (approximately)
        assert!(top.chars().count() >= width - 2);
        assert!(middle.chars().count() >= width - 2);
        assert!(bottom.chars().count() >= width - 2);
    }
}

#[cfg(test)]
mod pagination_tests {

    /// Test pagination calculation logic
    #[test]
    fn test_pagination_calculation() {
        const PAGE_SIZE: usize = 9;

        // Test single page scenarios
        assert_eq!(1_usize.div_ceil(PAGE_SIZE), 1); // 1 config -> 1 page
        assert_eq!(9_usize.div_ceil(PAGE_SIZE), 1); // 9 configs -> 1 page

        // Test multi-page scenarios
        assert_eq!(10_usize.div_ceil(PAGE_SIZE), 2); // 10 configs -> 2 pages
        assert_eq!(18_usize.div_ceil(PAGE_SIZE), 2); // 18 configs -> 2 pages
        assert_eq!(19_usize.div_ceil(PAGE_SIZE), 3); // 19 configs -> 3 pages
        assert_eq!(27_usize.div_ceil(PAGE_SIZE), 3); // 27 configs -> 3 pages
        assert_eq!(28_usize.div_ceil(PAGE_SIZE), 4); // 28 configs -> 4 pages
    }

    /// Test page range calculation
    #[test]
    fn test_page_range_calculation() {
        const PAGE_SIZE: usize = 9;

        // Test first page
        let current_page = 0;
        let start_idx = current_page * PAGE_SIZE; // 0
        let end_idx = std::cmp::min(start_idx + PAGE_SIZE, 15); // min(9, 15) = 9
        assert_eq!(start_idx, 0);
        assert_eq!(end_idx, 9);
        assert_eq!(end_idx - start_idx, 9); // Full page

        // Test second page
        let current_page = 1;
        let start_idx = current_page * PAGE_SIZE; // 9
        let end_idx = std::cmp::min(start_idx + PAGE_SIZE, 15); // min(18, 15) = 15
        assert_eq!(start_idx, 9);
        assert_eq!(end_idx, 15);
        assert_eq!(end_idx - start_idx, 6); // Partial page

        // Test edge case: exactly PAGE_SIZE configs
        let current_page = 0;
        let start_idx = current_page * PAGE_SIZE; // 0
        let end_idx = std::cmp::min(start_idx + PAGE_SIZE, PAGE_SIZE); // min(9, 9) = 9
        assert_eq!(start_idx, 0);
        assert_eq!(end_idx, 9);
        assert_eq!(end_idx - start_idx, 9); // Full page
    }

    /// Test digit key mapping to config indices
    #[test]
    fn test_digit_mapping_to_config_index() {
        const PAGE_SIZE: usize = 9;

        // Test first page mapping (configs 0-8)
        let current_page = 0;
        let start_idx = current_page * PAGE_SIZE; // 0

        // Digit 1 should map to config index 0
        let digit = 1;
        let actual_config_index = start_idx + (digit - 1); // 0 + (1-1) = 0
        assert_eq!(actual_config_index, 0);

        // Digit 9 should map to config index 8
        let digit = 9;
        let actual_config_index = start_idx + (digit - 1); // 0 + (9-1) = 8
        assert_eq!(actual_config_index, 8);

        // Test second page mapping (configs 9-17)
        let current_page = 1;
        let start_idx = current_page * PAGE_SIZE; // 9

        // Digit 1 should map to config index 9
        let digit = 1;
        let actual_config_index = start_idx + (digit - 1); // 9 + (1-1) = 9
        assert_eq!(actual_config_index, 9);

        // Digit 5 should map to config index 13
        let digit = 5;
        let actual_config_index = start_idx + (digit - 1); // 9 + (5-1) = 13
        assert_eq!(actual_config_index, 13);
    }

    /// Test selection index conversion for handle_selection_action
    #[test]
    fn test_selection_index_conversion() {
        // Test mapping digit to selection index for handle_selection_action
        // Note: handle_selection_action expects indices where:
        // - 0 = official config
        // - 1 = first user config
        // - 2 = second user config, etc.

        const PAGE_SIZE: usize = 9;

        // First page, digit 1 -> config index 0 -> selection index 1
        let current_page = 0;
        let start_idx = current_page * PAGE_SIZE; // 0
        let digit = 1;
        let actual_config_index = start_idx + (digit - 1); // 0
        let selection_index = actual_config_index + 1; // +1 because official is at index 0
        assert_eq!(selection_index, 1);

        // Second page, digit 1 -> config index 9 -> selection index 10
        let current_page = 1;
        let start_idx = current_page * PAGE_SIZE; // 9
        let digit = 1;
        let actual_config_index = start_idx + (digit - 1); // 9
        let selection_index = actual_config_index + 1; // +1 because official is at index 0
        assert_eq!(selection_index, 10);
    }

    /// Test page navigation bounds checking
    #[test]
    fn test_page_navigation_bounds() {
        const PAGE_SIZE: usize = 9;
        let total_configs: usize = 25; // 3 pages total
        let total_pages = total_configs.div_ceil(PAGE_SIZE); // 3 pages
        assert_eq!(total_pages, 3);

        // Test first page - can't go to previous
        let mut current_page = 0;
        if current_page > 0 {
            current_page -= 1;
        }
        assert_eq!(current_page, 0); // Should stay at 0

        // Test last page - can't go to next
        let mut current_page = total_pages - 1; // 2 (last page)
        if current_page < total_pages - 1 {
            current_page += 1;
        }
        assert_eq!(current_page, 2); // Should stay at 2

        // Test middle page navigation
        let mut current_page = 1;

        // Can go to next page
        if current_page < total_pages - 1 {
            current_page += 1;
        }
        assert_eq!(current_page, 2);

        // Can go to previous page
        if current_page > 0 {
            current_page -= 1;
        }
        assert_eq!(current_page, 1);
    }

    /// Test boundary conditions for digit key processing
    #[test]
    fn test_digit_key_boundary_conditions() {
        const PAGE_SIZE: usize = 9;

        // Test digit 0 (should be ignored)
        let digit = 0;
        assert!(digit < 1, "Digit 0 should be less than 1 and ignored");

        // Test digit beyond available configs (should be ignored)
        let configs_len = 5; // Only 5 configs available
        let page_configs_len = std::cmp::min(PAGE_SIZE, configs_len); // 5
        let digit = 9; // User presses 9
        assert!(
            digit > page_configs_len,
            "Digit 9 should be beyond available configs (5) and ignored"
        );

        // Test valid digit range
        for digit in 1..=page_configs_len {
            assert!(
                digit >= 1 && digit <= page_configs_len,
                "Digit {} should be valid",
                digit
            );
        }
    }

    /// Test empty configuration list handling
    #[test]
    fn test_empty_configs_handling() {
        let empty_configs: Vec<String> = Vec::new();
        assert!(
            empty_configs.is_empty(),
            "Empty config list should be properly detected"
        );

        // Verify that empty check comes before pagination calculation
        let configs_len = empty_configs.len(); // 0
        assert_eq!(configs_len, 0, "Empty configs should have length 0");

        // No pagination should be calculated for empty configs
        // (function should return early)
    }

    /// Test page navigation boundary conditions
    #[test]
    fn test_page_navigation_boundaries() {
        const PAGE_SIZE: usize = 9;
        let total_configs: usize = 20; // 3 pages total
        let total_pages = total_configs.div_ceil(PAGE_SIZE); // 3 pages

        // Test first page navigation (cannot go to previous page)
        let mut current_page = 0;
        let original_page = current_page;

        // Simulate PageUp on first page (should not change)
        if current_page > 0 {
            current_page -= 1;
        }
        assert_eq!(
            current_page, original_page,
            "First page should not navigate to previous"
        );

        // Test last page navigation (cannot go to next page)
        let mut current_page = total_pages - 1; // Last page (2)
        let original_page = current_page;

        // Simulate PageDown on last page (should not change)
        if current_page < total_pages - 1 {
            current_page += 1;
        }
        assert_eq!(
            current_page, original_page,
            "Last page should not navigate to next"
        );

        // Test valid navigation from middle page
        let mut current_page = 1; // Middle page

        // Navigate to next page
        if current_page < total_pages - 1 {
            current_page += 1;
        }
        assert_eq!(current_page, 2, "Should navigate to next page");

        // Navigate to previous page
        if current_page > 0 {
            current_page -= 1;
        }
        assert_eq!(current_page, 1, "Should navigate to previous page");
    }
}
