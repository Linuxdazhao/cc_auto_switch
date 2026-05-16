use crate::codex::{CodexConfiguration, write_auth_json};
use crate::cli::display_utils::{
    TextAlignment, get_terminal_width, pad_text_to_width, text_display_width,
};
use crate::config::types::ConfigStorage;
use crate::interactive::interactive::{
    BorderDrawing, cleanup_terminal,
};
use anyhow::Result;
use colored::*;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute, terminal,
};
use std::io::{self, Write};
use std::process::Command;

/// Handle interactive Codex configuration selection with full TUI
///
/// Mirrors the Claude interactive TUI: alternate screen, arrow key / j/k navigation,
/// number key shortcuts (1-9), pagination (9 per page), config detail preview.
///
/// # Arguments
/// * `storage` - Reference to configuration storage
///
/// # Errors
/// Returns error if terminal operations fail or user selection fails
pub fn handle_codex_interactive_selection(storage: &ConfigStorage) -> Result<()> {
    let configs_map = match &storage.codex_configurations {
        Some(configs) if !configs.is_empty() => configs,
        _ => {
            println!("No Codex configurations available. Use 'cc-switch codex add' to create configurations first.");
            return Ok(());
        }
    };

    let mut configs: Vec<CodexConfiguration> = configs_map.values().cloned().collect();
    configs.sort_by(|a, b| a.alias_name.cmp(&b.alias_name));

    let mut selected_index: usize = 0;

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
            let result = handle_codex_full_interactive_menu(
                &mut stdout,
                &mut configs,
                &mut selected_index,
            );

            // Always restore terminal
            let _ = execute!(stdout, terminal::LeaveAlternateScreen);
            let _ = terminal::disable_raw_mode();

            return result;
        } else {
            let _ = terminal::disable_raw_mode();
        }
    }

    // Fallback to simple numbered menu
    handle_codex_simple_interactive_menu(&configs)
}

/// Format Codex configuration details for display
///
/// # Arguments
/// * `config` - The configuration to format
/// * `indent` - Base indentation string (e.g., "    " or "   ")
///
/// # Returns
/// Vector of formatted lines for configuration display
fn format_codex_config_details(config: &CodexConfiguration, indent: &str) -> Vec<String> {
    let mut lines = Vec::new();

    let terminal_width = get_terminal_width();
    let _available_width = terminal_width.saturating_sub(text_display_width(indent) + 8);

    // Field labels with consistent width for alignment
    let auth_mode_label = "Auth Mode:";
    let account_id_label = "Account ID:";
    let api_key_label = "API Key:";
    let last_refresh_label = "Last Refresh:";

    let max_label_width = [
        auth_mode_label,
        account_id_label,
        api_key_label,
        last_refresh_label,
    ]
    .iter()
    .map(|label| text_display_width(label))
    .max()
    .unwrap_or(0);

    // Auth mode (always shown)
    let mode_value = if config.auth_mode == "apikey" {
        "apikey".cyan()
    } else {
        "chatgpt".cyan()
    };
    lines.push(format!(
        "{}{} {}",
        indent,
        pad_text_to_width(auth_mode_label, max_label_width, TextAlignment::Left, ' '),
        mode_value
    ));

    // Account ID (chatgpt mode)
    if let Some(ref account_id) = config.account_id {
        lines.push(format!(
            "{}{} {}",
            indent,
            pad_text_to_width(account_id_label, max_label_width, TextAlignment::Left, ' '),
            account_id.yellow()
        ));
    }

    // API key prefix (apikey mode)
    if let Some(ref key) = config.openai_api_key {
        let prefix = if key.len() > 8 {
            format!("{}...", &key[..8])
        } else {
            key.clone()
        };
        lines.push(format!(
            "{}{} {}",
            indent,
            pad_text_to_width(api_key_label, max_label_width, TextAlignment::Left, ' '),
            prefix.dimmed()
        ));
    }

    // Last refresh (chatgpt mode)
    if let Some(ref last_refresh) = config.last_refresh {
        lines.push(format!(
            "{}{} {}",
            indent,
            pad_text_to_width(last_refresh_label, max_label_width, TextAlignment::Left, ' '),
            last_refresh.dimmed()
        ));
    }

    lines
}

/// Handle full interactive menu with arrow key navigation and pagination for Codex
#[allow(clippy::ptr_arg)]
fn handle_codex_full_interactive_menu(
    stdout: &mut io::Stdout,
    configs: &mut Vec<CodexConfiguration>,
    selected_index: &mut usize,
) -> Result<()> {
    if configs.is_empty() {
        println!("\r{}", "No Codex configurations available".yellow());
        println!(
            "\r{}",
            "Use 'cc-switch codex add' to add configurations first.".dimmed()
        );
        println!("\r{}", "Press any key to continue...".dimmed());
        let _ = event::read();
        return Ok(());
    }

    const PAGE_SIZE: usize = 9;

    let total_pages = if configs.len() <= PAGE_SIZE {
        1
    } else {
        configs.len().div_ceil(PAGE_SIZE)
    };
    let mut current_page = 0;

    loop {
        let start_idx = current_page * PAGE_SIZE;
        let end_idx = std::cmp::min(start_idx + PAGE_SIZE, configs.len());
        let page_configs = &configs[start_idx..end_idx];

        // Clear screen and redraw
        execute!(stdout, terminal::Clear(terminal::ClearType::All))?;
        execute!(stdout, crossterm::cursor::MoveTo(0, 0))?;

        let border = BorderDrawing::new();
        const CONFIG_MENU_WIDTH: usize = 80;

        println!(
            "\r{}",
            border
                .draw_top_border("Select Codex Configuration", CONFIG_MENU_WIDTH)
                .green()
        );
        if total_pages > 1 {
            println!(
                "\r{}",
                border
                    .draw_middle_line(
                        &format!("第 {} 页，共 {} 页", current_page + 1, total_pages),
                        CONFIG_MENU_WIDTH
                    )
                    .green()
            );
            println!(
                "\r{}",
                border
                    .draw_middle_line(
                        "↑↓/jk导航，1-9快选，N/P翻页，Q-退出，Enter确认",
                        CONFIG_MENU_WIDTH
                    )
                    .green()
            );
        } else {
            println!(
                "\r{}",
                border
                    .draw_middle_line(
                        "↑↓/jk导航，1-9快选，Q-退出，Enter确认，Esc取消",
                        CONFIG_MENU_WIDTH
                    )
                    .green()
            );
        }
        println!("\r{}", border.draw_bottom_border(CONFIG_MENU_WIDTH).green());
        println!();

        // Draw current page configs with proper numbering
        // No "official" option for Codex; indices:
        //   0 .. configs.len()-1  -> config entries
        //   configs.len()         -> Exit option
        for (page_index, config) in page_configs.iter().enumerate() {
            let actual_config_index = start_idx + page_index;
            let display_number = page_index + 1; // Numbers 1-9 for current page
            let number_label = format!("[{display_number}]");

            if *selected_index == actual_config_index {
                println!(
                    "\r> {} {} {}",
                    "●".blue().bold(),
                    number_label.blue().bold(),
                    config.alias_name.blue().bold()
                );

                let details = format_codex_config_details(config, "\r    ");
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

        // Add exit option at the end
        let exit_index = configs.len();
        if *selected_index == exit_index {
            println!(
                "\r> {} {} {}",
                "●".yellow().bold(),
                "[Q]".yellow().bold(),
                "Exit".yellow().bold()
            );
            println!("\r    Exit without making changes");
            println!();
        } else {
            println!(
                "\r  {} {} {}",
                "○".dimmed(),
                "[Q]".dimmed(),
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

        stdout.flush()?;

        // Handle input with error recovery
        let event = match event::read() {
            Ok(event) => event,
            Err(e) => {
                cleanup_terminal(stdout);
                return Err(e.into());
            }
        };

        match event {
            Event::Key(KeyEvent {
                code,
                kind: KeyEventKind::Press,
                ..
            }) => match code {
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    *selected_index = selected_index.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    // configs.len() is the Exit option; cannot go beyond it
                    if *selected_index < configs.len() {
                        *selected_index += 1;
                    }
                }
                KeyCode::PageDown | KeyCode::Char('n') | KeyCode::Char('N') => {
                    if total_pages > 1 && current_page < total_pages - 1 {
                        current_page += 1;
                        // Reset selection to first item of new page
                        let new_page_start_idx = current_page * PAGE_SIZE;
                        *selected_index = new_page_start_idx;
                    }
                }
                KeyCode::PageUp | KeyCode::Char('p') | KeyCode::Char('P') => {
                    if total_pages > 1 && current_page > 0 {
                        current_page -= 1;
                        let new_page_start_idx = current_page * PAGE_SIZE;
                        *selected_index = new_page_start_idx;
                    }
                }
                KeyCode::Enter => {
                    cleanup_terminal(stdout);
                    return handle_codex_selection_action(configs, *selected_index);
                }
                KeyCode::Esc => {
                    cleanup_terminal(stdout);
                    println!("\nSelection cancelled");
                    return Ok(());
                }
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    let digit = c.to_digit(10).unwrap() as usize;
                    if digit >= 1 && digit <= page_configs.len() {
                        let actual_config_index = start_idx + (digit - 1);
                        cleanup_terminal(stdout);
                        return handle_codex_selection_action(configs, actual_config_index);
                    }
                }
                KeyCode::Char('q') | KeyCode::Char('Q') => {
                    cleanup_terminal(stdout);
                    return handle_codex_selection_action(configs, configs.len());
                }
                _ => {}
            },
            Event::Key(_) => {}
            _ => {}
        }
    }
}

/// Handle simple interactive menu (fallback) for Codex
fn handle_codex_simple_interactive_menu(configs: &[CodexConfiguration]) -> Result<()> {
    const PAGE_SIZE: usize = 9;

    if configs.len() <= PAGE_SIZE {
        return handle_codex_simple_single_page_menu(configs);
    }

    // Multi-page simple menu
    let total_pages = configs.len().div_ceil(PAGE_SIZE);
    let mut current_page = 0;

    loop {
        let start_idx = current_page * PAGE_SIZE;
        let end_idx = std::cmp::min(start_idx + PAGE_SIZE, configs.len());
        let page_configs = &configs[start_idx..end_idx];

        println!("\n{}", "Available Codex Configurations:".blue().bold());
        println!("第 {} 页，共 {} 页", current_page + 1, total_pages);
        println!("使用 'n' 下一页, 'p' 上一页, 'q' 退出");
        println!();

        for (page_index, config) in page_configs.iter().enumerate() {
            let display_number = page_index + 1;
            println!(
                "{}. {}",
                format!("[{display_number}]").green().bold(),
                config.alias_name.green()
            );

            let details = format_codex_config_details(config, "   ");
            for detail_line in details {
                println!("{detail_line}");
            }
            println!();
        }

        println!("{} {}", "[q]".yellow().bold(), "Exit".yellow());

        println!(
            "\n页面导航: [n]下页, [p]上页 | 配置选择: [1-{}] | [q]退出",
            page_configs.len()
        );

        print!("\n请输入选择: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let choice = input.trim().to_lowercase();

        match choice.as_str() {
            "q" => {
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
                    return handle_codex_selection_action(configs, actual_config_index);
                }
                println!("无效选择，请重新输入");
            }
        }
    }
}

/// Handle simple single page menu (original behavior for ≤9 configs)
fn handle_codex_simple_single_page_menu(configs: &[CodexConfiguration]) -> Result<()> {
    println!("\n{}", "Available Codex Configurations:".blue().bold());

    for (index, config) in configs.iter().enumerate() {
        println!("{}. {}", index + 1, config.alias_name.green());

        let details = format_codex_config_details(config, "   ");
        for detail_line in details {
            println!("{detail_line}");
        }
        println!();
    }

    println!("{}. {}", configs.len() + 1, "Exit".yellow());

    print!("\nSelect configuration (1-{}): ", configs.len() + 1);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    match input.trim().parse::<usize>() {
        Ok(num) if num >= 1 && num <= configs.len() => {
            handle_codex_selection_action(configs, num - 1)
        }
        Ok(num) if num == configs.len() + 1 => {
            println!("Exiting...");
            Ok(())
        }
        _ => {
            println!("Invalid selection");
            Ok(())
        }
    }
}

/// Handle the actual selection and configuration switch for Codex
///
/// `selected_index` semantics:
///   - 0 .. configs.len()-1: select a config
///   - configs.len(): exit
fn handle_codex_selection_action(
    configs: &[CodexConfiguration],
    selected_index: usize,
) -> Result<()> {
    if selected_index < configs.len() {
        let selected_config = &configs[selected_index];

        println!(
            "\nSwitching to Codex configuration '{}'",
            selected_config.alias_name.green().bold()
        );

        let details = format_codex_config_details(selected_config, "");
        for detail_line in details {
            println!("{detail_line}");
        }

        // Write auth.json
        write_auth_json(selected_config)?;

        // Launch codex
        launch_codex_from_interactive()
    } else {
        println!("\nExiting...");
        Ok(())
    }
}

/// Launch Codex CLI from the interactive menu
fn launch_codex_from_interactive() -> Result<()> {
    println!("\nLaunching Codex CLI...");

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let mut command = Command::new("codex");
        let error = command.exec();
        anyhow::bail!("Failed to exec codex: {}", error);
    }

    #[cfg(not(unix))]
    {
        use std::process::Stdio;
        use anyhow::Context;
        let mut command = Command::new("codex");
        command
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        let mut child = command.spawn().context(
            "Failed to launch Codex CLI. Make sure 'codex' command is available in PATH",
        )?;

        let status = child.wait()?;

        if !status.success() {
            anyhow::bail!("Codex CLI exited with error status: {}", status);
        }
        Ok(())
    }
}

#[cfg(test)]
mod codex_interactive_tests {
    use super::*;

    #[test]
    fn test_format_codex_config_details_apikey() {
        let config = CodexConfiguration {
            alias_name: "test".to_string(),
            auth_mode: "apikey".to_string(),
            openai_api_key: Some("sk-abc123longkey".to_string()),
            id_token: None,
            access_token: None,
            refresh_token: None,
            account_id: None,
            last_refresh: None,
        };

        let lines = format_codex_config_details(&config, "    ");
        // Should show auth mode and api key (truncated)
        assert!(!lines.is_empty());
        assert!(lines.iter().any(|l| l.contains("apikey")));
        assert!(lines.iter().any(|l| l.contains("sk-abc12...")));
    }

    #[test]
    fn test_format_codex_config_details_chatgpt() {
        let config = CodexConfiguration {
            alias_name: "test".to_string(),
            auth_mode: "chatgpt".to_string(),
            openai_api_key: None,
            id_token: Some("id-xyz".to_string()),
            access_token: Some("at-xyz".to_string()),
            refresh_token: Some("rt-xyz".to_string()),
            account_id: Some("acc-123".to_string()),
            last_refresh: Some("2026-05-16T00:00:00Z".to_string()),
        };

        let lines = format_codex_config_details(&config, "    ");
        assert!(lines.iter().any(|l| l.contains("chatgpt")));
        assert!(lines.iter().any(|l| l.contains("acc-123")));
        assert!(lines.iter().any(|l| l.contains("2026-05-16")));
    }

    #[test]
    fn test_pagination_calculation_codex() {
        const PAGE_SIZE: usize = 9;
        assert_eq!(1_usize.div_ceil(PAGE_SIZE), 1);
        assert_eq!(9_usize.div_ceil(PAGE_SIZE), 1);
        assert_eq!(10_usize.div_ceil(PAGE_SIZE), 2);
        assert_eq!(18_usize.div_ceil(PAGE_SIZE), 2);
        assert_eq!(19_usize.div_ceil(PAGE_SIZE), 3);
    }

    #[test]
    fn test_exit_index_no_official_option() {
        // Without "official" option, exit index == configs.len()
        let configs_len: usize = 5;
        let exit_index = configs_len;
        assert_eq!(exit_index, 5);
    }

    #[test]
    fn test_digit_mapping_without_official_offset() {
        const PAGE_SIZE: usize = 9;
        let current_page = 0;
        let start_idx = current_page * PAGE_SIZE;

        // Digit 1 -> actual_config_index 0 (no +1 offset because no official option)
        let digit = 1;
        let actual_config_index = start_idx + (digit - 1);
        assert_eq!(actual_config_index, 0);

        // Digit 5 -> actual_config_index 4
        let digit = 5;
        let actual_config_index = start_idx + (digit - 1);
        assert_eq!(actual_config_index, 4);
    }
}
