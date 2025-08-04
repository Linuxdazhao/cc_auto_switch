# cc-switch

[![Crates.io](https://img.shields.io/crates/v/cc-switch.svg)](https://crates.io/crates/cc-switch)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Build Status](https://github.com/yourusername/cc-switch/workflows/CI/badge.svg)](https://github.com/yourusername/cc-switch/actions)
[![Release](https://github.com/yourusername/cc-switch/workflows/Release/badge.svg)](https://github.com/yourusername/cc-switch/releases)

A powerful command-line tool for managing multiple Claude API configurations and switching between them effortlessly.

## 🎯 Features

- **Multi-Configuration Management**: Store and manage multiple Claude API configurations
- **Quick Switching**: Instantly switch between different API configurations using aliases
- **Shell Completion**: Built-in completion for fish, zsh, bash, and other shells
- **Dynamic Alias Completion**: Auto-complete configuration names for switch and remove commands
- **Safe Configuration Storage**: Securely stores configurations in `~/.cc-switch/`
- **Cross-Platform**: Works on Linux, macOS, and Windows
- **Custom Settings Directory**: Support for custom Claude settings directories
- **Zero Configuration**: Works out of the box with sensible defaults

## 🚀 Installation

### From Crates.io (Recommended)

```bash
cargo install cc-switch
```

### From Source

```bash
git clone https://github.com/yourusername/cc-switch.git
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
# Add a new Claude API configuration
cc-switch add my-config sk-ant-xxx https://api.anthropic.com
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
# Remove a configuration by alias
cc-switch remove my-config
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
cc-switch completion bash > ~/.bash_completion.d/cc-switch.bash
# Or add to your ~/.bash_completion
```

### Dynamic Completion

Both `switch` and `remove` commands support dynamic alias completion:

```bash
# After setting up completion, type:
cc-switch switch <TAB>  # Will show available aliases: cc, my-config, work-config
cc-switch remove <TAB>  # Will show available aliases: my-config, work-config
```

### Advanced Usage

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

### Building

```bash
# Clone the repository
git clone https://github.com/yourusername/cc-switch.git
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
```

### Project Structure

```
cc-switch/
├── src/
│   ├── main.rs          # Application entry point
│   └── cmd.rs           # Command logic and data models
├── Cargo.toml           # Project configuration
├── Cargo.lock           # Dependency lock file
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
cargo test --test integration
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

- 🐛 **Bug Reports**: [GitHub Issues](https://github.com/yourusername/cc-switch/issues)
- 💡 **Feature Requests**: [GitHub Issues](https://github.com/yourusername/cc-switch/issues)
- 📧 **Questions**: [GitHub Discussions](https://github.com/yourusername/cc-switch/discussions)

## 🔄 Changelog

See [CHANGELOG.md](CHANGELOG.md) for a list of changes and version history.

---

**Made with ❤️ by [Your Name](https://github.com/yourusername)**