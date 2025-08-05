# cc-switch

[![Crates.io](https://img.shields.io/crates/v/cc-switch.svg)](https://crates.io/crates/cc-switch)
[![GitHub Packages](https://img.shields.io/badge/GitHub-Packages-green)](https://github.com/jingzhao/cc_auto_switch/packages)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Build Status](https://github.com/jingzhao/cc_auto_switch/workflows/CI/badge.svg)](https://github.com/jingzhao/cc_auto_switch/actions)
[![Release](https://github.com/jingzhao/cc_auto_switch/workflows/Release/badge.svg)](https://github.com/jingzhao/cc_auto_switch/releases)

A powerful command-line tool for managing multiple Claude API configurations and switching between them effortlessly.

## 🎯 Features

- **Multi-Configuration Management**: Store and manage multiple Claude API configurations
- **Quick Switching**: Instantly switch between different API configurations using aliases
- **Interactive Mode**: Secure token entry without exposing in shell history
- **Bulk Operations**: Add and remove multiple configurations at once
- **Shell Completion**: Built-in completion for fish, zsh, bash, elvish, powershell, and nushell
- **Dynamic Alias Completion**: Auto-complete configuration names for switch and remove commands
- **Current Configuration Display**: Show active API configuration with `cc-switch current`
- **Safe Configuration Storage**: Securely stores configurations in `~/.cc-switch/`
- **Cross-Platform**: Works on Linux, macOS, and Windows
- **Custom Settings Directory**: Support for custom Claude settings directories
- **Force Overwrite**: Replace existing configurations with `--force` flag
- **Zero Configuration**: Works out of the box with sensible defaults

## 🚀 Installation

### From Crates.io (Recommended)

```bash
cargo install cc-switch
```

### From Source

```bash
git clone https://github.com/jingzhao/cc_auto_switch.git
cd cc-switch
cargo build --release
```

The binary will be available at `target/release/cc-switch`. You can copy it to your PATH:

```bash
cp target/release/cc-switch ~/.local/bin/
```

## 📖 Usage

### Basic Commands

#### Add a Configuration

```bash
# Add a new Claude API configuration (positional arguments)
cc-switch add my-config sk-ant-xxx https://api.anthropic.com

# Add using flags (more explicit)
cc-switch add my-config -t sk-ant-xxx -u https://api.anthropic.com

# Add in interactive mode (secure)
cc-switch add my-config -i

# Add with force overwrite
cc-switch add my-config -t sk-ant-xxx -u https://api.anthropic.com --force
```

#### List All Configurations

```bash
# List all stored configurations
cc-switch list
```

Output:
```
Stored configurations:
  my-config: token=sk-ant-xxx, url=https://api.anthropic.com
  work-config: token=sk-ant-work-123, url=https://api.anthropic.com
Claude settings directory: ~/.claude/ (default)
```

#### Switch Configuration

```bash
# Switch to a specific configuration
cc-switch switch my-config

# Reset to default (remove API configuration)
cc-switch switch cc
```

#### Remove a Configuration

```bash
# Remove a single configuration
cc-switch remove my-config

# Remove multiple configurations at once
cc-switch remove config1 config2 config3
```

#### Set Custom Settings Directory

```bash
# Set custom directory for Claude settings.json
cc-switch set-default-dir /path/to/claude/config
```

### Shell Completion

Generate shell completion scripts:

```bash
# Fish shell
cc-switch completion fish > ~/.config/fish/completions/cc-switch.fish

# Zsh shell
cc-switch completion zsh > ~/.zsh/completions/_cc-switch
# Or add to your ~/.zshrc:
# fpath=(~/.zsh/completions $fpath)

# Bash shell
cc-switch completion bash > ~/.bash_completion.d/cc-switch
# Or add to your ~/.bashrc:
# source ~/.bash_completion.d/cc-switch

# Additional supported shells
cc-switch completion elvish     # Elvish shell
cc-switch completion powershell # PowerShell
cc-switch completion nushell    # Nushell
```

### Dynamic Completion

Both `switch` and `remove` commands support dynamic alias completion:

```bash
# After setting up completion, type:
cc-switch switch <TAB>  # Will show available aliases: cc, my-config, work-config
cc-switch remove <TAB>  # Will show available aliases: my-config, work-config
```

### Advanced Usage

#### Show Current Configuration

```bash
# Display current API configuration
cc-switch current

# Output examples:
# Token: sk-ant-xxx
# URL: https://api.anthropic.com
# 
# Or when not configured:
# No ANTHROPIC_AUTH_TOKEN or ANTHROPIC_BASE_URL configured
```

#### Custom Settings Directory

If you have Claude settings in a non-standard location:

```bash
# Set custom directory
cc-switch set-default-dir ~/development/claude-config

# Now all operations will use this directory
cc-switch switch my-config
```

#### Configuration Management

```bash
# Add multiple configurations for different environments
cc-switch add dev sk-ant-dev-xxx https://api.anthropic.com
cc-switch add prod sk-ant-prod-xxx https://api.anthropic.com
cc-switch add staging sk-ant-staging-xxx https://api.anthropic.com

# Switch between environments
cc-switch switch dev      # Switch to development
cc-switch switch prod     # Switch to production
cc-switch switch cc       # Reset to default

# Check current configuration
cc-switch current
```

#### Interactive Mode

For secure token entry without exposing tokens in shell history:

```bash
# Add configuration interactively
cc-switch add my-config -i

# You'll be prompted for token and URL
Enter API token (sk-ant-xxx): [hidden input]
Enter API URL (default: https://api.anthropic.com): https://custom.api.com
```

#### Bulk Operations

```bash
# Add multiple configurations with flags
cc-switch add dev -t sk-ant-dev-xxx -u https://api.anthropic.com
cc-switch add prod -t sk-ant-prod-xxx -u https://api.anthropic.com

# Remove multiple configurations at once
cc-switch remove old-config deprecated-config test-config
```

## 🔧 Configuration

### Storage Location

Configurations are stored in `~/.cc-switch/configurations.json` in the following format:

```json
{
  "configurations": {
    "my-config": {
      "alias_name": "my-config",
      "token": "sk-ant-xxx",
      "url": "https://api.anthropic.com"
    }
  },
  "claude_settings_dir": null
}
```

### Claude Settings Integration

The tool modifies Claude's `settings.json` file to set environment variables:

- `ANTHROPIC_AUTH_TOKEN`: Your API token
- `ANTHROPIC_BASE_URL`: The API base URL

By default, it looks for `settings.json` in `~/.claude/`, but you can specify a custom directory.

## 🛠️ Development

### Prerequisites

- Rust 1.88.0 or later
- Cargo (included with Rust)
- audit (for security auditing) - `cargo install cargo-audit`

### Building

```bash
# Clone the repository
git clone https://github.com/jingzhao/cc_auto_switch.git
cd cc-switch

# Build in development mode
cargo build

# Build in release mode
cargo build --release

# Run tests
cargo test

# Check formatting
cargo fmt --check

# Run linter
cargo clippy

# Run security audit
cargo audit
```

### Project Structure

```
cc-switch/
├── src/
│   ├── main.rs          # Application entry point
│   └── cmd/
│       ├── main.rs      # Command logic and data models
│       ├── mod.rs       # Module definitions
│       ├── tests.rs     # Unit tests
│       ├── integration_tests.rs  # Integration tests
│       └── error_handling_tests.rs  # Error handling tests
├── Cargo.toml           # Project configuration
├── Cargo.lock           # Dependency lock file
├── .github/
│   └── workflows/
│       ├── ci.yml       # CI pipeline
│       └── release.yml  # Release automation
└── README.md           # This file
```

## 🧪 Testing

The project includes comprehensive tests:

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run integration tests
cargo test integration_tests

# Run error handling tests
cargo test error_handling_tests

# Run specific test modules
cargo test test_config_storage
cargo test test_claude_settings
```

## 📦 Release

Releases are automatically built and published using GitHub Actions:

- **Supported Platforms**: Linux (x86_64, aarch64), macOS (x86_64, aarch64), Windows (x86_64)
- **Release Artifacts**: Pre-compiled binaries for each platform
- **Automation**: CI/CD pipeline ensures quality and consistency

### Manual Release

```bash
# Create a new release tag
git tag -a v1.0.0 -m "Release version 1.0.0"
git push origin v1.0.0
```

GitHub Actions will automatically build and publish the release.

### Publishing to GitHub Packages

You can also publish the crate to GitHub Packages:

```bash
# Using the publish script
./publish.sh

# Or using cargo directly
cargo publish

# For a dry-run to check everything is correct
cargo publish --dry-run
```

The package will be available at: https://github.com/jingzhao/cc_auto_switch/packages

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guidelines](CONTRIBUTING.md) for details.

### Development Workflow

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Run tests and ensure code quality (`cargo test && cargo clippy`)
5. Commit your changes (`git commit -m 'Add amazing feature'`)
6. Push to the branch (`git push origin feature/amazing-feature`)
7. Open a Pull Request

### Code Style

- Follow Rust conventions and best practices
- Use `cargo fmt` for formatting
- Address all `cargo clippy` warnings
- Write tests for new functionality
- Update documentation as needed

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [Claude](https://claude.ai/) for the amazing AI assistant
- [Rust](https://www.rust-lang.org/) programming language
- [Clap](https://github.com/clap-rs/clap) for command-line argument parsing
- [Serde](https://github.com/serde-rs/serde) for JSON serialization

## 📞 Support

- 🐛 **Bug Reports**: [GitHub Issues](https://github.com/jingzhao/cc_auto_switch/issues)
- 💡 **Feature Requests**: [GitHub Issues](https://github.com/jingzhao/cc_auto_switch/issues)
- 📧 **Questions**: [GitHub Discussions](https://github.com/jingzhao/cc_auto_switch/discussions)

## 🔄 Changelog

See [CHANGELOG.md](CHANGELOG.md) for a list of changes and version history.

---

**Made with ❤️ by [jingzhao](https://github.com/jingzhao)**