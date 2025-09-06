use clap::{Parser, Subcommand};

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
    cc-switch use my-config
    cc-switch use -a my-config
    cc-switch use --alias my-config
    cc-switch use  # Interactive mode
    cc-switch use cc
    cc-switch list
    cc-switch remove config1 config2 config3
    cc-switch current  # Interactive menu for configuration management

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
    cs use my-config    # Instead of cc-switch use my-config
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
        #[arg(long = "model", short = 'm', help = "Custom model name (optional)")]
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
    /// Use a configuration by alias name
    ///
    /// Switches Claude to use the specified API configuration
    /// Use 'cc' as alias name to reset to default Claude behavior
    #[command(alias = "sw", alias = "switch")]
    Use {
        /// Configuration alias name (use 'cc' to reset to default)
        #[arg(help = "Configuration alias name (use 'cc' to reset to default)")]
        alias_name: String,
    },
    /// Interactive current configuration menu
    ///
    /// Shows current configuration and provides interactive menu for:
    /// 1. Execute claude --dangerously-skip-permissions
    /// 2. Switch configuration (lists available aliases)
    #[command(alias = "cur")]
    Current,
}
