use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Command-line interface for managing Claude API configurations
#[derive(Parser)]
#[command(name = "cc-switch")]
#[command(about = "A CLI tool for managing Claude API configurations")]
#[command(
    long_about = "cc-switch helps you manage multiple Claude API configurations and switch between them easily.

EXAMPLES:
    cc-switch add my-config sk-ant-xxx https://api.anthropic.com
    cc-switch switch my-config
    cc-switch switch cc
    cc-switch list
    cc-switch set-default-dir /path/to/claude/config"
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
    /// Stores a new configuration with alias, API token, and base URL
    #[command(alias = "a")]
    Add {
        /// Configuration alias name (used to identify this config)
        alias_name: String,
        /// ANTHROPIC_AUTH_TOKEN value (your Claude API token)
        token: String,
        /// ANTHROPIC_BASE_URL value (API endpoint URL)
        url: String,
    },
    /// Remove a configuration by alias name
    ///
    /// Deletes a stored configuration by its alias name
    #[command(alias = "r")]
    Remove {
        /// Configuration alias name to remove
        alias_name: String,
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
    /// Generates completion scripts for fish and zsh shells
    #[command(alias = "C")]
    Completion {
        /// Shell type (fish, zsh, bash, elvish, powershell)
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
}

/// Represents a Claude API configuration
///
/// Contains the three components needed to configure Claude API access:
/// - alias_name: User-friendly identifier for the configuration
/// - token: API authentication token
/// - url: Base URL for the API endpoint
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Configuration {
    /// User-friendly alias name for this configuration
    pub alias_name: String,
    /// ANTHROPIC_AUTH_TOKEN value (API authentication token)
    pub token: String,
    /// ANTHROPIC_BASE_URL value (API endpoint URL)
    pub url: String,
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
    /// Environment variables map (ANTHROPIC_AUTH_TOKEN, ANTHROPIC_BASE_URL)
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
    /// * `config` - Configuration containing token and URL to apply
    pub fn switch_to_config(&mut self, config: &Configuration) {
        // Ensure env field exists
        if self.env.is_empty() {
            self.env = HashMap::new();
        }

        self.env
            .insert("ANTHROPIC_AUTH_TOKEN".to_string(), config.token.clone());
        self.env
            .insert("ANTHROPIC_BASE_URL".to_string(), config.url.clone());
    }

    /// Remove Anthropic environment variables
    ///
    /// Clears ANTHROPIC_AUTH_TOKEN and ANTHROPIC_BASE_URL from settings
    /// Used to reset to default Claude behavior
    pub fn remove_anthropic_env(&mut self) {
        // Ensure env field exists
        if self.env.is_empty() {
            self.env = HashMap::new();
        }

        self.env.remove("ANTHROPIC_AUTH_TOKEN");
        self.env.remove("ANTHROPIC_BASE_URL");
    }
}

/// Get the path to the configuration storage file
///
/// Returns `~/.cc_auto_switch/configurations.json`
///
/// # Errors
/// Returns error if home directory cannot be found
fn get_config_storage_path() -> Result<PathBuf> {
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
fn get_claude_settings_path(custom_dir: Option<&str>) -> Result<PathBuf> {
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

/// Handle configuration switching command
///
/// Processes the switch subcommand to switch Claude API configuration:
/// - "cc": Remove API configuration (reset to default)
/// - Other alias: Switch to the specified configuration
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
    } else if let Some(config) = storage.get_configuration(alias_name) {
        claude_settings.switch_to_config(config);
        claude_settings.save(custom_dir)?;
        println!(
            "Switched to configuration '{}' (token: {}, url: {})",
            alias_name, config.token, config.url
        );
    } else {
        anyhow::bail!(
            "Configuration '{}' not found. Use 'list' command to see available configurations.",
            alias_name
        );
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
            println!("\n# Zsh completion generated successfully");
            println!("# Add this to your ~/.zsh/completions/_cc-switch");
            println!("# Or add this line to your ~/.zshrc:");
            println!("# fpath=(~/.zsh/completions $fpath)");
        }
        "bash" => {
            clap_complete::generate(
                clap_complete::shells::Bash,
                &mut app,
                "cc-switch",
                &mut stdout(),
            );
            println!("\n# Bash completion generated successfully");
            println!("# Add this to your ~/.bash_completion or /etc/bash_completion.d/");
        }
        "elvish" => {
            clap_complete::generate(
                clap_complete::shells::Elvish,
                &mut app,
                "cc-switch",
                &mut stdout(),
            );
            println!("\n# Elvish completion generated successfully");
        }
        "powershell" => {
            clap_complete::generate(
                clap_complete::shells::PowerShell,
                &mut app,
                "cc-switch",
                &mut stdout(),
            );
            println!("\n# PowerShell completion generated successfully");
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
    println!("\n# Fish completion generated successfully");
    println!("# Add this to your ~/.config/fish/completions/cc-switch.fish");
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
            } => {
                let config = Configuration {
                    alias_name: alias_name.clone(),
                    token,
                    url,
                };
                storage.add_configuration(config);
                storage.save()?;
                println!("Configuration '{alias_name}' added successfully");
            }
            Commands::Remove { alias_name } => {
                if storage.remove_configuration(&alias_name) {
                    storage.save()?;
                    println!("Configuration '{alias_name}' removed successfully");
                } else {
                    println!("Configuration '{alias_name}' not found");
                }
            }
            Commands::List => {
                if storage.configurations.is_empty() {
                    println!("No configurations stored");
                } else {
                    println!("Stored configurations:");
                    for (alias_name, config) in &storage.configurations {
                        println!(
                            "  {}: token={}, url={}",
                            alias_name, config.token, config.url
                        );
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
            Commands::Switch { alias_name } => {
                handle_switch_command(&alias_name)?;
            }
        }
    } else {
        // No command provided, show help
        println!("Use -h or --help for usage information");
    }

    Ok(())
}
