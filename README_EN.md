# cc-switch

[![Crates.io](https://img.shields.io/crates/v/cc-switch.svg)](https://crates.io/crates/cc-switch)
[![GitHub Packages](https://img.shields.io/badge/GitHub-Packages-green)](https://github.com/jingzhao/cc_auto_switch/packages)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Build Status](https://github.com/jingzhao/cc_auto_switch/workflows/CI/badge.svg)](https://github.com/jingzhao/cc_auto_switch/actions)
[![Release](https://github.com/jingzhao/cc_auto_switch/workflows/Release/badge.svg)](https://github.com/jingzhao/cc_auto_switch/releases)

A powerful command-line tool for managing multiple Claude API configurations and switching between them effortlessly through environment variables.

If you've ever used Claude API in different environments (development, testing, production, or even different client accounts), you deeply understand the pain of manually setting environment variables or managing different configurations. cc-switch eliminates this pain by providing a centralized environment variable management solution:

* **Store multiple API configurations** with easy-to-remember aliases
* **Instantly switch configurations** by launching Claude with environment variables safely
* **Maintain separate settings for different projects or environments**
* **Environment variable isolation** with independent environment configuration for each execution

## 🏗️ Core Architecture

The tool is built with a clean, modular architecture that effectively separates concerns:

The application follows a simple yet powerful design pattern, with the main entry point delegating tasks to a command module that handles all CLI operations. `ConfigStorage` manages the persistence of configurations, while `EnvironmentConfig` handles integration with environment variables, ensuring configuration isolation by setting specific environment variables for each command execution.

## 🎯 Key Features

cc-switch comes packed with features that make API configuration management effortless:

| Feature | Description | Benefits |
|---------|-------------|----------|
| **Multi-Configuration Management** | Store unlimited API configurations using custom aliases | Keep all environments organized |
| **Environment Variable Switching** | Switch configurations with `cc-switch use <alias>` via environment variables | Safe isolation, no global settings affected |
| **Interactive Selection Mode** | Visual menu with real-time configuration preview | Browse configurations with full details before switching |
| **Shell Auto-Completion** | Built-in completion support for fish, zsh, bash, and more | Speed up command entry with auto-completion |
| **Dynamic Alias Completion** | Auto-complete configuration names for switch/remove commands | Reduce errors and typing effort |
| **Shell Alias Generation** | Generate eval-compatible aliases for quick access | Streamline workflow with convenient shortcuts |
| **Secure Storage** | Configurations securely stored in `~/.cc-switch/` directory | Your API keys are kept separate and organized |
| **Cross-Platform Support** | Supports Linux and macOS | Use the same tool across all major development environments |
| **Environment Variable Isolation** | Independent environment variables per execution | Avoid global configuration conflicts |

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
   cc-switch use my-project
   ```

4. **Verify it works** (~10 seconds):
   ```bash
   cc-switch current
   ```

**Important Change:** The `cc-switch use` command now launches Claude through environment variables rather than modifying global configuration files. This ensures complete configuration isolation and security.

**Tip:** Running `cc-switch` directly (without any arguments) enters the interactive main menu mode, giving you quick access to all features!

## 🔒 Environment Variable Mode

cc-switch now uses environment variable mode for better security and isolation:

- ✅ **Isolation**: Each execution uses independent environment variables, not affecting global system settings
- ✅ **Security**: API keys are not written to any configuration files, only used during command execution
- ✅ **Simplicity**: No need to manage complex configuration files or worry about configuration conflicts
- ✅ **Convenience**: `cc-switch use <alias>` automatically sets environment variables and launches Claude

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
# Instead of: cc-switch use my-config
cs use my-config

# Interactive current menu
cs current

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

# Switch between environments as needed (each launches a new Claude instance)
cc-switch use dev      # Development work
cc-switch use staging  # Testing
cc-switch use prod     # Production deployment
cc-switch use cc       # Launch with default settings
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

* **crossterm** for cross-platform terminal manipulation and interactive UI
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

# Add with model specification (sets ANTHROPIC_MODEL environment variable)
cc-switch add my-config -t sk-ant-xxx -u https://api.anthropic.com -m claude-3-5-sonnet-20241022

# Add with small fast model for background tasks (sets ANTHROPIC_SMALL_FAST_MODEL environment variable)
cc-switch add my-config -t sk-ant-xxx -u https://api.anthropic.com --small-fast-model claude-3-haiku-20240307

# Add with both models (sets both model environment variables)
cc-switch add my-config -t sk-ant-xxx -u https://api.anthropic.com -m claude-3-5-sonnet-20241022 --small-fast-model claude-3-haiku-20240307

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
# Switch to a specific configuration (launches Claude with configuration's environment variables)
cc-switch use my-config

# Launch with default settings (no custom API configuration)
cc-switch use cc
```

**Note:** The `use` command now launches Claude CLI through environment variables, ensuring configuration isolation and security.

#### Current Configuration Interactive Menu

```bash
# Show environment variable mode information and interactive menu
cc-switch current

# Or run directly (enters interactive main menu when no arguments)
cc-switch
```

The `current` command provides an interactive menu with:
- Display of environment variable mode information
- Option 1: Execute `claude --dangerously-skip-permissions`
- Option 2: Switch configuration (with real-time preview and launches Claude)
- Option 3: Exit

Navigation:
- Use **↑↓** arrow keys for menu navigation (or number keys as fallback)
- Press **Enter** to select
- Press **Esc** to exit

#### Interactive Selection Mode

Use the interactive selection to visually browse configurations with real-time preview:

```bash
# Access through current command's menu option
cc-switch current  # Then select option 2

# Direct access to interactive main menu
cc-switch  # No arguments

# Direct access (if supported in your version)
cc-switch use  # Interactive mode when no alias specified
```

In interactive selection mode:
- Use **↑↓** arrow keys to navigate through configurations
- View detailed information (token, URL, model, small-fast-model) for the selected configuration
- Press **Enter** to select and launch Claude with that configuration's environment variables
- Press **Esc** to cancel selection
- Includes "Launch with default settings" option to run Claude without custom API configuration
- Smart fallback to numbered menu if terminal doesn't support advanced features

Interactive mode provides a visual way to browse and select configurations with full details preview before switching, and automatically launches Claude CLI with the selected configuration's environment variables.

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

The project includes a comprehensive build process that supports cross-platform compilation for Linux and macOS, making it simple to build for multiple targets:

This ensures cc-switch can be distributed on Linux and macOS platforms and maintains consistent behavior.

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
