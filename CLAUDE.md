# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`cc-switch` is a Rust CLI tool for managing multiple Claude API configurations and automatically switching between them. The tool allows users to store different API configurations (alias, token, URL, model settings) and switch between them by modifying Claude's settings.json file. This is particularly useful for developers who need to use multiple Claude API endpoints or switch between different accounts.

The project follows Rust best practices with a library + binary structure and domain-based organization.

our program config dir is ~/.claude/cc_auto_switch_setting.json

## Development Commands

### Build and Run

```bash
# Build project
cargo build

# Run project
cargo run [args]

# Release build
cargo build --release

# Run release binary
cargo run --release [args]
```

### Testing

```bash
# Run all tests (library + integration + doctests)
cargo test

# Run only library tests
cargo test --lib

# Run only integration tests
cargo test --tests

# Run specific test file
cargo test --test tests

# Run single test
cargo test test_name

# Run with output
cargo test -- --nocapture test_name

# Run integration tests only
cargo test --test integration_tests

# Run with nextest (if installed)
cargo install cargo-nextest
cargo nextest run
```

### Code Quality

```bash
# Check compilation errors
cargo check

# Format code
cargo fmt

# Format check (CI compatible)
cargo fmt --check

# Lint code
cargo clippy

# Lint with all warnings treated as errors
cargo clippy -- -W warnings

# Run security audit
cargo audit
```

### Pre-commit Hooks (prek)

We use [`prek`](https://github.com/j178/prek), a Rust-native drop-in
replacement for the Python `pre-commit` tool. It reads the same
`.pre-commit-config.yaml`.

```bash
# One-time setup (installs prek + the git hook)
./scripts/setup-pre-commit.sh

# Run on all files
prek run --all-files

# Run a single hook
prek run cargo-fmt --all-files

# Update hook versions
prek autoupdate

# Uninstall the git hook
prek uninstall
```

### Version Management and Release

**Publish ownership**: `.github/workflows/publish.yml` is the **single** publisher
to crates.io. It triggers on every `v*` tag push. Local scripts must never call
`cargo publish` directly — doing so races CI and one side fails with
`crate cc-switch@x.y.z already exists on crates.io index`.

**Complete Release Workflow** (Recommended):

```bash
./scripts/release.sh
```

This bumps the version, commits, tags `v$new_version`, and pushes both `main`
and the tag. The tag push triggers the `publish.yml` workflow, which runs tests
and `cargo publish` from CI. Watch the run at
<https://github.com/Linuxdazhao/cc_auto_switch/actions>.

**Manual Workflow**:

```bash
# 1. Make changes
git add .
git commit -m "Your message"

# 2. Increment version (updates Cargo.toml, commits "Release vX.Y.Z")
./scripts/increment-version.sh

# 3. Run tests locally
cargo test

# 4. Tag and push — CI publishes
git tag "v$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)"
git push origin main --follow-tags
```

**`scripts/publish.sh`** is an **emergency-only** script for publishing from a
developer machine when CI is unavailable. The normal release flow does not
invoke it.

**Version Format**: Semantic versioning (x.y.z)

- Major (x): Breaking changes
- Minor (y): New features
- Patch (z): Bug fixes

**Commit Message Convention** (drives auto-generated release notes):

The `.github/workflows/release.yml` workflow categorises commits by their
Conventional Commits prefix when building the release notes:

- `feat(scope): ...` → **New Features** section
- `fix(scope): ...` → **Bug Fixes** section
- `chore: ...` → skipped (version bumps, lock updates)
- anything else → **Other Changes** section

For breaking changes, append `!` after the type/scope:

- `feat(cli)!: drop -j short for --from-file`
- `fix!: remove deprecated env var`

The workflow detects the `!` marker and renders a **💥 Breaking Changes**
section at the top of the release notes, with a warning banner. Always use
the `!` marker when introducing user-visible breaking changes so users
upgrading via brew/cargo see the warning immediately.

### Dependency Management

```bash
# Update dependencies
cargo update

# Check outdated dependencies
cargo outdated

# Add new dependency
cargo add dependency_name

# Add development dependency
cargo add --dev dependency_name

# Remove dependency
cargo remove dependency_name
```

## Project Structure

```
cc-switch/
├── src/
│   ├── lib.rs                 # Library crate with public API
│   ├── main.rs                # Minimal binary entry point
│   ├── cli/                   # CLI domain
│   │   ├── cli.rs             # Command-line interface definitions
│   │   ├── completion.rs      # Shell completion logic
│   │   ├── display_utils.rs   # Terminal display utilities
│   │   └── main.rs            # CLI command handlers
│   ├── config/                # Configuration domain
│   │   ├── mod.rs             # Module exports
│   │   ├── config.rs          # Configuration management
│   │   ├── config_storage.rs  # Persistent storage
│   │   └── types.rs           # Data structures
│   ├── interactive/           # Interactive UI domain
│   │   ├── mod.rs             # Module exports
│   │   └── interactive.rs     # Terminal UI with keyboard navigation
│   ├── claude_settings.rs     # Claude settings.json management
│   └── utils.rs               # Utility functions
├── tests/                     # Integration tests (not unit tests)
│   ├── integration_tests.rs   # End-to-end workflows
│   ├── main_tests.rs          # CLI main logic tests
│   ├── tests.rs               # Core functionality tests
│   ├── completion_tests.rs    # Shell completion tests
│   ├── interactive_tests.rs   # Interactive UI tests
│   └── error_handling_tests.rs # Error scenarios
├── scripts/                   # Automation scripts
│   ├── release.sh             # Full release workflow
│   ├── increment-version.sh   # Version bumping
│   ├── publish.sh             # Publish to crates.io
│   └── setup-pre-commit.sh    # Pre-commit setup
└── .github/workflows/         # CI/CD pipelines
    ├── ci.yml                 # Multi-platform CI
    └── release.yml            # Release automation
```

## Frontend (web/)

The web dashboards are a **pnpm workspace** under `web/` (pnpm 9+; the
`web/pnpm-lock.yaml` lockfile **is** committed). Tech stack: Svelte 5, Vite 6,
TypeScript, Tailwind CSS 3, shadcn-svelte / bits-ui v2, Vitest.

### Workspace layout

```
web/
├── packages/
│   ├── api  (@ccs/api)   # Shared TS API types + typed fetch client + SSE parser
│   └── ui   (@ccs/ui)    # Design system: Tailwind preset, design tokens
│                         #   (light/dark via .dark class), shadcn-svelte
│                         #   components, custom components (StatusBadge,
│                         #   StatCard, DataTable, FilterGroup, ConversationView
│                         #   with bundled marked/DOMPurify/highlight.js — no CDN)
└── apps/
    ├── aggregate (@ccs/app-aggregate) # Dashboard for the aggregate daemon
    │                                  #   → Vite outDir web-aggregate/dist/
    └── proxy     (@ccs/app-proxy)     # Single-instance viewer for ccs-proxy
                                       #   → Vite outDir ccs-proxy/web/dist/
```

### Build / test / dev

```bash
cd web
pnpm install
pnpm -r build      # build all packages + apps (outputs to the dist/ dirs above)
pnpm -r test       # Vitest across the workspace
pnpm -r check      # type-check / svelte-check
```

`web/pnpm-workspace.yaml` sets `onlyBuiltDependencies: [esbuild]` and
`verifyDepsBeforeRun: false` (the latter avoids a pnpm 11
`ERR_PNPM_IGNORED_BUILDS` pre-run error on esbuild's build script).

**`$lib` alias requirement:** both apps are plain Vite apps (not SvelteKit), but
the shadcn components import `$lib/utils.js`. Each app's `vite.config.ts` adds a
`$lib` resolve alias → `../../packages/ui/src/lib`, with a matching `$lib` path
in each app's `tsconfig`.

### The `web-ui` cargo feature (default OFF)

Both crates expose a `web-ui` cargo feature, **off by default**. The root
`cc-switch` feature propagates to the proxy: `web-ui = ["ccs-proxy/web-ui"]`
(cc-switch runs ccs-proxy in-process via `ccs_proxy::serve`, so the proxy
dashboard ships inside the cc-switch binary).

- `build.rs` in **both** the root crate and `ccs-proxy` is a **no-op unless
  `web-ui` is enabled** (it checks `CARGO_FEATURE_WEB_UI`). When enabled, it
  runs `pnpm --filter <app> build` to produce the embedded `dist/`. With the
  feature off, **no Node/pnpm is required** — for `cargo build`, docs.rs, and
  downstream consumers.
- The embedded dashboards (rust-embed) are feature-gated. With `web-ui` OFF,
  `ui_router()` (aggregate) / `router()` (ccs-proxy) return an empty router and
  no assets are embedded, so a clean build needs no `dist/`.
- The Vite `dist/` outputs (`web-aggregate/dist/`, `ccs-proxy/web/dist/`) are
  **gitignored** and regenerated at build time — never committed.
- Published crates **exclude** web: root `cc-switch` `exclude = ["web/",
  "web-aggregate/dist/"]`; `ccs-proxy` `exclude = ["web/", "tests/fixtures/"]`.
  docs.rs builds with `features = []` (web-ui off), so it needs no Node.

Build a binary **with** the dashboards (requires Node + pnpm + a prior
`pnpm install` in `web/`, since build.rs shells out to pnpm):

```bash
cargo build --release --features web-ui
```

### CI

- `.github/workflows/ci.yml` has a `frontend` job: pnpm install + `pnpm -r test`
  + `pnpm -r build`.
- `.github/workflows/release.yml` installs Node/pnpm + frontend deps, then builds
  the binary with `--features web-ui` (embeds both dashboards).
- `.github/workflows/publish.yml` stays **Rust-only** (no `--features web-ui`);
  the published crate is web-free, and it asserts web exclusion via
  `cargo package --manifest-path ccs-proxy/Cargo.toml --allow-dirty --no-verify --list`.

## Architecture Overview

### Library + Binary Structure

The project is structured as a **library crate** with a **minimal binary entry point**:

- **src/lib.rs**: Declares the library crate with public API exports
- **src/main.rs**: Binary that calls `cc_switch::run()` from the library
- **Benefits**: Enables code reuse, better testing, can be imported by other projects

### Domain-Based Organization

The codebase is organized into three main domains:

#### 1. CLI Domain (`src/cli/`)

**Purpose**: Command-line interface, parsing, shell integration
**Key Components**:

- `cli.rs`: clap-based command parser, Commands enum
- `completion.rs`: Shell completion script generation (fish, zsh, bash, elvish, powershell)
- `main.rs`: Command handlers (add, remove, list, completion, etc.)
- `display_utils.rs`: Terminal text formatting, width calculation, alignment

#### 2. Configuration Domain (`src/config/`)

**Purpose**: Configuration management, persistence, validation
**Key Components**:

- `types.rs`: Data structures (Configuration, ConfigStorage, ClaudeSettings)
- `config.rs`: Environment variable management
- `config_storage.rs`: JSON persistence to `~/.cc-switch/configurations.json`
- Exports convenience functions: `validate_alias_name()`, `get_config_storage_path()`

#### 3. Interactive Domain (`src/interactive/`)

**Purpose**: Terminal-based interactive UI
**Key Components**:

- `interactive.rs`: Crossterm-based terminal UI
- `handle_current_command()`: Main interactive menu
- `handle_interactive_selection()`: Configuration browser with preview
- Features: Number key selection (1-9), smart pagination, keyboard navigation, auto-launch Claude

### Data Flow

1. **Add Configuration** → CLI parses args → Validates → Saves to JSON via ConfigStorage
2. **Switch Configuration** → Interactive mode → Reads config → Updates Claude settings.json → Launches Claude
3. **List Configurations** → Reads from ConfigStorage → Displays (JSON or plain text)
4. **Shell Completion** → Dynamically loads configuration names → Generates shell-specific scripts

### Key Data Types

**Configuration** (in `src/config/types.rs`):

```rust
struct Configuration {
    alias_name: String,
    token: String,
    url: String,
    model: Option<String>,
    small_fast_model: Option<String>,
    max_thinking_tokens: Option<u32>,
    api_timeout_ms: Option<u32>,
    claude_code_disable_nonessential_traffic: Option<u32>,
    anthropic_default_sonnet_model: Option<String>,
    anthropic_default_opus_model: Option<String>,
    anthropic_default_haiku_model: Option<String>,
}
```

**ConfigStorage**:

- Manages multiple Configuration objects
- Persists to `~/.cc-switch/configurations.json`
- Provides CRUD operations

**EnvironmentConfig**:

- Converts Configuration to environment variable tuples
- Used for launching Claude with custom settings

## CLI Usage Patterns

```bash
# Add configurations
cc-switch add my-config sk-ant-xxx https://api.anthropic.com
cc-switch add my-config -t sk-ant-xxx -u https://api.anthropic.com
cc-switch add my-config -t sk-ant-xxx -u https://api.anthropic.com -m claude-3-5-sonnet-20241022
cc-switch add my-config -i  # Interactive mode
cc-switch add my-config --from-file                   # Import from ~/.claude/settings.json
cc-switch add my-config --from-file config.json       # Import from a specific JSON file

# Switch configurations
# Interactive mode to view and switch configurations
cc-switch  # Enter interactive mode (no arguments needed)

# List configurations
cc-switch list  # JSON output (default)
cc-switch list -p  # Plain text output

# Current configuration with interactive menu
cc-switch current  # Shows current + allows switching

# Manage configurations
cc-switch remove config1 config2 config3

# Shell integration
cc-switch completion fish  # Generate completion script
```

## Interactive Features

### Interactive Configuration Selection (`cc-switch`)

**Navigation**:

- **↑↓**: Navigate menu
- **1-9**: Quick-select configuration on current page
- **N/PageDown**: Next page (when >9 configs)
- **P/PageUp**: Previous page
- **R**: Reset to official Claude (removes custom settings)
- **E**: Exit
- **Enter**: Confirm selection

**Features**:

- Real-time configuration preview
- Smart pagination (9 configs per page)
- Graceful terminal fallback
- Auto-launches Claude after switch

### Keyboard Shortcuts

**Single Page** (≤9 configs): 1-9 keys select directly
**Multi Page** (>9 configs): 1-9 keys select on current page, use N/P to navigate

## Shell Integration

### Setup Completion

**Fish**:

```bash
cargo run -- completion fish > ~/.config/fish/completions/cc-switch.fish
```

**Zsh**:

```bash
cargo run -- completion zsh > ~/.zsh/completions/_cc-switch
echo 'fpath=(~/.zsh/completions $fpath)' >> ~/.zshrc
```

**Bash**:

```bash
cargo run -- completion bash > ~/.bash_completion.d/cc-switch
echo 'source ~/.bash_completion.d/cc-switch' >> ~/.bashrc
```

### Aliases

You can add your own shell aliases for quick access:

```bash
# Fish
echo "alias cs='cc-switch'" >> ~/.config/fish/config.fish
echo "alias ccd='claude --dangerously-skip-permissions'" >> ~/.config/fish/config.fish

# Zsh
echo "alias cs='cc-switch'" >> ~/.zshrc
echo "alias ccd='claude --dangerously-skip-permissions'" >> ~/.zshrc

# Bash
echo "alias cs='cc-switch'" >> ~/.bashrc
echo "alias ccd='claude --dangerously-skip-permissions'" >> ~/.bashrc
```

## Testing Strategy

### Test Organization

- **Library Tests** (`#[cfg(test)]` in `src/`): 20 tests
  - Unit tests for individual functions
  - Tests in same file as code

- **Integration Tests** (`tests/` directory): 154 tests
  - End-to-end workflow testing
  - CLI interaction testing
  - Error handling and edge cases
  - Cross-platform compatibility

### Running Tests

```bash
# All tests
cargo test

# Library only
cargo test --lib

# Integration only
cargo test --tests

# Specific integration test
cargo test --test integration_tests

# Single test
cargo test test_name

# With output
cargo test -- --nocapture
```

### Test Coverage

- **Configuration Management**: CRUD operations, validation, JSON serialization
- **Settings Management**: Environment variable handling, JSON persistence
- **CLI Parsing**: Command structure, argument validation, help generation
- **Error Handling**: Invalid inputs, file operations, edge cases
- **Interactive Features**: Keyboard navigation, pagination, boundary conditions
- **Shell Integration**: Completion generation, alias output
- **Cross-Platform**: Path resolution, file operations on different OSes

## Pre-commit Hooks

**Automatic Checks** (run on every commit):

- `cargo fmt --all -- --check` - Code formatting (matches CI; `--all` covers the whole workspace including `ccs-proxy`)
- `cargo clippy -- -D warnings` - Linting (warnings as errors)
- `cargo test` - All tests
- `cargo audit` - Security vulnerability scan
- `cargo doc --no-deps` - Documentation build
- `cargo build --release` - Release build (artifacts cleaned after)

**Setup**: `./scripts/setup-pre-commit.sh`

## CI/CD Pipeline

### CI Workflow (`.github/workflows/ci.yml`)

- Multi-platform: Ubuntu, Windows, macOS
- Cross-compilation: x86_64 and aarch64
- Runs: formatting check, clippy, tests, security audit
- Coverage reporting with codecov

### Release Workflow (`.github/workflows/release.yml`)

- Multi-architecture releases (Linux, Windows, macOS Intel/ARM)
- Automatic tar.gz packaging
- GitHub release creation with changelog
- brew repo addr <https://github.com/Linuxdazhao/homebrew-cc-switch/tree/main>

## File Storage Locations

- **Configuration Storage**: `~/.cc-switch/configurations.json`
- **Claude Settings**: `~/.claude/settings.json` (default) or custom via `--set-default-dir`
- **Path Resolution**: Supports absolute and home-relative paths

## Error Handling

- Uses `anyhow` for error context and propagation
- All operations include detailed error messages
- Graceful handling of missing files (creates defaults)
- Validates inputs before processing

## Cross-Platform Support

- Uses `dirs` crate for home/config directory resolution
- Handles path differences (Windows, Linux, macOS)
- CI builds for multiple target architectures
- Terminal handling via crossterm (cross-platform)

## Common Development Tasks

### Adding a New Command

1. Add variant to `Commands` enum in `src/cli/cli.rs`
2. Implement handler in `src/cli/main.rs`
3. Add match arm in `run()` function
4. Add completion support in `src/cli/completion.rs` if needed
5. Write integration tests in `tests/`
6. Update help text in `src/cli/cli.rs`

### Modifying Configuration Structure

1. Update `Configuration` struct in `src/config/types.rs`
2. Update serialization in `config_storage.rs` if needed
3. Update validation in `src/config/config.rs`
4. Update tests to reflect changes
5. Test backward compatibility

### Adding Shell Support

1. Add shell variant in `src/cli/completion.rs::generate_completion()`
2. Add completion logic for the shell
3. Add to `generate_aliases()` if needed
4. Update help text
5. Test on actual shell

## Important Implementation Notes

### Security

- API tokens are never logged
- Sensitive input handled carefully in interactive mode
- Configuration files should have appropriate permissions

### Backward Compatibility

- Existing configuration files remain compatible
- Uses interactive mode for configuration switching

### Performance

- Configuration loading is lazy (only when needed)
- Large configuration lists paginated for responsiveness
- Release build optimized with LTO and size optimization

### Cross-Platform Binary Resolution

On Windows, the `claude` and `codex` CLIs are typically installed via npm and
ship as `.cmd` shims rather than `.exe`. `std::process::Command::new("claude")`
does not always pick these up — its behavior depends on how PATHEXT is wired
in the calling shell, which is inconsistent across PowerShell / CMD / Git Bash.

To handle this uniformly, every `claude` or `codex` launch in the codebase
goes through `crate::platform::resolve_npm_cli(name)` (see `src/platform.rs`).
The resolver:

1. Honors a `<NAME_UPPERCASE>_BINARY` env-var override (`CLAUDE_BINARY`,
   `CODEX_BINARY`) — useful when users want to pin a specific installation
   or test against a sibling binary.
2. On Windows, probes `<name>.exe`, `<name>.cmd`, `<name>.ps1` via the `which`
   crate.
3. Falls back to the bare name on non-Windows or when probing finds nothing
   — identical to the pre-resolver behavior.

**When adding a new external CLI launch:** always go through the resolver.
Never write `Command::new("some_cli")` directly for a binary that might be
installed via npm.

Terminal Unicode detection lives in the same module
(`crate::platform::unicode_support_enabled`). On Windows it defaults to ASCII
unless `WT_SESSION` is set (Windows Terminal), which prevents mojibake on
legacy conhost / CMD. Users can override either way via `CC_SWITCH_ASCII=1`
or `CC_SWITCH_UNICODE=1`.

## Key Dependencies

- **anyhow**: Context-rich error handling
- **clap**: CLI parsing with derive macros
- **clap_complete**: Shell completion generation
- **serde/serde_json**: JSON serialization
- **dirs**: Cross-platform directory paths
- **colored**: Terminal output formatting
- **crossterm**: Terminal manipulation and events
- **tempfile**: Testing with temporary files

## Version and Compatibility

- **Rust Version**: 1.88.0 or later
- **Rust Edition**: 2024
- **Platforms**: Linux, macOS, Windows (CI tested)
- **Architectures**: x86_64, aarch64 (via CI)

## Before Pushing to GitHub

Verify locally:

```bash
cargo test              # All tests pass
cargo clippy -- -W warnings  # No warnings
cargo fmt --check       # Code formatted
cargo audit            # No security vulnerabilities
```

These are automatically checked by CI, but catching issues locally saves time.
