# cc-switch

[![Crates.io](https://img.shields.io/crates/v/cc-switch.svg)](https://crates.io/crates/cc-switch)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**A simple CLI tool for managing multiple Claude API configurations**

Easily switch between different Claude API accounts, environments, or projects without manually editing configuration files.

## Why?

When working with Claude API across different projects or environments, you often need to switch API tokens and settings. cc-switch makes this painless:

- Store multiple configurations with memorable names
- Switch instantly between them
- Launch Claude with the right environment variables automatically
- Keep your API keys organized and secure

## Quick Start

```bash
# Install
cargo install cc-switch

# Add your first configuration
cc-switch add work sk-ant-work-xxx https://api.anthropic.com

# Add another one
cc-switch add personal sk-ant-personal-xxx https://api.anthropic.com

# Switch to work configuration
cc-switch use work

# Switch to personal configuration
cc-switch use personal

# See all configurations
cc-switch list
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
| `cc-switch add <name> <token> <url>` | Add new configuration |
| `cc-switch use <name>` | Switch to configuration |
| `cc-switch list` | Show all configurations |
| `cc-switch remove <name>` | Delete configuration |
| `cc-switch current` | Interactive menu |

## Advanced Usage

### Interactive Mode
```bash
# Enter interactive mode (no arguments needed)
cc-switch

# Or access via current command
cc-switch current
```

### Add with Models
```bash
# Add configuration with custom model
cc-switch add work -t sk-ant-xxx -u https://api.anthropic.com -m claude-3-5-sonnet-20241022

# Add with fast model for background tasks
cc-switch add work --small-fast-model claude-3-haiku-20240307
```

### Shell Integration
```bash
# Generate shell completions
cc-switch completion fish > ~/.config/fish/completions/cc-switch.fish

# Create handy aliases
eval "$(cc-switch alias fish)"
# Now you can use:
cs use work     # Instead of cc-switch use work
ccd             # Quick Claude launch
```

## How it Works

cc-switch stores your configurations in `~/.cc-switch/configurations.json` and launches Claude with the appropriate environment variables set. This means:

- ✅ No global configuration changes
- ✅ Complete isolation between configurations
- ✅ Safe and secure API key management
- ✅ Works with any Claude installation

## Environment Variables

The tool sets these environment variables when launching Claude:

- `ANTHROPIC_AUTH_TOKEN` - Your API token
- `ANTHROPIC_BASE_URL` - API endpoint URL
- `ANTHROPIC_MODEL` - Custom model (optional)
- `ANTHROPIC_SMALL_FAST_MODEL` - Fast model for background tasks (optional)

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