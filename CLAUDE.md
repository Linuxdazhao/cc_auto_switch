# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`cc_auto_switch` is a Rust CLI tool for managing multiple Claude API configurations and automatically switching between them. The tool allows users to store different API configurations (alias, token, URL) and switch between them by modifying Claude's settings.json file. This is particularly useful for developers who work with multiple Claude API endpoints or need to switch between different accounts.

## Development Commands

### Building and Running
```bash
# Build the project
cargo build

# Run the project
cargo run

# Build in release mode
cargo build --release

# Run with release optimization
cargo run --release
```

### Testing
```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

### Code Quality
```bash
# Check for compilation errors
cargo check

# Format code
cargo fmt

# Lint code
cargo clippy

# Lint with all warnings
cargo clippy -- -W warnings

# Run security audit
cargo audit
```

### Dependencies
```bash
# Update dependencies
cargo update

# Check for outdated dependencies
cargo outdated

# Add new dependency
cargo add dependency_name

# Remove dependency
cargo remove dependency_name
```

## Project Structure

```
cc_auto_switch/
├── Cargo.toml              # Project configuration and dependencies
├── Cargo.lock              # Dependency lock file
├── src/
│   ├── main.rs             # Main application entry point (minimal)
│   └── cmd.rs              # Core CLI logic and data structures
├── .github/
│   └── workflows/
│       ├── ci.yml          # CI pipeline with cross-platform builds
│       └── release.yml     # Release workflow for GitHub releases
└── target/                 # Build output directory (ignored by git)
```

## Architecture Overview

### Core Components

**Configuration Management**:
- `ConfigStorage`: Manages persistence of multiple API configurations in `~/.cc_auto_switch/configurations.json`
- `Configuration`: Represents a single API config with alias, token, and URL
- `ClaudeSettings`: Handles Claude's settings.json file for environment variable configuration

**CLI Interface**:
- `Cli`: Main command parser using clap with subcommands and `-c` switch flag
- `Commands`: Enum defining available subcommands (add, remove, list, set-default-dir)
- `handle_switch_command()`: Core logic for switching between configurations

### Key Data Flow

1. **Configuration Storage**: Uses JSON serialization to store configurations in `~/.cc_auto_switch/configurations.json`
2. **Settings Modification**: Reads/writes Claude's settings.json to update `ANTHROPIC_AUTH_TOKEN` and `ANTHROPIC_BASE_URL`
3. **Path Resolution**: Supports both absolute and relative paths for custom Claude settings directories

### CLI Usage Patterns

```bash
# Add a new configuration
cc_auto_switch add my-config sk-ant-xxx https://api.anthropic.com

# Switch to a configuration
cc_auto_switch -c my-config

# Reset to default (remove API config)
cc_auto_switch -c cc

# List all configurations
cc_auto_switch list

# Set custom Claude settings directory
cc_auto_switch set-default-dir /path/to/claude/config

# Generate shell completion
cc_auto_switch completion fish
cc_auto_switch completion zsh
cc_auto_switch completion bash
```

## Shell Completion Setup

### Fish Shell
```bash
# Generate completion script
cargo run -- completion fish > ~/.config/fish/completions/cc_auto_switch.fish

# Restart fish or reload completions
source ~/.config/fish/config.fish
```

### Zsh Shell
```bash
# Create completions directory if it doesn't exist
mkdir -p ~/.zsh/completions

# Generate completion script
cargo run -- completion zsh > ~/.zsh/completions/_cc_auto_switch

# Add to ~/.zshrc if not already present
echo 'fpath=(~/.zsh/completions $fpath)' >> ~/.zshrc

# Reload shell configuration
source ~/.zshrc

# Force rebuild completion cache
compinit
```

### Bash Shell
```bash
# Generate completion script
cargo run -- completion bash > ~/.bash_completion.d/cc_auto_switch

# Add to ~/.bashrc if not already present
echo 'source ~/.bash_completion.d/cc_auto_switch' >> ~/.bashrc

# Reload shell configuration
source ~/.bashrc
```

## Completion Features

The shell completion provides:
- **Command completion**: `cc_auto_switch <TAB>` shows all subcommands
- **Subcommand completion**: `cc_auto_switch completion <TAB>` shows available shells
- **Configuration alias completion**: `cc_auto_switch -c <TAB>` shows stored configuration names
- **Option completion**: `cc_auto_switch -<TAB>` shows available options
- **Help completion**: Context-aware help for all commands and options

## Development Environment

- **Rust Version**: 1.88.0 or later
- **Rust Edition**: 2024 (using nightly-2024-12-01 toolchain in CI)
- **Cargo Version**: 1.88.0 or later
- **Dependencies**: anyhow (error handling), clap (CLI parsing with completion), clap_complete (shell completion), serde (JSON), dirs (directory paths)

## CI/CD Pipeline

### CI Workflow (.github/workflows/ci.yml)
- **Multi-platform testing**: Ubuntu, Windows, macOS
- **Cross-compilation**: Builds for x86_64 and aarch64 architectures
- **Code quality**: Formatting checks, clippy linting, security audit
- **Coverage**: Code coverage reporting with codecov

### Release Workflow (.github/workflows/release.yml)
- **Multi-architecture releases**: Linux, Windows, macOS (Intel and ARM)
- **Automated packaging**: Creates tar.gz artifacts for each target
- **GitHub releases**: Automated release creation with changelog

## File Storage Locations

- **App Config**: `~/.cc_auto_switch/configurations.json`
- **Claude Settings**: `~/.claude/settings.json` (default) or custom directory
- **Path Resolution**: Supports both absolute paths and home-relative paths

## Important Implementation Details

### Error Handling
- Uses `anyhow` for comprehensive error handling with context
- All file operations include proper error context for debugging
- Graceful handling of missing files (creates defaults)

### Configuration Switching
- The `-c cc` command removes API configuration entirely (resets to default)
- Preserves other settings in Claude's settings.json when modifying API config
- Validates configuration existence before switching

### Cross-Platform Support
- Uses `dirs` crate for cross-platform directory resolution
- Handles file path differences between Windows, Linux, and macOS
- CI builds for multiple target architectures