use crate::config::ConfigStorage;
use anyhow::Result;
use clap::CommandFactory;
use colored::*;
use std::fs;
use std::io::stdout;

/// Generate shell aliases for eval
///
/// # Arguments
/// * `shell` - Shell type (fish, zsh, bash)
///
/// # Errors
/// Returns error if shell is not supported
pub fn generate_aliases(shell: &str) -> Result<()> {
    match shell {
        "fish" => {
            println!("alias cs='cc-switch'");
            println!("alias ccd='claude --dangerously-skip-permissions'");
            println!("alias cx='cc-switch codex'");
        }
        "zsh" => {
            println!("alias cs='cc-switch'");
            println!("alias ccd='claude --dangerously-skip-permissions'");
            println!("alias cx='cc-switch codex'");
        }
        "bash" => {
            println!("alias cs='cc-switch'");
            println!("alias ccd='claude --dangerously-skip-permissions'");
            println!("alias cx='cc-switch codex'");
        }
        _ => {
            anyhow::bail!(
                "Unsupported shell: {}. Supported shells: fish, zsh, bash",
                shell
            );
        }
    }

    Ok(())
}

/// Generate shell completion script
///
/// # Arguments
/// * `shell` - Shell type (fish, zsh, bash, elvish, powershell, nushell)
///
/// # Errors
/// Returns error if shell is not supported or generation fails
pub fn generate_completion(shell: &str) -> Result<()> {
    use crate::cli::Cli;

    let mut app = Cli::command();

    match shell {
        "fish" => {
            generate_fish_completion(&mut app);
        }
        "zsh" => {
            clap_complete::generate(
                clap_complete::shells::Zsh,
                &mut app,
                "cc-switch",
                &mut stdout(),
            );

            // Add aliases for zsh
            println!("\n# Useful aliases for cc-switch");
            println!("# Add these aliases to your ~/.zshrc:");
            println!("alias cs='cc-switch'");
            println!("alias ccd='claude --dangerously-skip-permissions'");
            println!("# Or run this command to add aliases temporarily:");
            println!("alias cs='cc-switch'; alias ccd='claude --dangerously-skip-permissions'");

            println!("\n# Zsh completion generated successfully");
            println!("# Add this to your ~/.zsh/completions/_cc-switch");
            println!("# Or add this line to your ~/.zshrc:");
            println!("# fpath=(~/.zsh/completions $fpath)");
            println!("# Then restart your shell or run 'source ~/.zshrc'");
            println!(
                "{}",
                "# Aliases 'cs' and 'ccd' have been added for convenience".green()
            );
        }
        "bash" => {
            clap_complete::generate(
                clap_complete::shells::Bash,
                &mut app,
                "cc-switch",
                &mut stdout(),
            );

            // Add aliases for bash
            println!("\n# Useful aliases for cc-switch");
            println!("# Add these aliases to your ~/.bashrc or ~/.bash_profile:");
            println!("alias cs='cc-switch'");
            println!("alias ccd='claude --dangerously-skip-permissions'");
            println!("# Or run this command to add aliases temporarily:");
            println!("alias cs='cc-switch'; alias ccd='claude --dangerously-skip-permissions'");

            println!("\n# Bash completion generated successfully");
            println!("# Add this to your ~/.bash_completion or /etc/bash_completion.d/");
            println!("# Then restart your shell or run 'source ~/.bashrc'");
            println!(
                "{}",
                "# Aliases 'cs' and 'ccd' have been added for convenience".green()
            );
        }
        "elvish" => {
            clap_complete::generate(
                clap_complete::shells::Elvish,
                &mut app,
                "cc-switch",
                &mut stdout(),
            );

            // Add aliases for elvish
            println!("\n# Useful aliases for cc-switch");
            println!("fn cs {{|@args| cc-switch $@args }}");
            println!("fn ccd {{|@args| claude --dangerously-skip-permissions $@args }}");

            println!("\n# Elvish completion generated successfully");
            println!(
                "{}",
                "# Aliases 'cs' and 'ccd' have been added for convenience".green()
            );
        }
        "powershell" => {
            clap_complete::generate(
                clap_complete::shells::PowerShell,
                &mut app,
                "cc-switch",
                &mut stdout(),
            );

            // Add aliases for PowerShell
            println!("\n# Useful aliases for cc-switch");
            println!("Set-Alias -Name cs -Value cc-switch");
            println!("function ccd {{ claude --dangerously-skip-permissions @args }}");

            println!("\n# PowerShell completion generated successfully");
            println!(
                "{}",
                "# Aliases 'cs' and 'ccd' have been added for convenience".green()
            );
        }
        _ => {
            anyhow::bail!(
                "Unsupported shell: {}. Supported shells: fish, zsh, bash, elvish, powershell",
                shell
            );
        }
    }

    Ok(())
}

/// List available configuration aliases for shell completion
///
/// Outputs all stored configuration aliases, one per line
/// Also includes 'cc' and 'official' as special aliases for resetting to default Claude
/// For contexts where user types 'cc-switch use c' or similar, 'current' is prioritized first
///
/// # Errors
/// Returns error if loading configurations fails
pub fn list_aliases_for_completion() -> Result<()> {
    let storage = ConfigStorage::load()?;

    // Always include 'cc' and 'official' for reset functionality
    println!("cc");
    println!("official");

    // Prioritize 'current' first if it exists - this ensures when user types 'cc-switch use c'
    // or 'cs use c', the 'current' configuration appears first in completion
    if storage.configurations.contains_key("current") {
        println!("current");
    }

    // Output all other stored aliases in alphabetical order
    let mut aliases: Vec<String> = storage.configurations.keys().cloned().collect();
    aliases.sort();

    for alias_name in aliases {
        if alias_name != "current" {
            println!("{alias_name}");
        }
    }

    Ok(())
}

/// List available Codex configuration aliases for shell completion
///
/// Outputs all stored Codex configuration aliases, one per line
///
/// # Errors
/// Returns error if loading configurations fails
pub fn list_codex_aliases_for_completion() -> Result<()> {
    let storage = ConfigStorage::load()?;

    // Output all stored Codex aliases in alphabetical order
    if let Some(ref configs) = storage.codex_configurations {
        let mut aliases: Vec<String> = configs.keys().cloned().collect();
        aliases.sort();

        for alias_name in aliases {
            println!("{alias_name}");
        }
    }

    Ok(())
}

/// Generate custom fish completion with dynamic alias completion
///
/// # Arguments
/// * `app` - The CLI application struct
fn generate_fish_completion(app: &mut clap::Command) {
    // Generate basic completion
    clap_complete::generate(
        clap_complete::shells::Fish,
        app,
        "cc-switch",
        &mut std::io::stdout(),
    );

    // Add custom completion for use subcommand with dynamic aliases
    println!("\n# Custom completion for use subcommand with dynamic aliases");
    println!(
        "complete -c cc-switch -n '__fish_cc_switch_using_subcommand use' -f -a '(cc-switch --list-aliases)' -d 'Configuration alias name'"
    );
    // Also support 'switch' as alias for 'use'
    println!("# Custom completion for switch subcommand (alias for use)");
    println!(
        "complete -c cc-switch -n '__fish_cc_switch_using_subcommand switch' -f -a '(cc-switch --list-aliases)' -d 'Configuration alias name'"
    );
    // Custom completion for remove subcommand with dynamic aliases
    println!("# Custom completion for remove subcommand with dynamic aliases");
    println!(
        "complete -c cc-switch -n '__fish_cc_switch_using_subcommand remove' -f -a '(cc-switch --list-aliases)' -d 'Configuration alias name'"
    );

    // Add custom completion for codex subcommand
    println!("\n# Custom completion for codex subcommand with dynamic aliases");
    println!(
        "complete -c cc-switch -n '__fish_seen_subcommand_from codex' -n '__fish_seen_subcommand_from use' -f -a '(cc-switch --list-codex-aliases)' -d 'Codex configuration alias name'"
    );
    println!(
        "complete -c cc-switch -n '__fish_seen_subcommand_from codex' -n '__fish_seen_subcommand_from remove' -f -a '(cc-switch --list-codex-aliases)' -d 'Codex configuration alias name'"
    );

    // Add useful aliases that can be eval'd
    println!("\n# To use these aliases immediately, run:");
    println!("# eval \"(cc-switch alias fish)\"");
    println!("\n# Or add them permanently to your ~/.config/fish/config.fish:");
    println!("# echo \"alias cs='cc-switch'\" >> ~/.config/fish/config.fish");
    println!(
        "# echo \"alias ccd='claude --dangerously-skip-permissions'\" >> ~/.config/fish/config.fish"
    );
    println!("\n# IMPORTANT: For cs alias completion to work, you must also:");
    println!(
        "# 1. Add the completion script: cc-switch completion fish > ~/.config/fish/completions/cc-switch.fish"
    );
    println!("# 2. OR run: eval \"(cc-switch completion fish)\" | source");

    // Add completion for the 'cs' alias
    println!("\n# Completion for the 'cs' alias");
    println!("complete -c cs -w cc-switch");

    // Add completion for cs alias subcommands (but NOT configuration aliases at top level)
    println!("\n# Completion for 'cs' alias subcommands");
    println!(
        "complete -c cs -n '__fish_use_subcommand' -f -a 'add remove list set-default-dir completion alias use switch current codex daemon statusline' -d 'Subcommand'"
    );

    // Add completion for daemon subcommand
    println!("\n# Completion for 'daemon' subcommand");
    println!(
        "complete -c cc-switch -n '__fish_cc_switch_using_subcommand daemon; and not __fish_seen_subcommand_from start stop status restart' -f -a 'start stop status restart' -d 'Daemon action'"
    );
    println!(
        "complete -c cc-switch -n '__fish_cc_switch_using_subcommand daemon; and __fish_seen_subcommand_from start' -l foreground -d 'Run in the foreground'"
    );
    println!(
        "complete -c cc-switch -n '__fish_cc_switch_using_subcommand daemon; and __fish_seen_subcommand_from start' -l log-level -d 'Log level (error/warn/info/debug/trace)' -r -f -a 'error warn info debug trace'"
    );
    println!(
        "complete -c cc-switch -n '__fish_cc_switch_using_subcommand daemon; and __fish_seen_subcommand_from start' -s v -l verbose -d 'Increase verbosity (-v/-vv/-vvv)'"
    );
    println!(
        "complete -c cc-switch -n '__fish_cc_switch_using_subcommand daemon; and __fish_seen_subcommand_from restart' -l foreground -d 'Run in the foreground after restart'"
    );
    println!(
        "complete -c cc-switch -n '__fish_cc_switch_using_subcommand daemon; and __fish_seen_subcommand_from restart' -l log-level -d 'Log level (error/warn/info/debug/trace)' -r -f -a 'error warn info debug trace'"
    );
    println!(
        "complete -c cc-switch -n '__fish_cc_switch_using_subcommand daemon; and __fish_seen_subcommand_from restart' -s v -l verbose -d 'Increase verbosity (-v/-vv/-vvv)'"
    );
    println!(
        "complete -c cc-switch -n '__fish_cc_switch_using_subcommand daemon; and __fish_seen_subcommand_from status' -l json -d 'Output as JSON'"
    );

    // Add completion for cs list subcommand flags
    println!("\n# Completion for 'cs list' subcommand");
    println!(
        "complete -c cs -n '__fish_seen_subcommand_from list' -l plain -s p -d 'Plain text output'"
    );
    println!(
        "complete -c cs -n '__fish_seen_subcommand_from list' -l name -s n -d 'Show only name and URL'"
    );

    // Add completion for 'cs daemon' subcommand
    println!("\n# Completion for 'cs daemon' subcommand");
    println!(
        "complete -c cs -n '__fish_seen_subcommand_from daemon; and not __fish_seen_subcommand_from start stop status restart' -f -a 'start stop status restart' -d 'Daemon action'"
    );
    println!(
        "complete -c cs -n '__fish_seen_subcommand_from daemon; and __fish_seen_subcommand_from start' -l foreground -d 'Run in the foreground'"
    );
    println!(
        "complete -c cs -n '__fish_seen_subcommand_from daemon; and __fish_seen_subcommand_from start' -l log-level -d 'Log level' -r -f -a 'error warn info debug trace'"
    );
    println!(
        "complete -c cs -n '__fish_seen_subcommand_from daemon; and __fish_seen_subcommand_from start' -s v -l verbose -d 'Increase verbosity'"
    );
    println!(
        "complete -c cs -n '__fish_seen_subcommand_from daemon; and __fish_seen_subcommand_from restart' -l foreground -d 'Run in the foreground after restart'"
    );
    println!(
        "complete -c cs -n '__fish_seen_subcommand_from daemon; and __fish_seen_subcommand_from restart' -l log-level -d 'Log level' -r -f -a 'error warn info debug trace'"
    );
    println!(
        "complete -c cs -n '__fish_seen_subcommand_from daemon; and __fish_seen_subcommand_from restart' -s v -l verbose -d 'Increase verbosity'"
    );
    println!(
        "complete -c cs -n '__fish_seen_subcommand_from daemon; and __fish_seen_subcommand_from status' -l json -d 'Output as JSON'"
    );

    // Add completion for 'cs statusline' subcommand
    println!("\n# Completion for 'cs statusline' subcommand");
    println!(
        "complete -c cs -n '__fish_seen_subcommand_from statusline' -f -a 'install uninstall' -d 'Statusline action'"
    );

    // Add completion for the 'cx' alias (cc-switch codex)
    println!("\n# Completion for the 'cx' alias (cc-switch codex)");
    println!("complete -c cx -f");
    println!(
        "complete -c cx -n '__fish_use_subcommand' -f -a 'add use remove list' -d 'Codex subcommand'"
    );
    println!(
        "complete -c cx -n '__fish_seen_subcommand_from use' -f -a '(cc-switch --list-codex-aliases)' -d 'Codex configuration alias name'"
    );
    println!(
        "complete -c cx -n '__fish_seen_subcommand_from remove' -f -a '(cc-switch --list-codex-aliases)' -d 'Codex configuration alias name'"
    );
    println!(
        "complete -c cx -n '__fish_seen_subcommand_from list' -f -l plain -s p -d 'Plain text output'"
    );
    println!(
        "complete -c cx -n '__fish_seen_subcommand_from list' -f -l name -s n -d 'Show only name and auth mode'"
    );
    println!(
        "complete -c cx -n '__fish_seen_subcommand_from add' -f -l interactive -s i -d 'Interactive mode'"
    );
    println!(
        "complete -c cx -n '__fish_seen_subcommand_from add' -f -l from-file -d 'Import from auth.json (defaults to ~/.codex/auth.json if no path)' -r"
    );

    println!("\n# Fish completion generated successfully");
    println!("# Add this to your ~/.config/fish/completions/cc-switch.fish");
    println!("# Then restart your shell or run 'source ~/.config/fish/config.fish'");
    println!(
        "{}",
        "# Aliases 'cs' and 'ccd' have been added for convenience".green()
    );

    // Generate separate completion file for cx function
    // Fish doesn't auto-load completions for functions, so we create a dedicated file
    generate_cx_completion_file();
}

/// Generate separate completion file for cx fish function
///
/// Fish doesn't automatically load completion files for functions, only for commands.
/// This creates ~/.config/fish/completions/cx.fish
fn generate_cx_completion_file() {
    // Fish uses ~/.config/fish/completions on all platforms (including macOS)
    let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("~"));
    let completions_dir = home.join(".config").join("fish").join("completions");

    if !completions_dir.exists()
        && let Err(e) = fs::create_dir_all(&completions_dir)
    {
        eprintln!("Warning: Could not create completions directory: {e}");
        return;
    }

    let cx_content = r#"# Completion for 'cx' alias (cc-switch codex)
# cx is a fish function; disable file completion by default
complete -c cx -f
complete -c cx -n '__fish_use_subcommand' -f -a 'add use remove list' -d 'Codex subcommand'
complete -c cx -n '__fish_seen_subcommand_from use' -f -a '(cc-switch --list-codex-aliases)' -d 'Codex configuration alias name'
complete -c cx -n '__fish_seen_subcommand_from remove' -f -a '(cc-switch --list-codex-aliases)' -d 'Codex configuration alias name'
complete -c cx -n '__fish_seen_subcommand_from list' -f -l plain -s p -d 'Plain text output'
complete -c cx -n '__fish_seen_subcommand_from list' -f -l name -s n -d 'Show only name and auth mode'
complete -c cx -n '__fish_seen_subcommand_from add' -f -l interactive -s i -d 'Interactive mode'
complete -c cx -n '__fish_seen_subcommand_from add' -f -l from-file -d 'Import from auth.json (defaults to ~/.codex/auth.json if no path)' -r
"#;

    let cx_path = completions_dir.join("cx.fish");

    if let Err(e) = fs::write(&cx_path, cx_content) {
        eprintln!("Warning: Could not write cx.fish: {e}");
    }

    eprintln!("\nCreated completion file: {}", cx_path.display());
}
