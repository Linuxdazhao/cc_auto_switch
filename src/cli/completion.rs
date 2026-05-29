use crate::config::ConfigStorage;
use anyhow::Result;
use clap::CommandFactory;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

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

/// Return the install path for a shell's completion file, if it has a standard location.
fn completion_install_path(shell: &str) -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    match shell {
        "fish" => Some(home.join(".config/fish/completions/cc-switch.fish")),
        "zsh" => Some(home.join(".zsh/completions/_cc-switch")),
        "bash" => Some(home.join(".bash_completion.d/cc-switch")),
        _ => None,
    }
}

/// Generate shell completion script and install it to the standard path.
///
/// For fish/zsh/bash the output is written directly to the shell's
/// completion directory. For other shells the script is printed to stdout.
///
/// # Errors
/// Returns error if shell is not supported or generation fails
pub fn generate_completion(shell: &str) -> Result<()> {
    use crate::cli::Cli;

    let mut app = Cli::command();
    let mut buf: Vec<u8> = Vec::new();

    match shell {
        "fish" => {
            generate_fish_completion(&mut app, &mut buf);
        }
        "zsh" => {
            clap_complete::generate(clap_complete::shells::Zsh, &mut app, "cc-switch", &mut buf);
        }
        "bash" => {
            clap_complete::generate(clap_complete::shells::Bash, &mut app, "cc-switch", &mut buf);
        }
        "elvish" => {
            clap_complete::generate(
                clap_complete::shells::Elvish,
                &mut app,
                "cc-switch",
                &mut std::io::stdout(),
            );
            return Ok(());
        }
        "powershell" => {
            clap_complete::generate(
                clap_complete::shells::PowerShell,
                &mut app,
                "cc-switch",
                &mut std::io::stdout(),
            );
            return Ok(());
        }
        _ => {
            anyhow::bail!(
                "Unsupported shell: {}. Supported shells: fish, zsh, bash, elvish, powershell",
                shell
            );
        }
    }

    if let Some(path) = completion_install_path(shell) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, &buf)?;
        eprintln!("Installed {shell} completion to {}", path.display());
    } else {
        std::io::stdout().write_all(&buf)?;
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

/// Generate custom fish completion with dynamic alias completion, writing to `out`.
fn generate_fish_completion(app: &mut clap::Command, out: &mut Vec<u8>) {
    clap_complete::generate(clap_complete::shells::Fish, app, "cc-switch", out);

    let extra = r#"
# Custom completion for use subcommand with dynamic aliases
complete -c cc-switch -n '__fish_cc_switch_using_subcommand use' -f -a '(cc-switch --list-aliases)' -d 'Configuration alias name'
# Custom completion for switch subcommand (alias for use)
complete -c cc-switch -n '__fish_cc_switch_using_subcommand switch' -f -a '(cc-switch --list-aliases)' -d 'Configuration alias name'
# Custom completion for remove subcommand with dynamic aliases
complete -c cc-switch -n '__fish_cc_switch_using_subcommand remove' -f -a '(cc-switch --list-aliases)' -d 'Configuration alias name'

# Completion for 'completion' subcommand with shell types
complete -c cc-switch -n '__fish_cc_switch_using_subcommand completion' -f -a 'fish zsh bash elvish powershell' -d 'Shell type'
complete -c cs -n '__fish_seen_subcommand_from completion' -f -a 'fish zsh bash elvish powershell' -d 'Shell type'

# Custom completion for codex subcommand with dynamic aliases
complete -c cc-switch -n '__fish_seen_subcommand_from codex' -n '__fish_seen_subcommand_from use' -f -a '(cc-switch --list-codex-aliases)' -d 'Codex configuration alias name'
complete -c cc-switch -n '__fish_seen_subcommand_from codex' -n '__fish_seen_subcommand_from remove' -f -a '(cc-switch --list-codex-aliases)' -d 'Codex configuration alias name'

# Completion for the 'cs' alias
complete -c cs -w cc-switch

# Completion for 'cs' alias subcommands
complete -c cs -n '__fish_use_subcommand' -f -a 'add remove list set-default-dir completion alias use switch current codex daemon statusline' -d 'Subcommand'

# Completion for 'daemon' subcommand
complete -c cc-switch -n '__fish_cc_switch_using_subcommand daemon; and not __fish_seen_subcommand_from start stop status restart' -f -a 'start stop status restart' -d 'Daemon action'
complete -c cc-switch -n '__fish_cc_switch_using_subcommand daemon; and __fish_seen_subcommand_from start' -l foreground -d 'Run in the foreground'
complete -c cc-switch -n '__fish_cc_switch_using_subcommand daemon; and __fish_seen_subcommand_from start' -l log-level -d 'Log level (error/warn/info/debug/trace)' -r -f -a 'error warn info debug trace'
complete -c cc-switch -n '__fish_cc_switch_using_subcommand daemon; and __fish_seen_subcommand_from start' -s v -l verbose -d 'Increase verbosity (-v/-vv/-vvv)'
complete -c cc-switch -n '__fish_cc_switch_using_subcommand daemon; and __fish_seen_subcommand_from restart' -l foreground -d 'Run in the foreground after restart'
complete -c cc-switch -n '__fish_cc_switch_using_subcommand daemon; and __fish_seen_subcommand_from restart' -l log-level -d 'Log level (error/warn/info/debug/trace)' -r -f -a 'error warn info debug trace'
complete -c cc-switch -n '__fish_cc_switch_using_subcommand daemon; and __fish_seen_subcommand_from restart' -s v -l verbose -d 'Increase verbosity (-v/-vv/-vvv)'
complete -c cc-switch -n '__fish_cc_switch_using_subcommand daemon; and __fish_seen_subcommand_from status' -l json -d 'Output as JSON'

# Completion for 'cs list' subcommand
complete -c cs -n '__fish_seen_subcommand_from list' -l plain -s p -d 'Plain text output'
complete -c cs -n '__fish_seen_subcommand_from list' -l name -s n -d 'Show only name and URL'

# Completion for 'cs daemon' subcommand
complete -c cs -n '__fish_seen_subcommand_from daemon; and not __fish_seen_subcommand_from start stop status restart' -f -a 'start stop status restart' -d 'Daemon action'
complete -c cs -n '__fish_seen_subcommand_from daemon; and __fish_seen_subcommand_from start' -l foreground -d 'Run in the foreground'
complete -c cs -n '__fish_seen_subcommand_from daemon; and __fish_seen_subcommand_from start' -l log-level -d 'Log level' -r -f -a 'error warn info debug trace'
complete -c cs -n '__fish_seen_subcommand_from daemon; and __fish_seen_subcommand_from start' -s v -l verbose -d 'Increase verbosity'
complete -c cs -n '__fish_seen_subcommand_from daemon; and __fish_seen_subcommand_from restart' -l foreground -d 'Run in the foreground after restart'
complete -c cs -n '__fish_seen_subcommand_from daemon; and __fish_seen_subcommand_from restart' -l log-level -d 'Log level' -r -f -a 'error warn info debug trace'
complete -c cs -n '__fish_seen_subcommand_from daemon; and __fish_seen_subcommand_from restart' -s v -l verbose -d 'Increase verbosity'
complete -c cs -n '__fish_seen_subcommand_from daemon; and __fish_seen_subcommand_from status' -l json -d 'Output as JSON'

# Completion for 'cs statusline' subcommand
complete -c cs -n '__fish_seen_subcommand_from statusline' -f -a 'install uninstall' -d 'Statusline action'

# Completion for the 'cx' alias (cc-switch codex)
complete -c cx -f
complete -c cx -n '__fish_use_subcommand' -f -a 'add use remove list' -d 'Codex subcommand'
complete -c cx -n '__fish_seen_subcommand_from use' -f -a '(cc-switch --list-codex-aliases)' -d 'Codex configuration alias name'
complete -c cx -n '__fish_seen_subcommand_from remove' -f -a '(cc-switch --list-codex-aliases)' -d 'Codex configuration alias name'
complete -c cx -n '__fish_seen_subcommand_from list' -f -l plain -s p -d 'Plain text output'
complete -c cx -n '__fish_seen_subcommand_from list' -f -l name -s n -d 'Show only name and auth mode'
complete -c cx -n '__fish_seen_subcommand_from add' -f -l interactive -s i -d 'Interactive mode'
complete -c cx -n '__fish_seen_subcommand_from add' -f -l from-file -d 'Import from auth.json (defaults to ~/.codex/auth.json if no path)' -r
"#;
    out.extend_from_slice(extra.as_bytes());

    generate_cs_completion_file();
    generate_cx_completion_file();
}

/// Generate separate completion file for cs fish alias.
///
/// Fish only auto-loads completions from files named after the command,
/// so `cs` needs its own `cs.fish`.
fn generate_cs_completion_file() {
    let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("~"));
    let completions_dir = home.join(".config").join("fish").join("completions");

    if !completions_dir.exists()
        && let Err(e) = fs::create_dir_all(&completions_dir)
    {
        eprintln!("Warning: Could not create completions directory: {e}");
        return;
    }

    let cs_content = r#"# Completion for 'cs' alias (cc-switch)
complete -c cs -w cc-switch
"#;

    let cs_path = completions_dir.join("cs.fish");

    if let Err(e) = fs::write(&cs_path, cs_content) {
        eprintln!("Warning: Could not write cs.fish: {e}");
    }

    eprintln!("Created completion file: {}", cs_path.display());
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
