# cc-switch

[![Crates.io](https://img.shields.io/crates/v/cc-switch.svg)](https://crates.io/crates/cc-switch)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**A CLI tool for managing multiple Claude API configurations and automatically switching between them**

Easily switch between different Claude API accounts, environments, or projects without manually editing configuration files.

## Why?

When working with Claude API across different projects or environments, you often need to switch API tokens and settings. cc-switch makes this painless:

- Store multiple configurations with memorable names
- Switch instantly between them
- Launch Claude with the right environment variables automatically
- Keep your API keys organized and secure
- Import/export configurations from JSON files
- Full interactive mode with keyboard navigation

## Quick Start

```bash
# Install
cargo install cc-switch

# Add your first configuration
cc-switch add work sk-ant-work-xxx https://api.anthropic.com

# Add another one
cc-switch add personal sk-ant-personal-xxx https://api.anthropic.com

# Switch to work configuration
cc-switch
# Then select 'work' from the interactive menu

# Switch to personal configuration
cc-switch
# Then select 'personal' from the interactive menu

# See all configurations
cc-switch list

# Enter interactive mode (same as above)
cc-switch
```

## Installation

### Cargo (Recommended)
```bash
cargo install cc-switch
```

### Homebrew
```bash
brew tap Linuxdazhao/cc-switch
brew install cc-switch
```

## Main Commands

| Command | What it does |
|---------|--------------|
| `cc-switch add <name>` | Add new configuration |
| `cc-switch list` | Show all configurations (JSON or plain text) |
| `cc-switch remove <name...>` | Delete one or more configurations |
| `cc-switch completion <shell>` | Generate shell completion scripts |
| `cc-switch` | Enter interactive mode |

## Advanced Usage

### Interactive Mode
```bash
# Enter interactive mode (no arguments needed)
cc-switch

# Navigate with:
# - ↑↓ arrows or 1-9 keys: Select configuration
# - N/P: Next/Previous page (when >9 configs)
# - R: Reset to default Claude
# - E: Exit
```

### Add with Full Configuration
```bash
# Add configuration with all options
cc-switch add work -t sk-ant-xxx -u https://api.anthropic.com \
  -m claude-3-5-sonnet-20241022 \
  --small-fast-model claude-3-haiku-20240307 \
  --max-thinking-tokens 8192 \
  --api-timeout-ms 300000 \
  --disable-nonessential-traffic 1 \
  --default-sonnet-model claude-3-5-sonnet-20241022 \
  --default-opus-model claude-3-opus-20240229 \
  --default-haiku-model claude-3-haiku-20240307

# Add with force overwrite
cc-switch add work -t sk-ant-xxx -u https://api.anthropic.com -f

# Add in interactive mode
cc-switch add work -i

# Import from JSON file (filename becomes alias)
cc-switch add --from-file config.json
```

### Storage Modes
```bash
# Set default storage mode
cc-switch --store env    # Write to env field (default)
cc-switch --store config # Write to root level with camelCase
```

### List Configurations
```bash
# List in JSON format (default)
cc-switch list

# List in plain text format
cc-switch list -p
```

### Remove Multiple Configurations
```bash
# Remove one configuration
cc-switch remove work

# Remove multiple configurations at once
cc-switch remove work personal test-config
```

### Configuration Migration
```bash
# Migrate from old path (~/.cc_auto_switch/) to new path
cc-switch --migrate
```

## Shell Integration

### Generate Completions
```bash
# Fish (recommended)
cc-switch completion fish > ~/.config/fish/completions/cc-switch.fish

# Zsh
cc-switch completion zsh > ~/.zsh/completions/_cc-switch
echo 'fpath=(~/.zsh/completions $fpath)' >> ~/.zshrc

# Bash
cc-switch completion bash > ~/.bash_completion.d/cc-switch
echo 'source ~/.bash_completion.d/cc-switch' >> ~/.bashrc

# Elvish or PowerShell also supported
cc-switch completion elvish
cc-switch completion powershell
```

### Create Aliases
```bash
# Add aliases permanently to shell config

# Fish
echo "alias cs='cc-switch'" >> ~/.config/fish/config.fish
echo "alias ccd='claude --dangerously-skip-permissions'" >> ~/.config/fish/config.fish

# Zsh
echo "alias cs='cc-switch'" >> ~/.zshrc
echo "alias ccd='claude --dangerously-skip-permissions'" >> ~/.zshrc

# Bash
echo "alias cs='cc-switch'" >> ~/.bashrc
echo "alias ccd='claude --dangerously-skip-permissions'" >> ~/.bashrc

# Now you can use:
cs              # Instead of cc-switch (enters interactive mode)
ccd             # Quick Claude launch
```

## How it Works

cc-switch stores your configurations in `~/.cc-switch/configurations.json` and updates Claude's `settings.json` file with the appropriate environment variables. This means:

- ✅ No global configuration changes
- ✅ Complete isolation between configurations
- ✅ Safe and secure API key management
- ✅ Works with any Claude installation
- ✅ Preserves other Claude settings
- ✅ Supports custom Claude settings directory

## Environment Variables

The tool sets these environment variables when switching configuration:

- `ANTHROPIC_AUTH_TOKEN` - Your API token
- `ANTHROPIC_BASE_URL` - API endpoint URL
- `ANTHROPIC_MODEL` - Custom model (optional)
- `ANTHROPIC_SMALL_FAST_MODEL` - Fast model for background tasks (optional)
- `ANTHROPIC_MAX_THINKING_TOKENS` - Maximum thinking tokens limit (optional)
- `API_TIMEOUT_MS` - API timeout in milliseconds (optional)
- `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` - Disable non-essential traffic flag (optional)
- `ANTHROPIC_DEFAULT_SONNET_MODEL` - Default Sonnet model (optional)
- `ANTHROPIC_DEFAULT_OPUS_MODEL` - Default Opus model (optional)
- `ANTHROPIC_DEFAULT_HAIKU_MODEL` - Default Haiku model (optional)

## Import/Export

### Import from JSON
```bash
# Import configuration from JSON file
# The filename (without extension) becomes the alias name
cc-switch add --from-file my-work-config.json

# JSON format expected:
# {
#   "env": {
#     "ANTHROPIC_AUTH_TOKEN": "sk-ant-xxx",
#     "ANTHROPIC_BASE_URL": "https://api.anthropic.com",
#     "ANTHROPIC_MODEL": "claude-3-5-sonnet-20241022"
#   }
# }
```

## Development

```bash
# Clone
git clone https://github.com/Linuxdazhao/cc_auto_switch.git
cd cc-switch

# Build
cargo build --release

# Test
cargo test
```

## License

MIT License - see [LICENSE](LICENSE) file.

---

**Made with ❤️ by [Linuxdazhao](https://github.com/Linuxdazhao)**