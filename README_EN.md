# cc-switch

[![Crates.io](https://img.shields.io/crates/v/cc-switch.svg)](https://crates.io/crates/cc-switch)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**A CLI tool for managing multiple Claude / Codex configurations and automatically switching between them.**

When working with Claude API or OpenAI Codex CLI across different projects or environments, you often need to switch API tokens and settings. cc-switch makes this painless:

- Store multiple configurations with memorable names
- Switch instantly between them
- Launch Claude / Codex with the right environment variables automatically
- Keep your API keys organized and secure
- Import/export configurations from JSON files
- Full interactive mode with keyboard navigation

## Quick Start

```bash
# Install
cargo install cc-switch

# ===== Claude Configurations =====

# Add your first configuration
cc-switch add work sk-ant-work-xxx https://api.anthropic.com

# Switch to work configuration
cc-switch
# Then select 'work' from the interactive menu

# ===== Codex Configurations =====

# Import from existing auth.json
cc-switch codex add work --from-file ~/.codex/auth.json

# Switch to work configuration and launch Codex
cc-switch codex use work

# Enter Codex interactive mode
cc-switch codex
# Then select 'work' from the interactive menu

# See all configurations
cc-switch list        # Claude configurations
cc-switch codex list  # Codex configurations
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

### Claude Configuration Management

| Command | What it does |
|---------|--------------|
| `cc-switch add <name>` | Add new configuration |
| `cc-switch list` | Show all configurations (JSON or plain text) |
| `cc-switch remove <name...>` | Delete one or more configurations |
| `cc-switch use <name>` | Quick switch and launch Claude |
| `cc-switch` | Enter interactive mode |

### Codex Configuration Management

| Command | What it does |
|---------|--------------|
| `cc-switch codex add <name>` | Add new configuration |
| `cc-switch codex list` | Show all configurations |
| `cc-switch codex remove <name...>` | Delete configurations |
| `cc-switch codex use <name>` | Switch and launch Codex |
| `cc-switch codex` | Enter interactive mode |

For detailed documentation, see [Codex Configuration Management](docs/codex.md).

### Common Commands

| Command | What it does |
|---------|--------------|
| `cc-switch completion <shell>` | Generate shell completion scripts |

## Advanced Usage

### Interactive Mode
```bash
# Claude interactive mode
cc-switch

# Codex interactive mode
cc-switch codex

# Navigate with:
# - ↑↓ arrows or 1-9 keys: Select configuration
# - N/P: Next/Previous page (when >9 configs)
# - R: Reset to default Claude (Claude mode only)
# - E: Edit configuration
# - Q: Exit
```

### Quick Switch (use command)
```bash
# Switch to a configuration and launch Claude
cc-switch use work

# Switch and send a prompt
cc-switch use work "Write a Python script for me"

# Switch and resume a previous session
cc-switch use work --resume c8439f36-44a9-4d85-9e88-de35e004fdd4
cc-switch use work -r c8439f36-44a9-4d85-9e88-de35e004fdd4

# Switch and continue the most recent session
cc-switch use work --continue
cc-switch use work -c
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

## Codex Configuration Management

`cc-switch codex` manages multiple OpenAI Codex CLI authentication configurations.

```bash
# Import from existing auth.json
cc-switch codex add work --from-file ~/.codex/auth.json

# Interactive creation
cc-switch codex add personal -i

# Enter interactive mode (TUI)
cc-switch codex

# Switch configuration and launch Codex
cc-switch codex use work
```

For detailed documentation, see [docs/codex.md](docs/codex.md).

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
echo "alias cx='cc-switch codex'" >> ~/.config/fish/config.fish

# Zsh
echo "alias cs='cc-switch'" >> ~/.zshrc
echo "alias ccd='claude --dangerously-skip-permissions'" >> ~/.zshrc
echo "alias cx='cc-switch codex'" >> ~/.zshrc

# Bash
echo "alias cs='cc-switch'" >> ~/.bashrc
echo "alias ccd='claude --dangerously-skip-permissions'" >> ~/.bashrc
echo "alias cx='cc-switch codex'" >> ~/.bashrc

# Now you can use:
cs              # Instead of cc-switch (enters interactive mode)
ccd             # Quick Claude launch
cx              # Enter Codex interactive mode
```

## How it Works

cc-switch stores configurations in `~/.claude/cc_auto_switch_setting.json`:

**Claude configurations**: Updates Claude's `settings.json` file with the appropriate environment variables.

**Codex configurations**: Writes to `~/.codex/auth.json` file, which Codex CLI reads for authentication.

This means:

- ✅ No global configuration changes
- ✅ Complete isolation between configurations
- ✅ Safe and secure API key management
- ✅ Works with any Claude / Codex installation
- ✅ Preserves other settings
- ✅ Supports custom settings directory

## Environment Variables

### Claude Configurations

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
- `CLAUDE_CODE_SUBAGENT_MODEL` - Subagent model (optional)
- `CLAUDE_CODE_DISABLE_NONSTREAMING_FALLBACK` - Disable non-streaming fallback flag (optional)
- `CLAUDE_CODE_EFFORT_LEVEL` - Effort level (optional, e.g., 'max')

### Codex Configurations

Codex configurations are stored in `~/.codex/auth.json`, supporting two authentication modes:

**chatgpt mode (OAuth)**:
- `id_token` - ID token
- `access_token` - Access token
- `refresh_token` - Refresh token
- `account_id` - Account ID

**apikey mode**:
- `OPENAI_API_KEY` - API key

For detailed documentation, see [Codex Configuration Management](docs/codex.md).

## Import/Export

### Claude Configurations from JSON
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

### Codex Configurations from auth.json
```bash
# Import from existing auth.json
cc-switch codex add work --from-file ~/.codex/auth.json
```

For detailed documentation, see [Codex Configuration Management](docs/codex.md).

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