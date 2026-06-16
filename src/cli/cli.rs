use clap::{Parser, Subcommand};

/// Command-line interface for managing Claude API configurations
#[derive(Parser)]
#[command(name = "cc-switch")]
#[command(about = "A CLI tool for managing Claude API configurations")]
#[command(version)]
#[command(disable_help_subcommand = true)]
#[command(
    long_about = "cc-switch helps you manage multiple Claude API configurations and switch between them easily.

EXAMPLES:
    cc-switch add my-config sk-ant-xxx https://api.anthropic.com
    cc-switch add my-config -t sk-ant-xxx -u https://api.anthropic.com
    cc-switch add my-config -t sk-ant-xxx -u https://api.anthropic.com -m claude-3-5-sonnet-20241022
    cc-switch add my-config -t sk-ant-xxx -u https://api.anthropic.com --small-fast-model claude-3-haiku-20240307
    cc-switch add my-config -t sk-ant-xxx -u https://api.anthropic.com --max-thinking-tokens 8192
    cc-switch add my-config -i                       # Interactive mode
    cc-switch add my-config --from-file              # Import from ~/.claude/settings.json
    cc-switch add my-config --from-file ./other.json # Import from an explicit JSON file
    cc-switch add my-config --force  # Overwrite existing config
    cc-switch list
    cc-switch remove config1 config2 config3
    cc-switch current  # Interactive mode to view and switch configurations
    cc-switch  # Enter interactive mode (same as 'current' without arguments)

CODEX CONFIGURATIONS:
    cc-switch codex add work --from-file                       # Import from ~/.codex/auth.json
    cc-switch codex add work --from-file ~/other/auth.json     # Import from an explicit path
    cc-switch codex add personal -i  # Interactive mode
    cc-switch codex list
    cc-switch codex use work  # Switch and launch Codex
    cc-switch codex remove work

SHELL COMPLETION AND ALIASES:
    cc-switch completion fish  # Generates shell completions

    Recommended aliases:
    - cs='cc-switch'           # Quick access to cc-switch
    - cx='cc-switch codex'     # Quick access to cc-switch codex

    Install permanently:
    cc-switch completion fish > ~/.config/fish/completions/cc-switch.fish
    echo \"alias cs='cc-switch'\" >> ~/.config/fish/config.fish
    echo \"alias cx='cc-switch codex'\" >> ~/.config/fish/config.fish

    Then use:
    cs current    # Instead of cc-switch current
    cx use work   # Instead of cc-switch codex use work"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// List available configuration aliases (for shell completion)
    #[arg(long = "list-aliases", hide = true)]
    pub list_aliases: bool,

    /// List available Codex configuration aliases (for shell completion)
    #[arg(long = "list-codex-aliases", hide = true)]
    pub list_codex_aliases: bool,

    /// Migrate old config path (~/.cc_auto_switch/configurations.json) to new path
    #[arg(
        long = "migrate",
        help = "Migrate old config path to new path and exit"
    )]
    pub migrate: bool,

    /// Storage mode for writing configuration (env or config)
    #[arg(
        long = "store",
        help = "Storage mode for writing configuration (env: write to env field, config: write to root with camelCase)",
        global = true
    )]
    pub store: Option<String>,
}

/// Available subcommands for configuration management
#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum Commands {
    /// Add a new Claude API configuration
    ///
    /// Stores a new configuration with alias, API token, base URL, and optional model settings
    Add {
        /// Configuration alias name (used to identify this config)
        #[arg(help = "Configuration alias name (cannot be 'cc')")]
        alias_name: String,

        /// ANTHROPIC_AUTH_TOKEN value (your Claude API token)
        #[arg(
            long = "token",
            short = 't',
            help = "API token (ANTHROPIC_AUTH_TOKEN, mutually exclusive with --api-key)"
        )]
        token: Option<String>,

        /// ANTHROPIC_API_KEY value (alternative to --token, mutually exclusive)
        #[arg(
            long = "api-key",
            short = 'k',
            help = "API key (ANTHROPIC_API_KEY, mutually exclusive with --token)"
        )]
        api_key: Option<String>,

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

        /// ANTHROPIC_MAX_THINKING_TOKENS value (Maximum thinking tokens limit)
        #[arg(
            long = "max-thinking-tokens",
            help = "Maximum thinking tokens limit (optional)"
        )]
        max_thinking_tokens: Option<u32>,

        /// API timeout in milliseconds
        #[arg(
            long = "api-timeout-ms",
            help = "API timeout in milliseconds (optional)"
        )]
        api_timeout_ms: Option<u32>,

        /// Disable non-essential traffic flag
        #[arg(
            long = "disable-nonessential-traffic",
            help = "Disable non-essential traffic flag (optional)"
        )]
        claude_code_disable_nonessential_traffic: Option<u32>,

        /// Default Sonnet model name
        #[arg(
            long = "default-sonnet-model",
            help = "Default Sonnet model name (optional)"
        )]
        anthropic_default_sonnet_model: Option<String>,

        /// Default Opus model name
        #[arg(
            long = "default-opus-model",
            help = "Default Opus model name (optional)"
        )]
        anthropic_default_opus_model: Option<String>,

        /// Default Haiku model name
        #[arg(
            long = "default-haiku-model",
            help = "Default Haiku model name (optional)"
        )]
        anthropic_default_haiku_model: Option<String>,

        /// CLAUDE_CODE_SUBAGENT_MODEL value (model for subagents)
        #[arg(long = "subagent-model", help = "Subagent model name (optional)")]
        claude_code_subagent_model: Option<String>,

        /// CLAUDE_CODE_DISABLE_NONSTREAMING_FALLBACK flag
        #[arg(
            long = "disable-nonstreaming-fallback",
            help = "Disable non-streaming fallback flag (optional)"
        )]
        claude_code_disable_nonstreaming_fallback: Option<u32>,

        /// CLAUDE_CODE_EFFORT_LEVEL value
        #[arg(
            long = "effort-level",
            help = "Effort level for Claude Code (optional, e.g., 'max')"
        )]
        claude_code_effort_level: Option<String>,

        /// DISABLE_PROMPT_CACHING flag
        #[arg(
            long = "disable-prompt-caching",
            help = "Disable prompt caching flag (optional)"
        )]
        disable_prompt_caching: Option<u32>,

        /// CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS flag
        #[arg(
            long = "disable-experimental-betas",
            help = "Disable experimental betas flag (optional)"
        )]
        claude_code_disable_experimental_betas: Option<u32>,

        /// DISABLE_AUTOUPDATER flag
        #[arg(
            long = "disable-autoupdater",
            help = "Disable auto-updater flag (optional)"
        )]
        disable_autoupdater: Option<u32>,

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

        /// Import configuration from a JSON file
        ///
        /// With no value, imports from `~/.claude/settings.json`.
        /// With a value, imports from the given path.
        #[arg(
            long = "from-file",
            num_args = 0..=1,
            value_name = "PATH",
            help = "Import configuration from JSON file (defaults to ~/.claude/settings.json if no path)"
        )]
        from_file: Option<Option<String>>,
    },
    /// Remove one or more configurations by alias name
    ///
    /// Deletes stored configurations by their alias names
    Remove {
        /// Configuration alias name(s) to remove (one or more)
        #[arg(required = true)]
        alias_names: Vec<String>,
    },
    /// List all stored configurations
    ///
    /// Displays all saved configurations with their aliases, tokens, and URLs
    List {
        /// Output in plain text format (default is JSON)
        #[arg(long = "plain", short = 'p')]
        plain: bool,
        /// Show only name and URL
        #[arg(long = "name", short = 'n')]
        name: bool,
    },
    /// Generate shell completion scripts
    ///
    /// Generates completion scripts for supported shells
    #[command(alias = "C")]
    Completion {
        /// Shell type (fish, zsh, bash, elvish, powershell)
        #[arg(default_value = "fish")]
        shell: String,
    },
    /// Switch to a configuration and optionally send a prompt to Claude
    ///
    /// Quickly switches to the specified configuration and launches Claude.
    /// Any additional arguments after the alias name are joined and sent as a prompt.
    /// Use --resume to resume a previous Claude session by ID.
    /// Use --continue to continue the most recent Claude session.
    #[command(trailing_var_arg = true)]
    Use {
        /// Configuration alias name to switch to
        alias_name: String,

        /// Resume a previous Claude session by ID
        #[arg(long, short = 'r')]
        resume: Option<String>,

        /// Continue the most recent Claude session
        #[arg(long, short = 'c')]
        r#continue: bool,

        /// Prompt to send to Claude (all remaining arguments)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        prompt: Vec<String>,
    },
    /// Manage Codex (OpenAI CLI) configurations
    Codex {
        #[command(subcommand)]
        command: Option<CodexCommands>,
    },
    /// Manage the ccs-proxy daemon (start/stop/status/restart)
    ///
    /// The daemon supervises one local ccs-proxy per unique upstream URL,
    /// transparently capturing all Claude API traffic for the dashboard.
    Daemon {
        #[command(subcommand)]
        command: DaemonCommands,
    },
    /// Manage statusLine integration with Claude Code
    ///
    /// Installs a wrapper script that displays the current cc-switch alias name
    /// in Claude Code's statusLine, alongside the original statusLine content.
    ///
    /// Usage:
    ///   cc-switch statusline install    # Install/update the wrapper
    ///   cc-switch statusline uninstall  # Remove the wrapper
    #[command(name = "statusline")]
    Statusline {
        /// Action to perform (install or uninstall)
        #[arg(value_enum, default_value = "install")]
        action: StatuslineAction,
    },
}

/// Actions for the statusline subcommand
#[derive(Clone, Copy, clap::ValueEnum)]
pub enum StatuslineAction {
    /// Install the statusLine wrapper script
    Install,
    /// Uninstall the statusLine wrapper script
    Uninstall,
}

/// Subcommands for `cc-switch daemon`
#[derive(Subcommand)]
pub enum DaemonCommands {
    /// Start the daemon (double-forks into background by default)
    Start {
        /// Run in the foreground (don't daemonize). Useful for debugging.
        #[arg(long)]
        foreground: bool,

        /// Log level: error, warn, info, debug, trace
        #[arg(long = "log-level", value_name = "LEVEL")]
        log_level: Option<String>,

        /// Increase verbosity (-v info, -vv debug, -vvv trace)
        #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count)]
        verbose: u8,

        /// Capture `cc use official` traffic through the daemon proxy.
        /// Off by default — official traffic flows direct to Anthropic.
        #[arg(long = "capture-official")]
        capture_official: bool,
    },
    /// Stop the running daemon
    Stop,
    /// Show daemon status and proxy health
    Status {
        /// Output as JSON instead of a human-readable table
        #[arg(long)]
        json: bool,
    },
    /// Stop then start the daemon (picks up configuration changes)
    Restart {
        /// Run in the foreground after restart
        #[arg(long)]
        foreground: bool,

        /// Log level: error, warn, info, debug, trace
        #[arg(long = "log-level", value_name = "LEVEL")]
        log_level: Option<String>,

        /// Increase verbosity (-v info, -vv debug, -vvv trace)
        #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count)]
        verbose: u8,

        /// Capture `cc use official` traffic through the daemon proxy.
        /// Off by default — official traffic flows direct to Anthropic.
        #[arg(long = "capture-official")]
        capture_official: bool,
    },
}

/// Available subcommands for Codex configuration management
#[derive(Subcommand)]
pub enum CodexCommands {
    /// Add a new Codex (OpenAI CLI) configuration
    Add {
        #[arg(help = "Configuration alias name")]
        alias_name: String,
        #[arg(long = "api-key", help = "OpenAI API key (optional)")]
        api_key: Option<String>,
        #[arg(long = "force", short = 'f', help = "Overwrite existing configuration")]
        force: bool,
        #[arg(
            long = "interactive",
            short = 'i',
            help = "Enter configuration values interactively"
        )]
        interactive: bool,
        /// Import from existing auth.json file
        ///
        /// With no value, imports from `~/.codex/auth.json`.
        /// With a value, imports from the given path.
        #[arg(
            long = "from-file",
            num_args = 0..=1,
            value_name = "PATH",
            help = "Import from auth.json (defaults to ~/.codex/auth.json if no path)"
        )]
        from_file: Option<Option<String>>,
    },
    List {
        #[arg(long = "plain", short = 'p')]
        plain: bool,
        #[arg(long = "name", short = 'n', help = "Show only name and auth mode")]
        name: bool,
    },
    #[command(trailing_var_arg = true)]
    Use {
        alias_name: String,
        #[arg(long = "continue", short = 'c')]
        r#continue: bool,
        #[arg(long = "resume", short = 'r')]
        resume: Option<String>,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        prompt: Vec<String>,
    },
    Remove {
        #[arg(required = true)]
        alias_names: Vec<String>,
    },
}
