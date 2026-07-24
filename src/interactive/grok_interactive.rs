use crate::cli::display_utils::{
    TextAlignment, get_terminal_width, pad_text_to_width, text_display_width,
};
use crate::config::types::ConfigStorage;
use crate::grok::{GrokConfiguration, write_grok_config};
use crate::interactive::interactive::{
    EditModeError, cleanup_terminal, edit_optional_string_field, edit_string_field,
};
use crate::platform::resolve_npm_cli;
use anyhow::Result;
use colored::*;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute, terminal,
};
use std::io::{self, Write};
use std::process::Command;

/// Handle interactive grok configuration selection with full TUI.
///
/// Mirrors the OMP interactive TUI: alternate screen, arrow key / j/k navigation,
/// number key shortcuts (1-9), pagination (9 per page), config detail preview.
pub fn handle_grok_interactive_selection(storage: &ConfigStorage) -> Result<()> {
    let configs_map = match &storage.grok_configurations {
        Some(configs) if !configs.is_empty() => configs,
        _ => {
            println!(
                "No grok configurations available. Use 'cc-switch grok add' to create configurations first."
            );
            return Ok(());
        }
    };

    let mut configs: Vec<GrokConfiguration> = configs_map.values().cloned().collect();
    configs.sort_by(|a, b| a.alias_name.cmp(&b.alias_name));

    let mut selected_index: usize = 0;

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
                handle_grok_full_interactive_menu(&mut stdout, &mut configs, &mut selected_index);

            let _ = execute!(stdout, terminal::LeaveAlternateScreen);
            let _ = terminal::disable_raw_mode();

            return result;
        } else {
            let _ = terminal::disable_raw_mode();
        }
    }

    handle_grok_simple_interactive_menu(&configs)
}

/// Format grok configuration details for display.
fn format_grok_config_details(config: &GrokConfiguration, indent: &str) -> Vec<String> {
    let mut lines = Vec::new();

    let model_label = "Model:";
    let base_url_label = "Base URL:";
    let backend_label = "API Backend:";
    let auth_label = "Auth:";

    let max_label_width = [model_label, base_url_label, backend_label, auth_label]
        .iter()
        .map(|label| text_display_width(label))
        .max()
        .unwrap_or(0);

    lines.push(format!(
        "{}{} {:<width$} {}",
        indent,
        "•".blue(),
        model_label.blue().bold(),
        config.model.green(),
        width = max_label_width
    ));

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

    if let Some(ref backend) = config.api_backend {
        lines.push(format!(
            "{}{} {:<width$} {}",
            indent,
            "•".blue(),
            backend_label.blue().bold(),
            backend,
            width = max_label_width
        ));
    }

    lines.push(format!(
        "{}{} {:<width$} {}",
        indent,
        "•".blue(),
        auth_label.blue().bold(),
        auth_display(config),
        width = max_label_width
    ));

    lines
}

fn auth_display(config: &GrokConfiguration) -> String {
    if let Some(keys) = &config.env_key
        && !keys.is_empty()
    {
        return format!("env:{}", keys.join(","));
    }
    if let Some(key) = &config.api_key {
        return mask_key(key);
    }
    if config.extra_headers.is_some() {
        return "headers".to_string();
    }
    "(none)".to_string()
}

fn mask_key(key: &str) -> String {
    if key.len() <= 8 {
        "*".repeat(key.len())
    } else {
        format!("{}****{}", &key[..4], &key[key.len() - 4..])
    }
}

#[allow(clippy::ptr_arg)]
fn handle_grok_full_interactive_menu(
    stdout: &mut io::Stdout,
    configs: &mut Vec<GrokConfiguration>,
    selected_index: &mut usize,
) -> Result<()> {
    let page_size = 9;
    let mut current_page = 0;
    let total_pages = configs.len().div_ceil(page_size);

    loop {
        execute!(stdout, terminal::Clear(terminal::ClearType::All))?;

        let header = "grok Configuration Selector";
        let width = get_terminal_width();
        let padded_header = pad_text_to_width(header, width, TextAlignment::Center, ' ');
        writeln!(stdout, "{}", padded_header.white().bold().on_blue())?;
        writeln!(stdout)?;

        if configs.is_empty() {
            writeln!(
                stdout,
                "\r{}",
                "Use 'cc-switch grok add' to add configurations first.".dimmed()
            )?;
            writeln!(stdout, "\r{}", "Press any key to continue...".dimmed())?;
            let _ = event::read();
            return Ok(());
        }

        let start_idx = current_page * page_size;
        let end_idx = (start_idx + page_size).min(configs.len());

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

            if is_selected {
                let details = format_grok_config_details(config, "\r    ");
                for detail_line in details {
                    writeln!(stdout, "{detail_line}")?;
                }
            }
            writeln!(stdout)?;
        }

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
                        if storage.remove_grok_configuration(&config_name) {
                            storage.save()?;
                            configs.remove(*selected_index);
                            if *selected_index >= configs.len() && !configs.is_empty() {
                                *selected_index = configs.len() - 1;
                            }
                            println!("Configuration '{}' deleted.", config_name);
                        }
                    }
                    execute!(
                        stdout,
                        terminal::EnterAlternateScreen,
                        terminal::Clear(terminal::ClearType::All)
                    )?;
                }
                KeyCode::Char('e') | KeyCode::Char('E') if *selected_index < configs.len() => {
                    cleanup_terminal(stdout);
                    let edit_result = handle_grok_config_edit(&configs[*selected_index]);
                    if execute!(
                        stdout,
                        terminal::EnterAlternateScreen,
                        terminal::Clear(terminal::ClearType::All)
                    )
                    .is_err()
                    {
                        return edit_result;
                    }
                    if edit_result.is_ok()
                        && let Ok(reloaded_storage) = ConfigStorage::load()
                        && let Some(ref map) = reloaded_storage.grok_configurations
                    {
                        *configs = map.values().cloned().collect();
                        configs.sort_by(|a, b| a.alias_name.cmp(&b.alias_name));
                    }
                }
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    let digit = c.to_digit(10).unwrap() as usize;
                    if digit >= 1 && digit <= (end_idx - start_idx) {
                        let actual_config_index = start_idx + (digit - 1);
                        cleanup_terminal(stdout);
                        return handle_grok_selection_action(configs, actual_config_index);
                    }
                }
                KeyCode::Enter => {
                    if *selected_index < configs.len() {
                        cleanup_terminal(stdout);
                        return handle_grok_selection_action(configs, *selected_index);
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

/// Handle simple interactive menu (fallback) for grok.
fn handle_grok_simple_interactive_menu(configs: &[GrokConfiguration]) -> Result<()> {
    const PAGE_SIZE: usize = 9;

    if configs.len() <= PAGE_SIZE {
        return handle_grok_simple_single_page_menu(configs);
    }

    let total_pages = configs.len().div_ceil(PAGE_SIZE);
    let mut current_page = 0;

    loop {
        println!("\n{}", "Available grok Configurations:".blue().bold());
        println!("Page {}/{}", current_page + 1, total_pages);
        println!("{}", "-".repeat(50));

        let start_idx = current_page * PAGE_SIZE;
        let end_idx = (start_idx + PAGE_SIZE).min(configs.len());

        for (i, config) in configs[start_idx..end_idx].iter().enumerate() {
            let number = start_idx + i + 1;
            println!(
                "{:>2}. {} ({})",
                number,
                config.alias_name.bold(),
                config.model
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
                    return handle_grok_selection_action(configs, num - 1);
                }
                println!("Invalid selection, please try again");
            }
        }
    }
}

/// Handle simple single page menu (for ≤9 configs).
fn handle_grok_simple_single_page_menu(configs: &[GrokConfiguration]) -> Result<()> {
    println!("\n{}", "Available grok Configurations:".blue().bold());

    for (index, config) in configs.iter().enumerate() {
        println!(
            "{:>2}. {} ({})",
            index + 1,
            config.alias_name.bold(),
            config.model
        );

        let details = format_grok_config_details(config, "   ");
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
            return handle_grok_selection_action(configs, num - 1);
        }

        println!("Invalid selection, please try again");
    }
}

/// Handle the action after a configuration is selected.
fn handle_grok_selection_action(configs: &[GrokConfiguration], index: usize) -> Result<()> {
    let config = &configs[index];
    println!(
        "\nSwitching to grok configuration '{}' (model: {})...",
        config.alias_name, config.model
    );

    write_grok_config(config)?;

    let grok_path = resolve_npm_cli("grok");
    let status = Command::new(grok_path).status()?;

    if !status.success() {
        eprintln!("grok exited with non-zero status: {}", status);
    }

    Ok(())
}

/// Handle editing a grok configuration interactively.
fn handle_grok_config_edit(config: &GrokConfiguration) -> Result<()> {
    println!("\n{}", "Edit grok Configuration".bold());
    println!("{}", "=".repeat(50));

    let mut storage = ConfigStorage::load()?;
    let mut edited_config = config.clone();

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

    match edit_string_field("Model id", &edited_config.model, |_| Ok(())) {
        Ok(Some(new_model)) => {
            edited_config.model = new_model;
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

    if let Some(result) = edit_optional_string_field("Base URL", edited_config.base_url.as_deref())?
    {
        edited_config.base_url = result;
    }

    if let Some(result) = edit_optional_string_field("Name", edited_config.name.as_deref())? {
        edited_config.name = result;
    }

    if let Some(result) =
        edit_optional_string_field("API Backend", edited_config.api_backend.as_deref())?
    {
        edited_config.api_backend = result;
    }

    if let Some(result) = edit_optional_string_field("API Key", edited_config.api_key.as_deref())? {
        edited_config.api_key = result;
    }

    let env_key_str = edited_config
        .env_key
        .as_ref()
        .map(|v| v.join(","))
        .unwrap_or_default();
    if let Some(result) =
        edit_optional_string_field("Env keys (comma-separated)", Some(env_key_str.as_str()))?
    {
        edited_config.env_key = result.map(|s| {
            s.split(',')
                .map(|k| k.trim().to_string())
                .filter(|k| !k.is_empty())
                .collect::<Vec<_>>()
        });
    }

    storage.update_grok_configuration(&config.alias_name, edited_config)?;
    storage.save()?;

    println!("\n{}", "Configuration updated successfully!".green().bold());
    Ok(())
}
