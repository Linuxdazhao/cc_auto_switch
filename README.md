# cc-switch

[![Crates.io](https://img.shields.io/crates/v/cc-switch.svg)](https://crates.io/crates/cc-switch)
[![GitHub Packages](https://img.shields.io/badge/GitHub-Packages-green)](https://github.com/jingzhao/cc_auto_switch/packages)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Build Status](https://github.com/jingzhao/cc_auto_switch/workflows/CI/badge.svg)](https://github.com/jingzhao/cc_auto_switch/actions)
[![Release](https://github.com/jingzhao/cc_auto_switch/workflows/Release/badge.svg)](https://github.com/jingzhao/cc_auto_switch/releases)

A powerful command-line tool for managing multiple Claude API configurations and switching between them effortlessly.

If you've ever used Claude API in different environments (development, testing, production, or even different client accounts), you deeply understand the pain of manually editing configuration files or setting environment variables. cc-switch eliminates this pain by providing a centralized solution:

* **Store multiple API configurations** with easy-to-remember aliases
* **Instantly switch configurations** with a single command
* **Maintain separate settings for different projects or environments**
* **Preserve existing Claude settings** while only modifying API-related configurations

## 🏗️ Core Architecture

The tool is built with a clean, modular architecture that effectively separates concerns:

The application follows a simple yet powerful design pattern, with the main entry point delegating tasks to a command module that handles all CLI operations. `ConfigStorage` manages the persistence of configurations, while `ClaudeSettings` handles integration with Claude's native configuration system.

## 🎯 Key Features

cc-switch comes packed with features that make API configuration management effortless:

| Feature | Description | Benefits |
|---------|-------------|----------|
| **Multi-Configuration Management** | Store unlimited API configurations using custom aliases | Keep all environments organized |
| **Instant Switching** | Switch configurations with `cc-switch switch <alias>` | Save time from manual configuration changes |
| **Shell Auto-Completion** | Built-in completion support for fish, zsh, bash, and more | Speed up command entry with auto-completion |
| **Dynamic Alias Completion** | Auto-complete configuration names for switch/remove commands | Reduce errors and typing effort |
| **Shell Alias Generation** | Generate eval-compatible aliases for quick access | Streamline workflow with convenient shortcuts |
| **Secure Storage** | Configurations securely stored in `~/.cc-switch/` directory | Your API keys are kept separate and organized |
| **Cross-Platform Support** | Supports Linux, macOS, and Windows | Use the same tool across all development environments |
| **Custom Directory Support** | Supports custom Claude settings directories | Flexibility for non-standard installations |

## ⚡ 3-Minute Quick Start

The beauty of cc-switch lies in its simplicity. Here are the steps to get up and running quickly:

1. **Install the tool** (~30 seconds):
   ```bash
   cargo install cc-switch
   ```

2. **Add your first configuration** (~15 seconds):
   ```bash
   cc-switch add my-project sk-ant-xxx https://api.anthropic.com
   ```

3. **Switch to your configuration** (~5 seconds):
   ```bash
   cc-switch switch my-project
   ```

4. **Verify it works** (~10 seconds):
   ```bash
   cc-switch current
   ```

That's it! You're now managing Claude API configurations like a pro.

## 🐚 Shell Integration

cc-switch provides powerful shell integration features to streamline your workflow:

### Shell Aliases

Generate convenient aliases for faster access:

```bash
# Generate aliases for your shell (fish, zsh, bash)
cc-switch alias fish

# Load aliases immediately in your current session
eval "$(cc-switch alias fish)"
```

Available aliases:
- `cs='cc-switch'` - Quick access to cc-switch commands
- `ccd='claude --dangerously-skip-permissions'` - Fast Claude launch

**Example usage with aliases:**
```bash
# Instead of: cc-switch switch my-config
cs switch my-config

# Instead of: claude --dangerously-skip-permissions
ccd
```

### Shell Completion

Set up auto-completion for your shell:

```bash
# Fish shell
cc-switch completion fish > ~/.config/fish/completions/cc-switch.fish

# Zsh shell  
cc-switch completion zsh > ~/.zsh/completions/_cc-switch

# Bash shell
cc-switch completion bash > ~/.bash_completion.d/cc-switch
```

### Permanent Setup

For permanent alias setup, add to your shell config:

**Fish (~/.config/fish/config.fish):**
```bash
alias cs='cc-switch'
alias ccd='claude --dangerously-skip-permissions'
```

**Zsh (~/.zshrc):**
```bash
alias cs='cc-switch'
alias ccd='claude --dangerously-skip-permissions'
```

**Bash (~/.bashrc or ~/.bash_profile):**
```bash
alias cs='cc-switch'
alias ccd='claude --dangerously-skip-permissions'
```

## 🌟 Real-World Use Cases

cc-switch excels in several common development scenarios:

### Multi-Environment Development

```bash
# Set up different environments
cc-switch add dev sk-ant-dev-xxx https://api.anthropic.com
cc-switch add staging sk-ant-staging-xxx https://api.anthropic.com
cc-switch add prod sk-ant-prod-xxx https://api.anthropic.com

# Switch between environments as needed
cc-switch switch dev      # Development work
cc-switch switch staging  # Testing
cc-switch switch prod     # Production deployment
cc-switch switch cc       # Reset to default
```

### Client Project Management

For developers who work with multiple clients requiring different API credentials:

```bash
cc-switch add client-a sk-ant-client-a https://api.anthropic.com
cc-switch add client-b sk-ant-client-b https://api.anthropic.com
cc-switch add personal sk-ant-personal https://api.anthropic.com
```

### Team Collaboration

Team members can share configuration aliases and quickly switch between team-specific settings without manually editing files.

## 🔧 Technical Foundation

cc-switch is built with modern Rust practices and leverages several key libraries:

* **clap** for robust command-line argument parsing with auto-generated help
* **clap_complete** for shell completion script generation
* **serde** for reliable JSON serialization/deserialization
* **dirs** for cross-platform directory management
* **anyhow** for comprehensive error handling
* **colored** for terminal output formatting

The tool is designed with a **zero-configuration** philosophy - it works out of the box with sensible defaults but provides customization options when needed.

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

#### Generate Shell Aliases

```bash
# Generate aliases for immediate use with eval
cc-switch alias fish

# Generate aliases for different shells
cc-switch alias zsh
cc-switch alias bash

# Load aliases immediately (recommended)
eval "$(cc-switch alias fish)"
```

#### Generate Shell Completion

```bash
# Generate completion scripts for your shell
cc-switch completion fish  > ~/.config/fish/completions/cc-switch.fish
cc-switch completion zsh   > ~/.zsh/completions/_cc-switch
cc-switch completion bash  > ~/.bash_completion.d/cc-switch
```

## 🛠️ Development and Build Process

The project includes a comprehensive build process that supports cross-platform compilation, making it simple to build for multiple targets:

This ensures cc-switch can be distributed on all major platforms and maintains consistent behavior.

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

---
**Made with ❤️ by [jingzhao](https://github.com/jingzhao)**# Test change for version management
