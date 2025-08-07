# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`cc-switch` is a Rust CLI tool for managing multiple Claude API configurations and automatically switching between them. The tool allows users to store different API configurations (alias, token, URL) and switch between them by modifying Claude's settings.json file. This is particularly useful for developers who work with multiple Claude API endpoints or need to switch between different accounts.

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

### Pre-commit Hooks
```bash
# Setup pre-commit hooks (one-time setup)
./scripts/setup-pre-commit.sh

# Run pre-commit hooks manually
pre-commit run --all-files

# Run pre-commit on specific files
pre-commit run --files src/main.rs

# Update pre-commit hooks
pre-commit autoupdate

# Uninstall pre-commit hooks
pre-commit uninstall
```

### Version Management & Publishing

The project includes automated version management and publishing to crates.io:

**Complete Release Workflow**:
```bash
# Run complete release workflow (version increment + commit + publish)
./scripts/release.sh
```

**Manual Version Management**:
```bash
# Increment version manually
./scripts/increment-version.sh

# Publish to crates.io manually
./scripts/publish.sh
```

**Version Format**: Uses semantic versioning (x.y.z) where:
- Major version (x): Breaking changes
- Minor version (y): New features
- Patch version (z): Bug fixes and patches

**Automated Workflow**:
1. Make code changes
2. `./scripts/release.sh` - Handles version increment, commit, and publish
3. Version is automatically incremented in Cargo.toml
4. Tests are run automatically
5. Package is automatically published to crates.io

**Manual Workflow**:
1. Make code changes
2. `./scripts/increment-version.sh` - Increment version
3. `git add . && git commit -m "Your message"`
4. `cargo test` - Run tests
5. `./scripts/publish.sh` - Publish to crates.io

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
│   └── cmd/
│       ├── main.rs         # Core CLI logic and data structures
│       ├── mod.rs          # Module declarations
│       ├── tests.rs        # Unit tests for core functionality
│       ├── error_handling_tests.rs  # Error handling edge cases
│       └── integration_tests.rs      # Integration tests
├── .github/
│   └── workflows/
│       ├── ci.yml          # CI pipeline with cross-platform builds
│       └── release.yml     # Release workflow for GitHub releases
└── target/                 # Build output directory (ignored by git)
```

## Architecture Overview

### Core Components

**Configuration Management**:
- `ConfigStorage`: Manages persistence of multiple API configurations in `~/.cc-switch/configurations.json`
- `Configuration`: Represents a single API config with alias, token, and URL
- `ClaudeSettings`: Handles Claude's settings.json file for environment variable configuration

**CLI Interface**:
- `Cli`: Main command parser using clap with subcommands
- `Commands`: Enum defining available subcommands (add, remove, list, set-default-dir, completion, alias, use, current)
- `handle_switch_command()`: Core logic for switching between configurations

### Key Data Flow

1. **Configuration Storage**: Uses JSON serialization to store configurations in `~/.cc-switch/configurations.json`
2. **Settings Modification**: Reads/writes Claude's settings.json to update `ANTHROPIC_AUTH_TOKEN` and `ANTHROPIC_BASE_URL`
3. **Path Resolution**: Supports both absolute and relative paths for custom Claude settings directories

### CLI Usage Patterns

```bash
# Add a new configuration
cc-switch add my-config sk-ant-xxx https://api.anthropic.com

# Switch to a configuration (formerly "switch" command)
cc-switch use my-config

# Reset to default (remove API config)
cc-switch use cc

# List all configurations
cc-switch list

# Set custom Claude settings directory
cc-switch set-default-dir /path/to/claude/config

# Generate shell completion
cc-switch completion fish
cc-switch completion zsh
cc-switch completion bash
```

## Shell Completion Setup

### Fish Shell
```bash
# Generate completion script
cargo run -- completion fish > ~/.config/fish/completions/cc-switch.fish

# Restart fish or reload completions
source ~/.config/fish/config.fish
```

### Zsh Shell
```bash
# Create completions directory if it doesn't exist
mkdir -p ~/.zsh/completions

# Generate completion script
cargo run -- completion zsh > ~/.zsh/completions/_cc-switch

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
cargo run -- completion bash > ~/.bash_completion.d/cc-switch

# Add to ~/.bashrc if not already present
echo 'source ~/.bash_completion.d/cc-switch' >> ~/.bashrc

# Reload shell configuration
source ~/.bashrc
```

## Completion Features

The shell completion provides:
- **Command completion**: `cc-switch <TAB>` shows all subcommands
- **Subcommand completion**: `cc-switch completion <TAB>` shows available shells
- **Configuration alias completion**: `cc-switch use <TAB>` shows stored configuration names
- **Option completion**: `cc-switch -<TAB>` shows available options
- **Help completion**: Context-aware help for all commands and options

## Pre-commit Hooks

The project includes pre-commit hooks that run automatically before each commit to ensure code quality:

### Required Checks (Run on every commit)
- **cargo check**: Verifies code compilation
- **cargo fmt --check**: Ensures code formatting compliance
- **cargo clippy -- -D warnings**: Runs linting with warnings as errors
- **cargo test**: Executes all tests
- **cargo audit**: Security vulnerability scanning
- **cargo doc --no-deps**: Validates documentation builds

### Setup Instructions
```bash
# One-time setup
./scripts/setup-pre-commit.sh

# Manual testing
pre-commit run --all-files

# Skip hooks (if needed)
git commit --no-verify
```

### Development Environment

- **Rust Version**: 1.88.0 or later
- **Rust Edition**: 2024 (using nightly-2024-12-01 toolchain in CI)
- **Cargo Version**: 1.88.0 or later
- **Dependencies**: anyhow (error handling), clap (CLI parsing with completion), clap_complete (shell completion), serde (JSON), dirs (directory paths), colored (terminal output), tempfile (testing)
- **Pre-commit**: Python-based pre-commit framework (auto-installed)

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

- **App Config**: `~/.cc-switch/configurations.json`
- **Claude Settings**: `~/.claude/settings.json` (default) or custom directory
- **Path Resolution**: Supports both absolute paths and home-relative paths

## Important Implementation Details

### Command Changes (Previously Implemented)
- **"switch" → "use"**: The main command changed from `switch` to `use` for clarity
- **Backward Compatibility**: The `switch` command is still available as an alias
- **Completion Logic**: Shell completion prioritizes "current" configuration for c-prefixed commands

### Error Handling
- Uses `anyhow` for comprehensive error handling with context
- All file operations include proper error context for debugging
- Graceful handling of missing files (creates defaults)

### Configuration Switching
- The `use cc` command removes API configuration entirely (resets to default)
- Preserves other settings in Claude's settings.json when modifying API config
- Validates configuration existence before switching

### Shell Integration
- Dynamic completion for configuration aliases
- Custom fish completion with `--list-aliases` functionality
- Alias generation for common workflows (`cs`, `ccd`)

### Cross-Platform Support
- Uses `dirs` crate for cross-platform directory resolution
- Handles file path differences between Windows, Linux, and macOS
- CI builds for multiple target architectures

## Testing Strategy

### Test Coverage
- **Unit Tests**: 57 tests covering all core functionality
- **Integration Tests**: Full workflow testing, error scenarios, edge cases
- **Error Handling Tests**: Comprehensive error condition testing
- **Cross-Platform Tests**: Path resolution, file operations on different platforms

### Test Categories
1. **Configuration Management**: CRUD operations, validation, serialization
2. **Settings Management**: JSON handling, environment variable management
3. **CLI Parsing**: Command structure, argument validation, help generation
4. **Error Handling**: Invalid inputs, file operations, edge cases
5. **Integration**: End-to-end workflows, shell integration

## Key Dependencies and Their Roles

- **anyhow**: Context-rich error handling and propagation
- **clap**: Command-line argument parsing with auto-generated help and completion
- **clap_complete**: Shell completion script generation
- **serde**: JSON serialization/deserialization with derive macros
- **dirs**: Cross-platform directory resolution (home, config directories)
- **tempfile**: Temporary file management for testing
- **colored**: Terminal output formatting and colors

## Common Development Tasks

### Adding New Commands
1. Add variant to `Commands` enum in `src/cmd/main.rs`
2. Implement command handler function
3. Add match arm in `run()` function
4. Write comprehensive tests
5. Update documentation in README.md

### Modifying Configuration Structure
1. Update `Configuration` struct
2. Update serialization/deserialization logic if needed
3. Modify storage operations
4. Update tests to reflect changes
5. Test backward compatibility

### Adding New Shell Support
1. Add shell variant to `generate_completion()` function
2. Implement shell-specific completion logic
3. Add to `generate_aliases()` if supported
4. Update documentation
5. Test completion functionality

## Important Notes for Future Development

1. **Backward Compatibility**: Maintain compatibility with existing configuration files
2. **Error Context**: Provide detailed error messages with context for debugging
3. **Cross-Platform**: Test on all supported platforms (Linux, macOS, Windows)
4. **Security**: Handle API tokens securely, avoid logging sensitive data
5. **Testing**: Maintain high test coverage (currently 100% with 57 tests)
6. **Documentation**: Keep README.md and CLAUDE.md synchronized with code changes