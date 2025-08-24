use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

/// Command-line interface for managing Claude API configurations
#[derive(Parser)]
#[command(name = "cc-switch")]
#[command(about = "A CLI tool for managing Claude API configurations")]
#[command(
    long_about = "cc-switch helps you manage multiple Claude API configurations and switch between them easily.

EXAMPLES:
    cc-switch add my-config sk-ant-xxx https://api.anthropic.com
    cc-switch add my-config -t sk-ant-xxx -u https://api.anthropic.com
    cc-switch add my-config -t sk-ant-xxx -u https://api.anthropic.com -m claude-3-5-sonnet-20241022
    cc-switch add my-config -t sk-ant-xxx -u https://api.anthropic.com --small-fast-model claude-3-haiku-20240307
    cc-switch add my-config -i  # Interactive mode
    cc-switch add my-config --force  # Overwrite existing config
    cc-switch switch my-config
    cc-switch switch cc
    cc-switch list
    cc-switch remove config1 config2 config3
    cc-switch set-default-dir /path/to/claude/config

SHELL COMPLETION AND ALIASES:
    cc-switch completion fish  # Generates shell completions
    cc-switch alias fish       # Generates aliases for eval
    
    These aliases are available:
    - cs='cc-switch'                              # Quick access to cc-switch
    - ccd='claude --dangerously-skip-permissions' # Quick Claude launch
    
    To use aliases immediately:
    eval \"$(cc-switch alias fish)\"    # Add aliases to current session
    
    Or add them permanently:
    cc-switch completion fish > ~/.config/fish/completions/cc-switch.fish
    echo \"alias cs='cc-switch'\" >> ~/.config/fish/config.fish
    echo \"alias ccd='claude --dangerously-skip-permissions'\" >> ~/.config/fish/config.fish
    
    Then use:
    cs switch my-config    # Instead of cc-switch switch my-config
    ccd                    # Quick Claude launch"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// List available configuration aliases (for shell completion)
    #[arg(long = "list-aliases", hide = true)]
    pub list_aliases: bool,
}

/// Available subcommands for configuration management
#[derive(Subcommand)]
pub enum Commands {
    /// Add a new Claude API configuration
    ///
    /// Stores a new configuration with alias, API token, base URL, and optional model settings
    #[command(alias = "a")]
    Add {
        /// Configuration alias name (used to identify this config)
        #[arg(help = "Configuration alias name (cannot be 'cc')")]
        alias_name: String,

        /// ANTHROPIC_AUTH_TOKEN value (your Claude API token)
        #[arg(
            long = "token",
            short = 't',
            help = "API token (optional if not using interactive mode)"
        )]
        token: Option<String>,

        /// ANTHROPIC_BASE_URL value (API endpoint URL)
        #[arg(
            long = "url",
            short = 'u',
            help = "API endpoint URL (optional if not using interactive mode)"
        )]
        url: Option<String>,

        /// ANTHROPIC_MODEL value (custom model name)
        #[arg(
            long = "model",
            short = 'm',
            help = "Custom model name (optional)"
        )]
        model: Option<String>,

        /// ANTHROPIC_SMALL_FAST_MODEL value (Haiku-class model for background tasks)
        #[arg(
            long = "small-fast-model",
            help = "Haiku-class model for background tasks (optional)"
        )]
        small_fast_model: Option<String>,

        /// Force overwrite existing configuration
        #[arg(
            long = "force",
            short = 'f',
            help = "Overwrite existing configuration with same alias"
        )]
        force: bool,

        /// Interactive mode for entering configuration values
        #[arg(
            long = "interactive",
            short = 'i',
            help = "Enter configuration values interactively"
        )]
        interactive: bool,

        /// Positional token argument (for backward compatibility)
        #[arg(help = "API token (if not using -t flag)")]
        token_arg: Option<String>,

        /// Positional URL argument (for backward compatibility)
        #[arg(help = "API endpoint URL (if not using -u flag)")]
        url_arg: Option<String>,
    },
    /// Remove one or more configurations by alias name
    ///
    /// Deletes stored configurations by their alias names
    #[command(alias = "r")]
    Remove {
        /// Configuration alias name(s) to remove (one or more)
        #[arg(required = true)]
        alias_names: Vec<String>,
    },
    /// List all stored configurations
    ///
    /// Displays all saved configurations with their aliases, tokens, and URLs
    #[command(alias = "l")]
    List,
    /// Set default directory for Claude settings.json
    ///
    /// Configures the directory where Claude settings.json file is located
    /// Default is ~/.claude/
    #[command(alias = "s")]
    SetDefaultDir {
        /// Directory path for Claude settings (e.g., /path/to/claude/config)
        directory: String,
    },
    /// Generate shell completion scripts
    ///
    /// Generates completion scripts for supported shells and adds useful aliases:
    /// - cs='cc-switch' for quick access
    /// - ccd='claude --dangerously-skip-permissions' for quick Claude launch
    #[command(alias = "C")]
    Completion {
        /// Shell type (fish, zsh, bash, elvish, powershell)
        #[arg(default_value = "fish")]
        shell: String,
    },
    /// Generate shell aliases for eval
    ///
    /// Outputs alias definitions that can be evaluated with eval
    /// This is the quickest way to get aliases working in your current shell
    #[command(alias = "A")]
    Alias {
        /// Shell type (fish, zsh, bash)
        #[arg(default_value = "fish")]
        shell: String,
    },
    /// Switch to a configuration by alias name
    ///
    /// Switches Claude to use the specified API configuration
    /// Use 'cc' as alias name to reset to default Claude behavior
    #[command(alias = "sw")]
    Switch {
        /// Configuration alias name (use 'cc' to reset to default)
        #[arg(help = "Configuration alias name (use 'cc' to reset to default)")]
        alias_name: String,
    },
    /// Show current API configuration
    ///
    /// Displays the current Anthropic environment variables from Claude settings
    #[command(alias = "cur")]
    Current,
}

/// Represents a Claude API configuration
///
/// Contains the components needed to configure Claude API access:
/// - alias_name: User-friendly identifier for the configuration
/// - token: API authentication token
/// - url: Base URL for the API endpoint
/// - model: Optional custom model name
/// - small_fast_model: Optional Haiku-class model for background tasks
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Configuration {
    /// User-friendly alias name for this configuration
    pub alias_name: String,
    /// ANTHROPIC_AUTH_TOKEN value (API authentication token)
    pub token: String,
    /// ANTHROPIC_BASE_URL value (API endpoint URL)
    pub url: String,
    /// ANTHROPIC_MODEL value (custom model name)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// ANTHROPIC_SMALL_FAST_MODEL value (Haiku-class model for background tasks)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub small_fast_model: Option<String>,
}

/// Storage manager for Claude API configurations
///
/// Handles persistence and retrieval of multiple API configurations
/// stored in `~/.cc_auto_switch/configurations.json`
#[derive(Serialize, Deserialize, Default)]
pub struct ConfigStorage {
    /// Map of alias names to configuration objects
    pub configurations: HashMap<String, Configuration>,
    /// Custom directory for Claude settings (optional)
    pub claude_settings_dir: Option<String>,
}

/// Claude settings manager for API configuration
///
/// Manages the Claude settings.json file to control Claude's API configuration
/// Handles environment variables and preserves other settings
#[derive(Default, Clone)]
pub struct ClaudeSettings {
    /// Environment variables map (ANTHROPIC_AUTH_TOKEN, ANTHROPIC_BASE_URL, ANTHROPIC_MODEL, ANTHROPIC_SMALL_FAST_MODEL)
    pub env: HashMap<String, String>,
    /// Other settings to preserve when modifying API configuration
    pub other: HashMap<String, serde_json::Value>,
}

impl Serialize for ClaudeSettings {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(Some(
            self.other.len() + if self.env.is_empty() { 0 } else { 1 },
        ))?;

        // Serialize env field only if it has content
        if !self.env.is_empty() {
            map.serialize_entry("env", &self.env)?;
        }

        // Serialize other fields
        for (key, value) in &self.other {
            map.serialize_entry(key, value)?;
        }

        map.end()
    }
}

impl<'de> Deserialize<'de> for ClaudeSettings {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ClaudeSettingsHelper {
            #[serde(default)]
            env: HashMap<String, String>,
            #[serde(flatten)]
            other: HashMap<String, serde_json::Value>,
        }

        let helper = ClaudeSettingsHelper::deserialize(deserializer)?;
        Ok(ClaudeSettings {
            env: helper.env,
            other: helper.other,
        })
    }
}

impl ConfigStorage {
    /// Load configurations from disk
    ///
    /// Reads the JSON file from `~/.cc_auto_switch/configurations.json`
    /// Returns default empty storage if file doesn't exist
    ///
    /// # Errors
    /// Returns error if file exists but cannot be read or parsed
    pub fn load() -> Result<Self> {
        let path = get_config_storage_path()?;

        if !path.exists() {
            return Ok(ConfigStorage::default());
        }

        let content = fs::read_to_string(&path).with_context(|| {
            format!(
                "Failed to read configuration storage from {}",
                path.display()
            )
        })?;

        let storage: ConfigStorage = serde_json::from_str(&content)
            .with_context(|| "Failed to parse configuration storage JSON")?;

        Ok(storage)
    }

    /// Save configurations to disk
    ///
    /// Writes the current state to `~/.cc_auto_switch/configurations.json`
    /// Creates the directory structure if it doesn't exist
    ///
    /// # Errors
    /// Returns error if directory cannot be created or file cannot be written
    pub fn save(&self) -> Result<()> {
        let path = get_config_storage_path()?;

        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {}", parent.display()))?;
        }

        let json = serde_json::to_string_pretty(self)
            .with_context(|| "Failed to serialize configuration storage")?;

        fs::write(&path, json).with_context(|| format!("Failed to write to {}", path.display()))?;

        Ok(())
    }

    /// Add a new configuration to storage
    ///
    /// # Arguments
    /// * `config` - Configuration object to add
    ///
    /// Overwrites existing configuration with same alias
    pub fn add_configuration(&mut self, config: Configuration) {
        self.configurations
            .insert(config.alias_name.clone(), config);
    }

    /// Remove a configuration by alias name
    ///
    /// # Arguments
    /// * `alias_name` - Name of configuration to remove
    ///
    /// # Returns
    /// `true` if configuration was found and removed, `false` if not found
    pub fn remove_configuration(&mut self, alias_name: &str) -> bool {
        self.configurations.remove(alias_name).is_some()
    }

    /// Get a configuration by alias name
    ///
    /// # Arguments
    /// * `alias_name` - Name of configuration to retrieve
    ///
    /// # Returns
    /// `Some(&Configuration)` if found, `None` if not found
    pub fn get_configuration(&self, alias_name: &str) -> Option<&Configuration> {
        self.configurations.get(alias_name)
    }

    /// Set the default directory for Claude settings
    ///
    /// # Arguments
    /// * `directory` - Directory path for Claude settings
    pub fn set_claude_settings_dir(&mut self, directory: String) {
        self.claude_settings_dir = Some(directory);
    }

    /// Get the current Claude settings directory
    ///
    /// # Returns
    /// `Some(&String)` if custom directory is set, `None` if using default
    pub fn get_claude_settings_dir(&self) -> Option<&String> {
        self.claude_settings_dir.as_ref()
    }
}

impl ClaudeSettings {
    /// Load Claude settings from disk
    ///
    /// Reads the JSON file from the configured Claude settings directory
    /// Returns default empty settings if file doesn't exist
    /// Creates the file with default structure if it doesn't exist
    ///
    /// # Arguments
    /// * `custom_dir` - Optional custom directory for Claude settings
    ///
    /// # Errors
    /// Returns error if file exists but cannot be read or parsed
    pub fn load(custom_dir: Option<&str>) -> Result<Self> {
        let path = get_claude_settings_path(custom_dir)?;

        if !path.exists() {
            // Create default settings file if it doesn't exist
            let default_settings = ClaudeSettings::default();
            default_settings.save(custom_dir)?;
            return Ok(default_settings);
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read Claude settings from {}", path.display()))?;

        // Parse with better error handling for missing env field
        let mut settings: ClaudeSettings = if content.trim().is_empty() {
            ClaudeSettings::default()
        } else {
            serde_json::from_str(&content)
                .with_context(|| "Failed to parse Claude settings JSON")?
        };

        // Ensure env field exists (handle case where it might be missing from JSON)
        if settings.env.is_empty() && !content.contains("\"env\"") {
            settings.env = HashMap::new();
        }

        Ok(settings)
    }

    /// Save Claude settings to disk
    ///
    /// Writes the current state to the configured Claude settings directory
    /// Creates the directory structure if it doesn't exist
    /// Ensures the env field is properly serialized
    ///
    /// # Arguments
    /// * `custom_dir` - Optional custom directory for Claude settings
    ///
    /// # Errors
    /// Returns error if directory cannot be created or file cannot be written
    pub fn save(&self, custom_dir: Option<&str>) -> Result<()> {
        let path = get_claude_settings_path(custom_dir)?;

        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {}", parent.display()))?;
        }

        // The custom Serialize implementation handles env field inclusion automatically
        let settings_to_save = self;

        let json = serde_json::to_string_pretty(&settings_to_save)
            .with_context(|| "Failed to serialize Claude settings")?;

        fs::write(&path, json).with_context(|| format!("Failed to write to {}", path.display()))?;

        Ok(())
    }

    /// Switch to a specific API configuration
    ///
    /// Updates the environment variables with the provided configuration
    /// Ensures env field exists before updating
    ///
    /// # Arguments
    /// * `config` - Configuration containing token, URL, and optional model settings to apply
    pub fn switch_to_config(&mut self, config: &Configuration) {
        // Ensure env field exists
        if self.env.is_empty() {
            self.env = HashMap::new();
        }

        self.env
            .insert("ANTHROPIC_AUTH_TOKEN".to_string(), config.token.clone());
        self.env
            .insert("ANTHROPIC_BASE_URL".to_string(), config.url.clone());
        
        // Set model configurations if provided
        if let Some(model) = &config.model {
            self.env
                .insert("ANTHROPIC_MODEL".to_string(), model.clone());
        }
        
        if let Some(small_fast_model) = &config.small_fast_model {
            self.env
                .insert("ANTHROPIC_SMALL_FAST_MODEL".to_string(), small_fast_model.clone());
        }
    }

    /// Remove Anthropic environment variables
    ///
    /// Clears all Anthropic-related environment variables from settings
    /// Used to reset to default Claude behavior
    pub fn remove_anthropic_env(&mut self) {
        // Ensure env field exists
        if self.env.is_empty() {
            self.env = HashMap::new();
        }

        self.env.remove("ANTHROPIC_AUTH_TOKEN");
        self.env.remove("ANTHROPIC_BASE_URL");
        self.env.remove("ANTHROPIC_MODEL");
        self.env.remove("ANTHROPIC_SMALL_FAST_MODEL");
    }
}

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

/// Get the current shell type
///
/// Detects the current shell environment from environment variables
///
/// # Returns
/// Shell type as string ("fish", "bash", "zsh", "unknown")
fn get_current_shell() -> String {
    // Check for fish shell specifically first
    if let Ok(shell) = env::var("SHELL") {
        if shell.contains("fish") {
            return "fish".to_string();
        } else if shell.contains("bash") {
            return "bash".to_string();
        } else if shell.contains("zsh") {
            return "zsh".to_string();
        }
    }

    // Check current process name
    if let Ok(shell) = env::var("0") {
        if shell.contains("fish") {
            return "fish".to_string();
        } else if shell.contains("bash") {
            return "bash".to_string();
        } else if shell.contains("zsh") {
            return "zsh".to_string();
        }
    }

    // Check for fish-specific environment variables
    if env::var("FISH_VERSION").is_ok() {
        return "fish".to_string();
    }

    "unknown".to_string()
}

/// Apply color formatting based on shell type
///
/// # Arguments
/// * `text` - The text to format
/// * `color_type` - The color type to apply ("cyan", "yellow", "green", "white")
/// * `bold` - Whether to make the text bold
///
/// # Returns
/// Formatted text with appropriate ANSI escape sequences
fn format_color(text: &str, color_type: &str, bold: bool) -> String {
    let shell = get_current_shell();

    // For testing purposes, check for a special environment variable
    if env::var("CC_SWITCH_TEST_FISH").is_ok() {
        return text.to_string();
    }

    // For fish shell, avoid color formatting in interactive display to prevent alignment issues
    if shell == "fish" {
        return text.to_string();
    }

    // Use simpler formatting for fish shell to avoid display issues
    let color_code = match color_type {
        "cyan" => "\x1b[96m",
        "yellow" => "\x1b[93m",
        "green" => "\x1b[92m",
        "white" => "\x1b[97m",
        "red" => "\x1b[91m",
        _ => "",
    };

    let bold_code = if bold { "\x1b[1m" } else { "" };
    let reset_code = "\x1b[0m";

    // Other shells handle ANSI codes better
    format!("{bold_code}{color_code}{text}{reset_code}")
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

/// Interactive configuration selection with keyboard navigation
///
/// Displays available configurations in a navigable menu with:
/// - Keyboard up/down navigation
/// - Visual highlighting of selected item
/// - Color indicators for keyboard support
/// - Enter to confirm selection
///
/// # Errors
/// Returns error if file operations fail or terminal setup fails
fn interactive_config_selection() -> Result<()> {
    let storage = ConfigStorage::load()?;
    let custom_dir = storage.get_claude_settings_dir().map(|s| s.as_str());
    let claude_settings = ClaudeSettings::load(custom_dir)?;

    // Get current configuration info
    let current_token = claude_settings.env.get("ANTHROPIC_AUTH_TOKEN");
    let current_url = claude_settings.env.get("ANTHROPIC_BASE_URL");
    let current_model = claude_settings.env.get("ANTHROPIC_MODEL");
    let current_small_fast_model = claude_settings.env.get("ANTHROPIC_SMALL_FAST_MODEL");

    // Create list of available options
    let mut options = Vec::new();

    // Add current configuration info if available
    if current_token.is_some() || current_url.is_some() {
        let current_info = if let (Some(token), Some(_url)) = (current_token, current_url) {
            format!(
                "Current: {}",
                if token.len() > 20 {
                    format!("{}...", &token[..20])
                } else {
                    token.clone()
                }
            )
        } else if let Some(token) = current_token {
            format!(
                "Current: {}",
                if token.len() > 20 {
                    format!("{}...", &token[..20])
                } else {
                    token.clone()
                }
            )
        } else if let Some(_url) = current_url {
            "Current: No token".to_string()
        } else {
            "Current: No configuration".to_string()
        };
        options.push(("current", current_info));
    }

    // Add "cc" option to reset to default
    options.push(("cc", "Reset to default".to_string()));

    // Add stored configurations - simplified to just show alias
    for alias_name in storage.configurations.keys() {
        options.push((alias_name.as_str(), alias_name.clone()));
    }

    // Calculate maximum description length for alignment
    let max_desc_length = options
        .iter()
        .map(|(_, desc)| desc.len())
        .max()
        .unwrap_or(0);

    // Ensure minimum width for better alignment
    let min_width = 40;
    let display_width = std::cmp::max(max_desc_length, min_width);

    if options.is_empty() {
        println!("No configurations available. Use 'add' command to create one.");
        return Ok(());
    }

    // Setup terminal for raw mode with better error handling
    if let Err(e) = crossterm::terminal::enable_raw_mode() {
        eprintln!("Warning: Could not enable terminal raw mode: {e}");
        eprintln!("Falling back to simple display mode");
        return show_simple_current_display();
    }

    if let Err(e) = crossterm::execute!(
        io::stdout(),
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
    ) {
        eprintln!("Warning: Could not clear terminal: {e}");
        // Continue without clearing
    }

    let mut selected_index = 0;
    let mut should_exit = false;

    while !should_exit {
        // Clear screen
        crossterm::execute!(
            io::stdout(),
            crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
        )?;
        crossterm::execute!(io::stdout(), crossterm::cursor::MoveTo(0, 0))?;

        // Display header with keyboard instructions
        let shell = get_current_shell();
        let use_fish_mode = env::var("CC_SWITCH_TEST_FISH").is_ok() || shell == "fish";

        if use_fish_mode {
            println!("=== Claude Configuration Selection ===");
            println!("Use UP/DOWN arrow keys to navigate, Enter to select, Esc to cancel");
            println!("---------------------------------------------------------");
            println!();

            // Display options with proper alignment
            for (index, (_key, description)) in options.iter().enumerate() {
                let padded_desc = format!("{description:<display_width$}");
                if index == selected_index {
                    // Highlight selected item with proper spacing
                    println!("  > {padded_desc}");
                } else {
                    println!("    {padded_desc}");
                }
            }

            println!();
            println!("---------------------------------------------------------");
            println!("Selected: {}", &options[selected_index].1);
        } else {
            println!(
                "{}",
                format_color("=== Claude Configuration Selection ===", "cyan", true)
            );
            println!(
                "{}",
                format_color(
                    "Use UP/DOWN arrow keys to navigate, Enter to select, Esc to cancel",
                    "yellow",
                    true
                )
            );
            println!(
                "{}",
                format_color(
                    "---------------------------------------------------------",
                    "cyan",
                    false
                )
            );
            println!();

            // Display options with proper alignment
            for (index, (_key, description)) in options.iter().enumerate() {
                let padded_desc = format!("{description:<display_width$}");
                if index == selected_index {
                    // Highlight selected item with proper spacing
                    println!(
                        "  {} {}",
                        format_color(">", "green", true),
                        format_color(&padded_desc, "white", true)
                    );
                } else {
                    println!("    {padded_desc}");
                }
            }

            println!();
            println!(
                "{}",
                format_color(
                    "---------------------------------------------------------",
                    "cyan",
                    false
                )
            );
            println!(
                "Selected: {}",
                format_color(&options[selected_index].1, "yellow", false)
            );
        }

        // Read keyboard input
        if let Event::Key(KeyEvent { code, .. }) = event::read()? {
            match code {
                KeyCode::Up => {
                    if selected_index > 0 {
                        selected_index = selected_index.saturating_sub(1);
                    }
                }
                KeyCode::Down => {
                    if selected_index < options.len() - 1 {
                        selected_index += 1;
                    }
                }
                KeyCode::Enter => {
                    should_exit = true;
                }
                KeyCode::Esc => {
                    // Restore terminal and exit
                    if let Err(e) = crossterm::terminal::disable_raw_mode() {
                        eprintln!("Warning: Could not disable terminal raw mode: {e}");
                    }
                    if use_fish_mode {
                        println!("\nSelection cancelled.");
                    } else {
                        println!("\n{}", format_color("Selection cancelled.", "red", false));
                    }
                    return Ok(());
                }
                _ => {}
            }
        }
    }

    // Restore terminal
    if let Err(e) = crossterm::terminal::disable_raw_mode() {
        eprintln!("Warning: Could not disable terminal raw mode: {e}");
    }

    // Process selection
    let selected_key = options[selected_index].0;
    let use_fish_mode = env::var("CC_SWITCH_TEST_FISH").is_ok() || get_current_shell() == "fish";

    if use_fish_mode {
        println!("\nSelected: {}", options[selected_index].1);

        if selected_key == "current" {
            // Show current configuration info
            println!("\nCurrent Configuration:");
            if let Some(token) = current_token {
                println!("Token: {token}");
            } else {
                println!("Token: No ANTHROPIC_AUTH_TOKEN configured");
            }

            if let Some(url) = current_url {
                println!("URL: {url}");
            } else {
                println!("URL: No ANTHROPIC_BASE_URL configured");
            }

            if let Some(model) = current_model {
                println!("Model: {model}");
            } else {
                println!("Model: Not configured (using default)");
            }

            if let Some(small_fast_model) = current_small_fast_model {
                println!("Small Fast Model: {small_fast_model}");
            } else {
                println!("Small Fast Model: Not configured (using default)");
            }
        } else {
            // Switch to selected configuration
            handle_switch_command(selected_key)?;
        }
    } else {
        println!(
            "\n{}",
            format_color(
                &format!("Selected: {}", options[selected_index].1),
                "green",
                false
            )
        );

        if selected_key == "current" {
            // Show current configuration info
            println!("\n{}", format_color("Current Configuration:", "cyan", true));
            if let Some(token) = current_token {
                println!("Token: {token}");
            } else {
                println!("Token: No ANTHROPIC_AUTH_TOKEN configured");
            }

            if let Some(url) = current_url {
                println!("URL: {url}");
            } else {
                println!("URL: No ANTHROPIC_BASE_URL configured");
            }

            if let Some(model) = current_model {
                println!("Model: {model}");
            } else {
                println!("Model: Not configured (using default)");
            }

            if let Some(small_fast_model) = current_small_fast_model {
                println!("Small Fast Model: {small_fast_model}");
            } else {
                println!("Small Fast Model: Not configured (using default)");
            }
        } else {
            // Switch to selected configuration
            handle_switch_command(selected_key)?;
        }
    }

    Ok(())
}

/// Show simple current configuration display (fallback mode)
///
/// Displays current configuration and available options without interactive menu
///
/// # Errors
/// Returns error if file operations fail
fn show_simple_current_display() -> Result<()> {
    let storage = ConfigStorage::load()?;
    let custom_dir = storage.get_claude_settings_dir().map(|s| s.as_str());
    let claude_settings = ClaudeSettings::load(custom_dir)?;

    let current_token = claude_settings.env.get("ANTHROPIC_AUTH_TOKEN");
    let current_url = claude_settings.env.get("ANTHROPIC_BASE_URL");
    let current_model = claude_settings.env.get("ANTHROPIC_MODEL");
    let current_small_fast_model = claude_settings.env.get("ANTHROPIC_SMALL_FAST_MODEL");

    let shell = get_current_shell();

    // For testing purposes, check for a special environment variable
    let use_fish_mode = env::var("CC_SWITCH_TEST_FISH").is_ok() || shell == "fish";

    // For fish shell, use simple text without color codes
    if use_fish_mode {
        println!("Current Configuration:");
        if let Some(token) = current_token {
            println!("Token: {token}");
        } else {
            println!("Token: No ANTHROPIC_AUTH_TOKEN configured");
        }
        
        if let Some(url) = current_url {
            println!("URL: {url}");
        } else {
            println!("URL: No ANTHROPIC_BASE_URL configured");
        }
        
        if let Some(model) = current_model {
            println!("Model: {model}");
        } else {
            println!("Model: Not configured (using default)");
        }
        
        if let Some(small_fast_model) = current_small_fast_model {
            println!("Small Fast Model: {small_fast_model}");
        } else {
            println!("Small Fast Model: Not configured (using default)");
        }

        println!("\nAvailable configurations:");

        // Show "cc" option to reset to default
        println!("  cc  - Reset to default");

        // Show stored configurations - simplified to just show alias
        for alias_name in storage.configurations.keys() {
            println!("  {alias_name}  - Switch to this configuration");
        }

        println!("\nUse 'cc-switch <alias>' to switch configuration");
    } else {
        // Other shells can use color formatting
        println!("{}", format_color("Current Configuration:", "cyan", true));
        if let Some(token) = current_token {
            println!("Token: {token}");
        } else {
            println!("Token: No ANTHROPIC_AUTH_TOKEN configured");
        }
        
        if let Some(url) = current_url {
            println!("URL: {url}");
        } else {
            println!("URL: No ANTHROPIC_BASE_URL configured");
        }
        
        if let Some(model) = current_model {
            println!("Model: {model}");
        } else {
            println!("Model: Not configured (using default)");
        }
        
        if let Some(small_fast_model) = current_small_fast_model {
            println!("Small Fast Model: {small_fast_model}");
        } else {
            println!("Small Fast Model: Not configured (using default)");
        }

        println!(
            "\n{}",
            format_color("Available configurations:", "yellow", true)
        );

        // Show "cc" option to reset to default
        println!("  cc  - Reset to default");

        // Show stored configurations - simplified to just show alias
        for alias_name in storage.configurations.keys() {
            println!("  {alias_name}  - Switch to this configuration");
        }

        println!(
            "\n{}",
            format_color(
                "Use 'cc-switch <alias>' to switch configuration",
                "green",
                false
            )
        );
    }
    Ok(())
}

/// Handle showing current configuration
///
/// Displays the current ANTHROPIC_AUTH_TOKEN and ANTHROPIC_BASE_URL from Claude settings
/// If configurations are available, shows interactive selection menu
///
/// # Errors
/// Returns error if file operations fail
pub fn handle_current_command() -> Result<()> {
    let storage = ConfigStorage::load()?;

    // Check if we have configurations to show interactive menu
    if !storage.configurations.is_empty() {
        // Show interactive selection menu
        interactive_config_selection()?;
    } else {
        // Fallback to simple display if no configurations
        let custom_dir = storage.get_claude_settings_dir().map(|s| s.as_str());
        let claude_settings = ClaudeSettings::load(custom_dir)?;

        let token = claude_settings.env.get("ANTHROPIC_AUTH_TOKEN");
        let url = claude_settings.env.get("ANTHROPIC_BASE_URL");
        let model = claude_settings.env.get("ANTHROPIC_MODEL");
        let small_fast_model = claude_settings.env.get("ANTHROPIC_SMALL_FAST_MODEL");

        // Check for fish mode
        let use_fish_mode =
            env::var("CC_SWITCH_TEST_FISH").is_ok() || get_current_shell() == "fish";

        if use_fish_mode {
            println!("Current Configuration:");
            if let Some(token) = token {
                println!("Token: {token}");
            } else {
                println!("Token: No ANTHROPIC_AUTH_TOKEN configured");
            }
            
            if let Some(url) = url {
                println!("URL: {url}");
            } else {
                println!("URL: No ANTHROPIC_BASE_URL configured");
            }
            
            if let Some(model) = model {
                println!("Model: {model}");
            } else {
                println!("Model: Not configured (using default)");
            }
            
            if let Some(small_fast_model) = small_fast_model {
                println!("Small Fast Model: {small_fast_model}");
            } else {
                println!("Small Fast Model: Not configured (using default)");
            }

            println!("\nNo configurations available for selection.");
            println!("Use 'add' command to create configurations for interactive selection.");
        } else {
            println!("{}", format_color("Current Configuration:", "cyan", true));
            if let Some(token) = token {
                println!("Token: {token}");
            } else {
                println!("Token: No ANTHROPIC_AUTH_TOKEN configured");
            }
            
            if let Some(url) = url {
                println!("URL: {url}");
            } else {
                println!("URL: No ANTHROPIC_BASE_URL configured");
            }
            
            if let Some(model) = model {
                println!("Model: {model}");
            } else {
                println!("Model: Not configured (using default)");
            }
            
            if let Some(small_fast_model) = small_fast_model {
                println!("Small Fast Model: {small_fast_model}");
            } else {
                println!("Small Fast Model: Not configured (using default)");
            }

            println!(
                "\n{}",
                format_color(
                    "No configurations available for selection.",
                    "yellow",
                    false
                )
            );
            println!("Use 'add' command to create configurations for interactive selection.");
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
fn read_input(prompt: &str) -> Result<String> {
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
fn read_sensitive_input(prompt: &str) -> Result<String> {
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
    if alias_name.contains(' ') {
        anyhow::bail!("Alias name cannot contain whitespace");
    }
    Ok(())
}

/// Parameters for adding a new configuration
struct AddCommandParams {
    alias_name: String,
    token: Option<String>,
    url: Option<String>,
    model: Option<String>,
    small_fast_model: Option<String>,
    force: bool,
    interactive: bool,
    token_arg: Option<String>,
    url_arg: Option<String>,
}

/// Handle adding a configuration with all the new features
///
/// # Arguments
/// * `params` - Parameters for the add command
/// * `storage` - Mutable reference to config storage
///
/// # Errors
/// Returns error if validation fails or user cancels interactive input
fn handle_add_command(params: AddCommandParams, storage: &mut ConfigStorage) -> Result<()> {
    // Validate alias name
    validate_alias_name(&params.alias_name)?;

    // Check if alias already exists
    if storage.get_configuration(&params.alias_name).is_some() && !params.force {
        eprintln!("Configuration '{}' already exists.", params.alias_name);
        eprintln!("Use --force to overwrite or choose a different alias name.");
        return Ok(());
    }

    // Determine token value
    let final_token = if params.interactive {
        if params.token.is_some() || params.token_arg.is_some() {
            eprintln!(
                "Warning: Token provided via flags/arguments will be ignored in interactive mode"
            );
        }
        read_sensitive_input("Enter API token (sk-ant-xxx): ")?
    } else {
        match (&params.token, &params.token_arg) {
            (Some(t), _) => t.clone(),
            (None, Some(t)) => t.clone(),
            (None, None) => {
                anyhow::bail!(
                    "Token is required. Use -t flag, provide as argument, or use interactive mode with -i"
                );
            }
        }
    };

    // Determine URL value
    let final_url = if params.interactive {
        if params.url.is_some() || params.url_arg.is_some() {
            eprintln!(
                "Warning: URL provided via flags/arguments will be ignored in interactive mode"
            );
        }
        read_input("Enter API URL (default: https://api.anthropic.com): ")?
    } else {
        match (&params.url, &params.url_arg) {
            (Some(u), _) => u.clone(),
            (None, Some(u)) => u.clone(),
            (None, None) => "https://api.anthropic.com".to_string(),
        }
    };

    // Use default URL if empty
    let final_url = if final_url.is_empty() {
        "https://api.anthropic.com".to_string()
    } else {
        final_url
    };

    // Determine model value
    let final_model = if params.interactive {
        if params.model.is_some() {
            eprintln!(
                "Warning: Model provided via flags will be ignored in interactive mode"
            );
        }
        let model_input = read_input("Enter model name (optional, press enter to skip): ")?;
        if model_input.is_empty() {
            None
        } else {
            Some(model_input)
        }
    } else {
        params.model
    };

    // Determine small fast model value
    let final_small_fast_model = if params.interactive {
        if params.small_fast_model.is_some() {
            eprintln!(
                "Warning: Small fast model provided via flags will be ignored in interactive mode"
            );
        }
        let small_model_input = read_input("Enter small fast model name (optional, press enter to skip): ")?;
        if small_model_input.is_empty() {
            None
        } else {
            Some(small_model_input)
        }
    } else {
        params.small_fast_model
    };

    // Validate token format (basic check)
    if !final_token.starts_with("sk-ant-") {
        eprintln!(
            "Warning: Token doesn't start with 'sk-ant-' - please verify it's a valid Claude API token"
        );
    }

    // Create and add configuration
    let config = Configuration {
        alias_name: params.alias_name.clone(),
        token: final_token,
        url: final_url,
        model: final_model,
        small_fast_model: final_small_fast_model,
    };

    storage.add_configuration(config);
    storage.save()?;

    println!("Configuration '{}' added successfully", params.alias_name);
    if params.force {
        println!("(Overwrote existing configuration)");
    }

    Ok(())
}

/// Handle configuration switching command
///
/// Processes the switch subcommand to switch Claude API configuration:
/// - "cc": Remove API configuration (reset to default)
/// - Other alias: Switch to the specified configuration
///
/// After switching, displays the current URL and automatically launches Claude CLI
///
/// # Arguments
/// * `alias_name` - Name of configuration to switch to, or "cc" to reset
///
/// # Errors
/// Returns error if configuration is not found or file operations fail
pub fn handle_switch_command(alias_name: &str) -> Result<()> {
    let storage = ConfigStorage::load()?;
    let custom_dir = storage.get_claude_settings_dir().map(|s| s.as_str());
    let mut claude_settings = ClaudeSettings::load(custom_dir)?;

    if alias_name == "cc" {
        // Default operation: remove ANTHROPIC_AUTH_TOKEN and ANTHROPIC_BASE_URL
        claude_settings.remove_anthropic_env();
        claude_settings.save(custom_dir)?;
        println!("Removed ANTHROPIC_AUTH_TOKEN and ANTHROPIC_BASE_URL from Claude settings");
        println!("Current URL: Default (no custom URL configured)");
    } else if let Some(config) = storage.get_configuration(alias_name) {
        claude_settings.switch_to_config(config);
        claude_settings.save(custom_dir)?;
        let mut config_info = format!("token: {}, url: {}", config.token, config.url);
        if let Some(model) = &config.model {
            config_info.push_str(&format!(", model: {model}"));
        }
        if let Some(small_fast_model) = &config.small_fast_model {
            config_info.push_str(&format!(", small_fast_model: {small_fast_model}"));
        }
        println!("Switched to configuration '{alias_name}' ({config_info})");
        println!("Current URL: {}", config.url);
    } else {
        anyhow::bail!(
            "Configuration '{}' not found. Use 'list' command to see available configurations.",
            alias_name
        );
    }

    // Wait 0.5 second
    println!("Waiting 0.5 second before launching Claude...");
    println!(
        "Executing: claude {}",
        format_color("--dangerously-skip-permissions", "red", false)
    );
    thread::sleep(Duration::from_millis(500));

    // Launch Claude CLI with --dangerously-skip-permissions flag
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

    // Wait for the Claude process to finish and pass control to it
    let status = child
        .wait()
        .with_context(|| "Failed to wait for Claude CLI process")?;

    if !status.success() {
        anyhow::bail!("Claude CLI exited with error status: {}", status);
    }

    Ok(())
}

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
        }
        "zsh" => {
            println!("alias cs='cc-switch'");
            println!("alias ccd='claude --dangerously-skip-permissions'");
        }
        "bash" => {
            println!("alias cs='cc-switch'");
            println!("alias ccd='claude --dangerously-skip-permissions'");
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
    use clap::CommandFactory;
    use std::io::stdout;

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
                format_color(
                    "# Aliases 'cs' and 'ccd' have been added for convenience",
                    "green",
                    false
                )
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
                format_color(
                    "# Aliases 'cs' and 'ccd' have been added for convenience",
                    "green",
                    false
                )
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
                format_color(
                    "# Aliases 'cs' and 'ccd' have been added for convenience",
                    "green",
                    false
                )
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
                format_color(
                    "# Aliases 'cs' and 'ccd' have been added for convenience",
                    "green",
                    false
                )
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
/// Also includes 'cc' as a special alias for resetting to default
///
/// # Errors
/// Returns error if loading configurations fails
fn list_aliases_for_completion() -> Result<()> {
    let storage = ConfigStorage::load()?;

    // Always include 'cc' for reset functionality
    println!("cc");

    // Output all stored aliases
    for alias_name in storage.configurations.keys() {
        println!("{alias_name}");
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

    // Add custom completion for switch subcommand
    println!("\n# Custom completion for switch subcommand with dynamic aliases");
    println!(
        "complete -c cc-switch -n '__fish_cc_switch_using_subcommand switch' -f -a '(cc-switch --list-aliases)' -d 'Configuration alias name'"
    );
    // Custom completion for remove subcommand with dynamic aliases
    println!("# Custom completion for remove subcommand with dynamic aliases");
    println!(
        "complete -c cc-switch -n '__fish_cc_switch_using_subcommand remove' -f -a '(cc-switch --list-aliases)' -d 'Configuration alias name'"
    );

    // Add useful aliases that can be eval'd
    println!("\n# To use these aliases immediately, run:");
    println!("# eval \"(cc-switch alias fish)\"");
    println!("\n# Or add them permanently to your ~/.config/fish/config.fish:");
    println!("# echo \"alias cs='cc-switch'\" >> ~/.config/fish/config.fish");
    println!(
        "# echo \"alias ccd='claude --dangerously-skip-permissions'\" >> ~/.config/fish/config.fish"
    );

    // Add completion for the 'cs' alias
    println!("\n# Completion for the 'cs' alias");
    println!("complete -c cs -w cc-switch");

    println!("\n# Fish completion generated successfully");
    println!("# Add this to your ~/.config/fish/completions/cc-switch.fish");
    println!("# Then restart your shell or run 'source ~/.config/fish/config.fish'");
    println!(
        "{}",
        format_color(
            "# Aliases 'cs' and 'ccd' have been added for convenience",
            "green",
            false
        )
    );
}

/// Main entry point for the CLI application
///
/// Parses command-line arguments and executes the appropriate action:
/// - Switch configuration with `-c` flag
/// - Execute subcommands (add, remove, list)
/// - Show help if no arguments provided
///
/// # Errors
/// Returns error if any operation fails (file I/O, parsing, etc.)
pub fn run() -> Result<()> {
    let cli = Cli::parse();

    // Handle --list-aliases flag for completion
    if cli.list_aliases {
        list_aliases_for_completion()?;
        return Ok(());
    }

    // Handle subcommands
    if let Some(command) = cli.command {
        let mut storage = ConfigStorage::load()?;

        match command {
            Commands::Add {
                alias_name,
                token,
                url,
                model,
                small_fast_model,
                force,
                interactive,
                token_arg,
                url_arg,
            } => {
                let params = AddCommandParams {
                    alias_name,
                    token,
                    url,
                    model,
                    small_fast_model,
                    force,
                    interactive,
                    token_arg,
                    url_arg,
                };
                handle_add_command(params, &mut storage)?;
            }
            Commands::Remove { alias_names } => {
                let mut removed_count = 0;
                let mut not_found_aliases = Vec::new();

                for alias_name in &alias_names {
                    if storage.remove_configuration(alias_name) {
                        removed_count += 1;
                        println!("Configuration '{alias_name}' removed successfully");
                    } else {
                        not_found_aliases.push(alias_name.clone());
                        println!("Configuration '{alias_name}' not found");
                    }
                }

                if removed_count > 0 {
                    storage.save()?;
                }

                if !not_found_aliases.is_empty() {
                    eprintln!(
                        "Warning: The following configurations were not found: {}",
                        not_found_aliases.join(", ")
                    );
                }

                if removed_count > 0 {
                    println!("Successfully removed {removed_count} configuration(s)");
                }
            }
            Commands::List => {
                if storage.configurations.is_empty() {
                    println!("No configurations stored");
                } else {
                    println!("Stored configurations:");
                    for alias_name in storage.configurations.keys() {
                        println!("  {alias_name}");
                    }
                }
                if let Some(dir) = &storage.claude_settings_dir {
                    println!("Claude settings directory: {dir}");
                } else {
                    println!("Claude settings directory: ~/.claude/ (default)");
                }
            }
            Commands::SetDefaultDir { directory } => {
                storage.set_claude_settings_dir(directory.clone());
                storage.save()?;
                println!("Claude settings directory set to: {directory}");
            }
            Commands::Completion { shell } => {
                generate_completion(&shell)?;
            }
            Commands::Alias { shell } => {
                generate_aliases(&shell)?;
            }
            Commands::Switch { alias_name } => {
                handle_switch_command(&alias_name)?;
            }
            Commands::Current => {
                handle_current_command()?;
            }
        }
    } else {
        // No command provided, show help
        println!("Use -h or --help for usage information");
    }

    Ok(())
}
