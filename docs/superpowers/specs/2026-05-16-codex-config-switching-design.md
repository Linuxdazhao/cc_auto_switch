# Codex Configuration Switching Feature Design

**Date**: 2026-05-16
**Status**: Design Complete

## Overview

Add support for managing multiple Codex (OpenAI CLI) configurations through `cc-switch`, mirroring the existing Claude configuration management functionality.

## User Requirements

- Support both OAuth (chatgpt) and API Key authentication modes
- Allow importing configurations from existing `auth.json` files or manual input
- Provide `use` command that switches configuration and launches Codex
- Maintain separate storage from Claude configurations in the same file

## CLI Interface

```bash
cc-switch codex add <alias> [OPTIONS]
cc-switch codex list [-p]
cc-switch codex use <alias> [--continue] [--resume ID] [PROMPT...]
cc-switch codex remove <alias>...
```

### Subcommand Details

#### `codex add`
Add a new Codex configuration:
- `--from-file <path>`: Import from existing auth.json file
- `-i, --interactive`: Enter values interactively
- `--force, -f`: Overwrite existing configuration
- Supports both OAuth and API Key modes

#### `codex list`
List all stored Codex configurations:
- Default: JSON output
- `-p, --plain`: Plain text output

#### `codex use`
Switch to a configuration and launch Codex:
- Writes configuration to `~/.codex/auth.json`
- Executes `codex` command with optional prompt
- Supports `--continue` and `--resume` flags

#### `codex remove`
Delete one or more configurations by alias name.

## Data Model

### Configuration Structure

```rust
struct CodexConfiguration {
    alias_name: String,
    auth_mode: String,           // "chatgpt" or "apikey"
    openai_api_key: Option<String>,
    // OAuth tokens (chatgpt mode)
    id_token: Option<String>,
    access_token: Option<String>,
    refresh_token: Option<String>,
    account_id: Option<String>,
    last_refresh: Option<String>,
}
```

### Storage

Extend `ConfigStorage` struct:

```rust
pub struct ConfigStorage {
    pub configurations: BTreeMap<String, Configuration>,      // Existing
    pub codex_configurations: BTreeMap<String, CodexConfiguration>,  // New
    pub claude_settings_dir: Option<String>,
    pub default_storage_mode: Option<StorageMode>,
}
```

Storage location: `~/.claude/cc_auto_switch_setting.json` (same file, new field).

## Implementation Architecture

### Module Organization

```
src/
├── codex/                    # New domain
│   ├── mod.rs
│   ├── types.rs              # CodexConfiguration struct
│   ├── storage.rs            # Codex-specific storage methods
│   └── commands.rs           # Command handlers
└── cli/
    └── cli.rs                # Add Codex subcommand enum
```

### Key Components

#### 1. CLI Layer (`src/cli/cli.rs`)

Add `Codex` variant to `Commands` enum with nested subcommands:

```rust
pub enum Commands {
    // Existing commands...
    Codex {
        #[command(subcommand)]
        command: CodexCommands,
    },
}

pub enum CodexCommands {
    Add { /* parameters */ },
    List { plain: bool },
    Use { alias_name: String, /* other flags */ },
    Remove { alias_names: Vec<String> },
}
```

#### 2. Data Layer (`src/codex/types.rs`)

Define `CodexConfiguration` struct with serde serialization for JSON storage.

#### 3. Storage Layer (`src/codex/storage.rs`)

Extend `ConfigStorage` with methods:
- `add_codex_configuration()`
- `get_codex_configuration()`
- `remove_codex_configuration()`
- `save()` (modified to include codex_configurations)

#### 4. Command Handlers (`src/codex/commands.rs`)

Implement handlers for each subcommand:
- `handle_codex_add()`: Parse file or interactive input, validate, save
- `handle_codex_list()`: Display configurations in JSON or plain text
- `handle_codex_use()`: Write auth.json, launch codex
- `handle_codex_remove()`: Delete configurations

#### 5. Auth File Writer

Create utility to write `~/.codex/auth.json`:

```rust
fn write_auth_json(config: &CodexConfiguration) -> Result<()> {
    // Generate JSON structure matching Codex format
    // Write to ~/.codex/auth.json
}
```

## File Operations

### Reading Existing auth.json

When using `--from-file`:
1. Read and parse JSON file
2. Detect auth mode from `auth_mode` field
3. Extract tokens or API key based on mode
4. Validate required fields present

### Writing auth.json

When switching configurations:
1. Load configuration from storage
2. Generate JSON structure based on auth mode
3. Write to `~/.codex/auth.json` (create directory if needed)
4. Set appropriate file permissions (0600)

## Command Execution

### Launching Codex

After writing auth.json:
1. Execute `codex` command
2. Pass through `--continue`, `--resume`, and prompt arguments
3. Inherit parent process stdin/stdout/stderr

## Error Handling

- Invalid auth.json format: Clear error message with expected structure
- Missing required fields: Specify which fields are missing
- File permission errors: Suggest checking ~/.codex directory permissions
- Codex binary not found: Inform user to install Codex CLI

## Testing Strategy

### Unit Tests
- Configuration serialization/deserialization
- auth.json generation with different auth modes
- Validation logic for both modes

### Integration Tests
- Add configuration from file
- Add configuration interactively
- List configurations
- Switch and verify auth.json content
- Remove configurations
- Error cases (invalid files, missing fields)

### Manual Testing
- Test with actual Codex CLI
- Verify OAuth mode works with real tokens
- Verify API Key mode works
- Test switching between multiple configurations

## Security Considerations

- Tokens are sensitive: Never log or display in plain text
- File permissions: Set auth.json to 0600 (owner read/write only)
- Input validation: Sanitize all user inputs
- Secure storage: Tokens stored in same file as Claude configs (existing security model)

## Migration & Compatibility

- **Backward Compatible**: Existing Claude configurations unaffected
- **No Migration Needed**: New feature, no existing data to migrate
- **Storage Evolution**: New `codex_configurations` field added to existing JSON

## Future Extensions

This design supports easy addition of other tools:
- `cc-switch gemini add/use/list/remove`
- `cc-switch cursor add/use/list/remove`
- Each tool gets its own nested subcommand structure

## Success Criteria

1. Users can add Codex configurations via file import or manual input
2. Users can list all stored Codex configurations
3. Users can switch configurations and launch Codex in one command
4. Users can remove configurations
5. Both OAuth and API Key modes work correctly
6. Configuration switching updates `~/.codex/auth.json` properly
7. All existing Claude functionality remains unchanged

## Implementation Plan

### Phase 1: Data Model & Storage
- Create `CodexConfiguration` struct
- Extend `ConfigStorage` with codex methods
- Add serialization tests

### Phase 2: CLI Structure
- Add `Codex` command to CLI parser
- Define nested subcommands
- Wire up command routing

### Phase 3: Command Handlers
- Implement `add` with file import and interactive mode
- Implement `list` with JSON and plain text output
- Implement `use` with auth.json writing and codex launch
- Implement `remove`

### Phase 4: Integration & Testing
- Write integration tests for all commands
- Manual testing with real Codex CLI
- Update documentation

## Dependencies

- Existing: `serde`, `serde_json`, `anyhow`, `dirs`
- No new dependencies required
- Reuse existing CLI parsing infrastructure
