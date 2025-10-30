# CLAUDE.md

This file provides guidance for Claude Code (claude.ai/code) when working in this codebase.

## Project Overview

`cc-switch` is a Rust CLI tool for managing multiple Claude API configurations and automatically switching between them. The tool allows users to store different API configurations (alias, token, URL) and switch between them by modifying Claude's settings.json file. This is particularly useful for developers who need to use multiple Claude API endpoints or switch between different accounts.

## Development Commands

### Build and Run

```bash
# Build project
cargo build

# Run project
cargo run

# Release build
cargo build --release

# Release run
cargo run --release
```

### Testing

```bash
# Run all tests
cargo nextest run

```

### Code Quality

```bash
# Check compilation errors
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
# One-time setup
./scripts/setup-pre-commit.sh

# Manual run on all files
pre-commit run --all-files

# Run on specific files
pre-commit run --files src/main.rs

# Update hooks
pre-commit autoupdate

# Uninstall hooks
pre-commit uninstall
```

### Version Management and Release

The project includes automated version management and publishing to crates.io:

**Complete Release Workflow**:

```bash
# Run full release workflow (version increment + commit + publish)
./scripts/release.sh
```

**Manual Version Management**:

```bash
# Manually increment version
./scripts/increment-version.sh

# Manually publish to crates.io
./scripts/publish.sh
```

**Version Format**: Uses semantic versioning (x.y.z) where:

- Major version (x): Breaking changes
- Minor version (y): New features
- Patch version (z): Bug fixes and patches

**Automated Workflow**:

1. Make code changes
2. `./scripts/release.sh` - Handles version increment, commit, and publish
3. Version in Cargo.toml is auto-incremented
4. Tests run automatically
5. Package is auto-published to crates.io

**Manual Workflow**:

1. Make code changes
2. `./scripts/increment-version.sh` - Increment version
3. `git add . && git commit -m "Your message"`
4. `cargo nextest` - Run tests
5. `./scripts/publish.sh` - Publish to crates.io

### Dependency Management

```bash
# Update dependencies
cargo update

# Check outdated dependencies
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
│       ├── main.rs         # Core CLI logic and orchestration
│       ├── mod.rs          # Module declarations
│       ├── cli.rs          # Command-line interface definitions
│       ├── types.rs        # Core data structures and types
│       ├── config.rs       # Configuration management logic
│       ├── config_storage.rs # Configuration persistence and storage
│       ├── interactive.rs  # Interactive menus and terminal UI
│       ├── completion.rs   # Shell completion logic
│       ├── shell_completion.rs # Shell-specific completion handlers
│       ├── utils.rs        # Utility functions
│       ├── tests.rs        # Core functionality unit tests
│       ├── error_handling_tests.rs  # Error handling edge cases
│       └── integration_tests.rs      # Integration tests
├── .github/
│   └── workflows/
│       ├── ci.yml          # CI pipeline and cross-platform builds
│       └── release.yml     # GitHub release workflow
└── target/                 # Build output directory (git ignored)
```

## Architecture Overview

### Core Components

**Configuration Management** (`config.rs`, `config_storage.rs`, `types.rs`):

- `ConfigStorage`: Manages persistence of multiple API configurations in `~/.cc-switch/configurations.json`
- `Configuration`: Represents a single API configuration, including alias, token, URL, model, and small_fast_model
- `ClaudeSettings`: Handles environment variable configuration in Claude's settings.json file
- `AddCommandParams`: Parameter structure for add command, supports interactive mode

**CLI Interface** (`cli.rs`):

- `Cli`: Main command parser using clap, supports subcommands and hidden completion flags
- `Commands`: Enum defining available subcommands (add, remove, list, set-default-dir, completion, alias, use, current)
- Rich help text with examples and Shell integration instructions

**Interactive Terminal UI** (`interactive.rs`):

- `handle_current_command()`: Interactive main menu with keyboard navigation
- `handle_interactive_selection()`: Real-time configuration browser with preview
- **Number Key Quick Selection**: Press number keys 1-9 to directly select corresponding configurations
- **Smart Pagination System**: Auto-paginate when >9 configurations, supports PageUp/PageDown or N/P keys
- **Shortcut Key Support**: R key to reset to official config, E key to exit
- Crossterm-based terminal handling with graceful degradation to simple menus
- Auto-launch Claude CLI after configuration switch

**Shell Integration** (`completion.rs`, `shell_completion.rs`):

- Dynamic completion for configuration aliases
- Shell-specific completion handlers for fish, zsh, bash, elvish, powershell
- Eval-compatible alias generation system

### Key Data Flow

1. **Configuration Storage**: Uses JSON serialization to store configurations in `~/.cc-switch/configurations.json`
2. **Settings Modification**: Read/write Claude's settings.json to update `ANTHROPIC_AUTH_TOKEN` and `ANTHROPIC_BASE_URL`
3. **Path Resolution**: Supports both absolute and relative paths for custom Claude settings directory

### CLI Usage Patterns

```bash
# Add configurations (multiple formats supported)
cc-switch add my-config sk-ant-xxx https://api.anthropic.com
cc-switch add my-config -t sk-ant-xxx -u https://api.anthropic.com
cc-switch add my-config -t sk-ant-xxx -u https://api.anthropic.com -m claude-3-5-sonnet-20241022
cc-switch add my-config -t sk-ant-xxx -u https://api.anthropic.com --small-fast-model claude-3-haiku-20240307
cc-switch add my-config -i  # Interactive mode
cc-switch add my-config --force  # Overwrite existing config

# Switch configurations
cc-switch use my-config
cc-switch use -a my-config
cc-switch use --alias my-config
cc-switch use  # Interactive mode

# Interactive current configuration menu
cc-switch current  # Shows current config + interactive menu

# Reset to default (remove API config)
cc-switch use cc

# List all configurations
cc-switch list

# Manage multiple configurations
cc-switch remove config1 config2 config3

# Set custom Claude settings directory
cc-switch set-default-dir /path/to/claude/config

# Shell integration
cc-switch completion fish  # Generate completion scripts
cc-switch alias fish       # Generate eval-compatible aliases
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

## Interactive Features

### Current Command Interactive Menu

The `cc-switch current` command provides a sophisticated interactive menu with:

- **Current Configuration Display**: Shows active API token and URL
- **Keyboard Navigation**: Arrow keys for menu navigation (with fallback to numbered menu)
- **Number Key Quick Selection**: Press number keys 1-9 to directly select corresponding configurations
- **Smart Pagination**: Auto-paginate when >9 configurations, display up to 9 per page
- **Page Navigation**: PageUp/PageDown or N/P keys for quick page switching
- **Quick Actions**: R key to reset to official config, E key to exit
- **Real-time Selection**: Instant preview of configuration details during browsing
- **Automatic Claude Launch**: Seamlessly launches Claude CLI after configuration switches
- **Terminal Compatibility**: Crossterm-based terminal handling with graceful fallbacks

### Interactive Selection Mode

- **Visual Configuration Browser**: Browse all stored configurations with full details
- **Configuration Preview**: See token, URL, model settings before switching
- **Reset Option**: Quick reset to default Claude behavior
- **Smart Fallbacks**: Automatic fallback to simple menus when terminal capabilities are limited

### Keyboard Shortcuts Reference

#### Single Page Mode (≤9 configurations)

- **↑↓**: Navigate up/down
- **1-9**: Number keys to directly select configuration
- **R**: Reset to official configuration
- **E**: Exit program
- **Enter**: Confirm current selection
- **Esc**: Cancel operation

#### Pagination Mode (>9 configurations)

- **↑↓**: Navigate up/down
- **1-9**: Number keys to directly select configuration on current page
- **N/PageDown**: Next page
- **P/PageUp**: Previous page
- **R**: Reset to official configuration (available on any page)
- **E**: Exit program
- **Enter**: Confirm current selection

## Completion Features

The shell completion provides:

- **Command completion**: `cc-switch <TAB>` shows all subcommands
- **Subcommand completion**: `cc-switch completion <TAB>` shows available shells
- **Configuration alias completion**: `cc-switch use <TAB>` shows stored configuration names
- **Option completion**: `cc-switch -<TAB>` shows available options
- **Help completion**: Context-aware help for all commands and options
- **Dynamic alias loading**: Completion system dynamically loads available configuration names

## Pre-commit Hooks

The project includes pre-commit hooks that run automatically before each commit to ensure code quality:

### Required Checks (Run on every commit)

- **cargo check**: Verifies code compilation
- **cargo fmt --check**: Ensures code formatting compliance
- **cargo clippy -- -D warnings**: Runs linting with warnings as errors
- **cargo nextest**: Executes all tests
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
- **Dependencies**: anyhow (error handling), clap (CLI parsing with completion), clap_complete (shell completion), serde (JSON), dirs (directory paths), colored (terminal output), crossterm (terminal UI), tempfile (testing)
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

### Major Architecture Changes

- **Modular Structure**: Codebase refactored from single file to multi-module architecture
- **Interactive Terminal UI**: Full terminal-based interactive menus with keyboard navigation
- **Enhanced Configuration**: Support for model and small_fast_model environment variables
- **Real-time Preview**: Interactive selection shows full configuration details before switching
- **Auto-launching**: Automatic Claude CLI execution after configuration switches

### Command Evolution

- **"switch" → "use"**: The main command changed from `switch` to `use` for clarity
- **Backward Compatibility**: The `switch` command is still available as an alias
- **Interactive Modes**: Both `use` and `current` commands support interactive selection
- **Enhanced Add Command**: Support for positional and flag-based arguments with interactive mode

### Error Handling

- Uses `anyhow` for comprehensive error handling with context
- All file operations include proper error context for debugging
- Graceful handling of missing files (creates defaults)

### Configuration Switching

- The `use cc` command removes API configuration entirely (resets to default)
- Preserves other settings in Claude's settings.json when modifying API config
- Validates configuration existence before switching

### Shell Integration

- Dynamic completion for configuration aliases with real-time loading
- Multi-shell support: fish, zsh, bash, elvish, powershell
- Alias generation system: `cs='cc-switch'` and `ccd='claude --dangerously-skip-permissions'`
- Hidden `--list-aliases` flag for programmatic access
- Eval-compatible alias output for immediate shell integration

### Cross-Platform Support

- Uses `dirs` crate for cross-platform directory resolution
- Handles file path differences between Windows, Linux, and macOS
- CI builds for multiple target architectures

## Testing Strategy

### Test Coverage

- **Unit Tests**: 43 tests covering all core functionality
- **Integration Tests**: Full workflow testing, error scenarios, edge cases
- **Error Handling Tests**: Comprehensive error condition testing including boundary cases
- **Interactive Feature Tests**: Number key quick selection, pagination logic, boundary condition testing
- **Cross-Platform Tests**: Path resolution, file operations on different platforms

### Test Categories

1. **Configuration Management**: CRUD operations, validation, serialization
2. **Settings Management**: JSON handling, environment variable management
3. **CLI Parsing**: Command structure, argument validation, help generation
4. **Error Handling**: Invalid inputs, file operations, edge cases
5. **Integration**: End-to-end workflows, shell integration
6. **Interactive Features**:
   - Pagination calculation and navigation logic testing
   - Number key mapping and boundary condition testing
   - Empty configuration list and exception handling testing

## Key Dependencies and Their Roles

- **anyhow**: Context-rich error handling and propagation
- **clap**: Command-line argument parsing with auto-generated help and completion
- **clap_complete**: Shell completion script generation
- **serde**: JSON serialization/deserialization with derive macros
- **dirs**: Cross-platform directory resolution (home, config directories)
- **colored**: Terminal output formatting and colors
- **crossterm**: Cross-platform terminal manipulation and events (keyboard navigation, raw mode)
- **tempfile**: Temporary file management for testing

## Common Development Tasks

### Adding New Commands

1. Add variant to `Commands` enum in `src/cmd/cli.rs`
2. Implement command handler function in appropriate module (`src/cmd/main.rs` or dedicated module)
3. Add match arm in `run()` function in `src/cmd/main.rs`
4. Add completion logic if needed in `src/cmd/completion.rs`
5. Write comprehensive tests in appropriate test module
6. Update help text and documentation

### Modifying Configuration Structure

1. Update `Configuration` struct
2. Update serialization/deserialization logic if needed
3. Modify storage operations
4. Update tests to reflect changes
5. Test backward compatibility

### Adding New Shell Support

1. Add shell variant to `generate_completion()` function in `src/cmd/completion.rs`
2. Implement shell-specific completion logic in `src/cmd/shell_completion.rs`
3. Add to `generate_aliases()` function for alias support
4. Update help text in `src/cmd/cli.rs`
5. Test completion and alias functionality across platforms

## Important Notes for Future Development

1. **Backward Compatibility**: Maintain compatibility with existing configuration files
2. **Error Context**: Provide detailed error messages with context for debugging
3. **Cross-Platform**: Test on all supported platforms (Linux, macOS, Windows)
4. **Security**: Handle API tokens securely, avoid logging sensitive data
5. **Testing**: Maintain high test coverage (currently 100% with 57 tests)
6. **Documentation**: Keep README.md and CLAUDE.md synchronized with code changes
7. **Git Push Validation**: Before pushing to GitHub, ensure all CI/CD workflows pass locally by running:
   - `cargo nextest` - All tests must pass
   - `cargo clippy -- -W warnings` - No clippy warnings
   - `cargo fmt --check` - Code must be formatted
   - `cargo audit` - No security vulnerabilities
   - Verify `.github/workflows/` configuration files are valid and workflow will succeed
