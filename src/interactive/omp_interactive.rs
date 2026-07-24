use crate::cli::display_utils::{
    TextAlignment, get_terminal_width, pad_text_to_width, text_display_width,
};
use crate::config::types::ConfigStorage;
use crate::interactive::interactive::{
    EditModeError, cleanup_terminal, edit_optional_string_field, edit_string_field,
};
use crate::omp::{OmpConfiguration, write_env_file};
use crate::platform::resolve_npm_cli;
use anyhow::Result;
use colored::*;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute, terminal,
};
use std::io::{self, Write};
use std::process::Command;

/// Handle interactive OMP configuration selection with full TUI
///
/// Mirrors the Claude interactive TUI: alternate screen, arrow key / j/k navigation,
/// number key shortcuts (1-9), pagination (9 per page), config detail preview.
///
/// # Arguments
/// * `storage` - Reference to configuration storage
///
/// # Errors
/// Returns error if terminal operations fail or user selection fails
pub fn handle_omp_interactive_selection(storage: &ConfigStorage) -> Result<()> {
    let configs_map = match &storage.omp_configurations {
        Some(configs) if !configs.is_empty() => configs,
        _ => {
            println!(
                "No OMP configurations available. Use 'cc-switch omp add' to create configurations first."
            );
            return Ok(());
        }
    };

    let mut configs: Vec<OmpConfiguration> = configs_map.values().cloned().collect();
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
            let result =
                handle_omp_full_interactive_menu(&mut stdout, &mut configs, &mut selected_index);

            // Always restore terminal
            let _ = execute!(stdout, terminal::LeaveAlternateScreen);
            let _ = terminal::disable_raw_mode();

            return result;
        } else {
            let _ = terminal::disable_raw_mode();
        }
    }

    // Fallback to simple numbered menu
    handle_omp_simple_interactive_menu(&configs)
}

/// Format OMP configuration details for display
///
/// # Arguments
/// * `config` - The configuration to format
/// * `indent` - Base indentation string (e.g., "    " or "   ")
///
/// # Returns
/// Vector of formatted lines for configuration display
fn format_omp_config_details(config: &OmpConfiguration, indent: &str) -> Vec<String> {
    let mut lines = Vec::new();

    let terminal_width = get_terminal_width();
    let _available_width = terminal_width.saturating_sub(text_display_width(indent) + 8);

    // Field labels with consistent width for alignment
    let api_key_label = "API Key:";
    let base_url_label = "Base URL:";
    let slow_model_label = "Slow Model:";
    let smol_model_label = "Smol Model:";
    let plan_model_label = "Plan Model:";

    let max_label_width = [
        api_key_label,
        base_url_label,
        slow_model_label,
        smol_model_label,
        plan_model_label,
    ]
    .iter()
    .map(|label| text_display_width(label))
    .max()
    .unwrap_or(0);

    // API Key (masked)
    if let Some(key) = config.openai_api_key.as_ref().map(|k| mask_key(k)) {
        lines.push(format!(
            "{}{} {:<width$} {}",
            indent,
            "•".blue(),
            api_key_label.blue().bold(),
            key,
            width = max_label_width
        ));
    }

    // Base URL
    if let Some(ref url) = config.base_url {
        lines.push(format!(
            "{}{} {:<width$} {}",
            indent,
            "•".blue(),
            base_url_label.blue().bold(),
            url,
            width = max_label_width
        ));
    }

    // Model configurations
    if let Some(ref model) = config.slow_model {
        lines.push(format!(
            "{}{} {:<width$} {}",
            indent,
            "•".blue(),
            slow_model_label.blue().bold(),
            model,
            width = max_label_width
        ));
    }

    if let Some(ref model) = config.smol_model {
        lines.push(format!(
            "{}{} {:<width$} {}",
            indent,
            "•".blue(),
            smol_model_label.blue().bold(),
            model,
            width = max_label_width
        ));
    }

    if let Some(ref model) = config.plan_model {
        lines.push(format!(
            "{}{} {:<width$} {}",
            indent,
            "•".blue(),
            plan_model_label.blue().bold(),
            model,
            width = max_label_width
        ));
    }

    lines
}

/// Mask API key for display (show first 4 and last 4 characters)
fn mask_key(key: &str) -> String {
    if key.len() <= 8 {
        "*".repeat(key.len())
    } else {
        format!("{}****{}", &key[..4], &key[key.len() - 4..])
    }
}

#[allow(clippy::ptr_arg)]
fn handle_omp_full_interactive_menu(
    stdout: &mut io::Stdout,
    configs: &mut Vec<OmpConfiguration>,
    selected_index: &mut usize,
) -> Result<()> {
    let page_size = 9;
    let mut current_page = 0;
    let total_pages = configs.len().div_ceil(page_size);

    loop {
        execute!(stdout, terminal::Clear(terminal::ClearType::All))?;

        // Draw header
        let header = "OMP Configuration Selector";
        let width = get_terminal_width();
        let padded_header = pad_text_to_width(header, width, TextAlignment::Center, ' ');
        writeln!(stdout, "{}", padded_header.white().bold().on_blue())?;
        writeln!(stdout)?;

        if configs.is_empty() {
            writeln!(
                stdout,
                "\r{}",
                "Use 'cc-switch omp add' to add configurations first.".dimmed()
            )?;
            writeln!(stdout, "\r{}", "Press any key to continue...".dimmed())?;
            let _ = event::read();
            return Ok(());
        }

        // Calculate page boundaries
        let start_idx = current_page * page_size;
        let end_idx = (start_idx + page_size).min(configs.len());

        // Display configurations on current page
        for (i, config) in configs[start_idx..end_idx].iter().enumerate() {
            let actual_index = start_idx + i;
            let is_selected = actual_index == *selected_index;

            let indicator = if is_selected {
                format!("{} ", "→".green().bold())
            } else {
                "  ".to_string()
            };

            let number = format!("{:>2}.", i + 1);
            let number_display = if is_selected {
                number.green().bold()
            } else {
                number.normal()
            };

            writeln!(
                stdout,
                "\r{}{} {}",
                indicator,
                number_display,
                config.alias_name.bold()
            )?;

            // Show config details if selected
            if is_selected {
                let details = format_omp_config_details(config, "\r    ");
                for detail_line in details {
                    writeln!(stdout, "{detail_line}")?;
                }
            }
            writeln!(stdout)?;
        }

        // Draw footer
        writeln!(stdout)?;
        let footer = if total_pages > 1 {
            format!(
                "Page {}/{} | ↑↓/j/k: Navigate | 1-9: Select | Enter: Use | e: Edit | d: Delete | n: Next | p: Prev | q: Quit",
                current_page + 1,
                total_pages
            )
        } else {
            "↑↓/j/k: Navigate | 1-9: Select | Enter: Use | e: Edit | d: Delete | q: Quit"
                .to_string()
        };
        let padded_footer = pad_text_to_width(&footer, width, TextAlignment::Center, ' ');
        writeln!(stdout, "{}", padded_footer.white().on_bright_black())?;

        stdout.flush()?;

        // Handle input
        if let Event::Key(KeyEvent {
            code,
            kind: KeyEventKind::Press,
            ..
        }) = event::read()?
        {
            match code {
                KeyCode::Char('q') | KeyCode::Char('Q') => {
                    cleanup_terminal(stdout);
                    return Ok(());
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if *selected_index > 0 {
                        *selected_index -= 1;
                        if *selected_index < start_idx {
                            current_page = current_page.saturating_sub(1);
                        }
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if *selected_index < configs.len() - 1 {
                        *selected_index += 1;
                        if *selected_index >= end_idx {
                            current_page = (current_page + 1).min(total_pages - 1);
                        }
                    }
                }
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    if current_page < total_pages - 1 {
                        current_page += 1;
                        *selected_index = current_page * page_size;
                    }
                }
                KeyCode::Char('p') | KeyCode::Char('P') => {
                    if current_page > 0 {
                        current_page -= 1;
                        *selected_index = current_page * page_size;
                    }
                }
                KeyCode::Char('d') | KeyCode::Char('D') if *selected_index < configs.len() => {
                    cleanup_terminal(stdout);
                    let config_name = configs[*selected_index].alias_name.clone();
                    println!("Delete configuration '{}'? (y/n): ", config_name);
                    let mut input = String::new();
                    io::stdin().read_line(&mut input)?;
                    if input.trim().to_lowercase() == "y" {
                        let mut storage = ConfigStorage::load()?;
                        if storage.remove_omp_configuration(&config_name) {
                            storage.save()?;
                            configs.remove(*selected_index);
                            if *selected_index >= configs.len() && !configs.is_empty() {
                                *selected_index = configs.len() - 1;
                            }
                            println!("Configuration '{}' deleted.", config_name);
                        }
                    }
                    // Re-enter alternate screen
                    execute!(
                        stdout,
                        terminal::EnterAlternateScreen,
                        terminal::Clear(terminal::ClearType::All)
                    )?;
                }
                KeyCode::Char('e') | KeyCode::Char('E') if *selected_index < configs.len() => {
                    cleanup_terminal(stdout);
                    let edit_result = handle_omp_config_edit(&configs[*selected_index]);
                    if execute!(
                        stdout,
                        terminal::EnterAlternateScreen,
                        terminal::Clear(terminal::ClearType::All)
                    )
                    .is_err()
                    {
                        return edit_result;
                    }
                    if edit_result.is_ok() {
                        // Reload configurations
                        if let Ok(reloaded_storage) = ConfigStorage::load()
                            && let Some(ref map) = reloaded_storage.omp_configurations
                        {
                            *configs = map.values().cloned().collect();
                            configs.sort_by(|a, b| a.alias_name.cmp(&b.alias_name));
                        }
                    }
                }
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    let digit = c.to_digit(10).unwrap() as usize;
                    if digit >= 1 && digit <= (end_idx - start_idx) {
                        let actual_config_index = start_idx + (digit - 1);
                        cleanup_terminal(stdout);
                        return handle_omp_selection_action(configs, actual_config_index);
                    }
                }
                KeyCode::Enter => {
                    if *selected_index < configs.len() {
                        cleanup_terminal(stdout);
                        return handle_omp_selection_action(configs, *selected_index);
                    }
                }
                KeyCode::Esc => {
                    cleanup_terminal(stdout);
                    return Ok(());
                }
                _ => {}
            }
        }
    }
}

/// Handle simple interactive menu (fallback) for OMP
fn handle_omp_simple_interactive_menu(configs: &[OmpConfiguration]) -> Result<()> {
    const PAGE_SIZE: usize = 9;

    if configs.len() <= PAGE_SIZE {
        return handle_omp_simple_single_page_menu(configs);
    }

    // Multi-page simple menu
    let total_pages = configs.len().div_ceil(PAGE_SIZE);
    let mut current_page = 0;

    loop {
        println!("\n{}", "Available OMP Configurations:".blue().bold());
        println!("Page {}/{}", current_page + 1, total_pages);
        println!("{}", "-".repeat(50));

        let start_idx = current_page * PAGE_SIZE;
        let end_idx = (start_idx + PAGE_SIZE).min(configs.len());

        for (i, config) in configs[start_idx..end_idx].iter().enumerate() {
            let number = start_idx + i + 1;
            println!(
                "{:>2}. {}{}",
                number,
                config.alias_name.bold(),
                config
                    .slow_model
                    .as_ref()
                    .map(|m| format!(" ({})", m))
                    .unwrap_or_default()
            );
        }

        println!(
            "\nEnter number (1-{}), 'n' for next, 'p' for prev, or 'q' to quit: ",
            end_idx - start_idx
        );

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        match input {
            "q" | "Q" => return Ok(()),
            "n" | "N" => {
                if current_page < total_pages - 1 {
                    current_page += 1;
                }
            }
            "p" | "P" => {
                current_page = current_page.saturating_sub(1);
            }
            _ => {
                if let Ok(num) = input.parse::<usize>()
                    && num >= 1
                    && num <= configs.len()
                {
                    return handle_omp_selection_action(configs, num - 1);
                }
                println!("Invalid selection, please try again");
            }
        }
    }
}

/// Handle simple single page menu (original behavior for ≤9 configs)
fn handle_omp_simple_single_page_menu(configs: &[OmpConfiguration]) -> Result<()> {
    println!("\n{}", "Available OMP Configurations:".blue().bold());

    for (index, config) in configs.iter().enumerate() {
        println!("{:>2}. {}", index + 1, config.alias_name.bold());

        let details = format_omp_config_details(config, "   ");
        for detail_line in details {
            println!("{detail_line}");
        }
        println!();
    }

    println!("Enter number (1-{}), or 'q' to quit: ", configs.len());

    loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input == "q" || input == "Q" {
            return Ok(());
        }

        if let Ok(num) = input.parse::<usize>()
            && num >= 1
            && num <= configs.len()
        {
            return handle_omp_selection_action(configs, num - 1);
        }

        println!("Invalid selection, please try again");
    }
}

/// Handle the action after a configuration is selected
fn handle_omp_selection_action(configs: &[OmpConfiguration], index: usize) -> Result<()> {
    let config = &configs[index];
    println!(
        "\nSwitching to OMP configuration '{}'...",
        config.alias_name
    );

    // Write the .env file
    write_env_file(config)?;

    // Launch OMP
    let omp_path = resolve_npm_cli("omp");
    let status = Command::new(omp_path).status()?;

    if !status.success() {
        eprintln!("OMP exited with non-zero status: {}", status);
    }

    Ok(())
}

/// Handle editing an OMP configuration interactively
fn handle_omp_config_edit(config: &OmpConfiguration) -> Result<()> {
    println!("\n{}", "Edit OMP Configuration".bold());
    println!("{}", "=".repeat(50));

    let mut storage = ConfigStorage::load()?;
    let mut edited_config = config.clone();

    // Edit alias name
    match edit_string_field("Alias name", &edited_config.alias_name, |_| Ok(())) {
        Ok(Some(new_alias)) => {
            edited_config.alias_name = new_alias;
        }
        Ok(None) => {}
        Err(e) => {
            if e.downcast_ref::<EditModeError>() == Some(&EditModeError::ReturnToMenu) {
                println!("Edit cancelled.");
                return Ok(());
            }
            return Err(e);
        }
    }

    // Edit OpenAI API key
    let current_key = edited_config.openai_api_key.clone().unwrap_or_default();
    match edit_string_field("OpenAI API Key", &current_key, |_| Ok(())) {
        Ok(Some(new_key)) => {
            edited_config.openai_api_key = Some(new_key);
        }
        Ok(None) => {}
        Err(e) => {
            if e.downcast_ref::<EditModeError>() == Some(&EditModeError::ReturnToMenu) {
                println!("Edit cancelled.");
                return Ok(());
            }
            return Err(e);
        }
    }

    // Edit base URL
    if let Some(result) = edit_optional_string_field("Base URL", edited_config.base_url.as_deref())?
    {
        edited_config.base_url = result;
    }

    // Edit model configurations
    if let Some(result) =
        edit_optional_string_field("Slow Model", edited_config.slow_model.as_deref())?
    {
        edited_config.slow_model = result;
    }

    if let Some(result) =
        edit_optional_string_field("Smol Model", edited_config.smol_model.as_deref())?
    {
        edited_config.smol_model = result;
    }

    if let Some(result) =
        edit_optional_string_field("Plan Model", edited_config.plan_model.as_deref())?
    {
        edited_config.plan_model = result;
    }

    // Save the edited configuration
    storage.update_omp_configuration(&config.alias_name, edited_config)?;
    storage.save()?;

    println!("\n{}", "Configuration updated successfully!".green().bold());
    Ok(())
}
