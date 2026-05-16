# Codex Configuration Switching Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add support for managing multiple Codex (OpenAI CLI) configurations through `cc-switch`, mirroring the existing Claude configuration management functionality.

**Architecture:** Extend the existing `ConfigStorage` structure with a new `codex_configurations` field to store Codex-specific configuration data. Add a new `codex` top-level subcommand with nested subcommands (`add`, `list`, `use`, `remove`) that parallel the existing Claude commands. When switching configurations, write the appropriate `auth.json` file to `~/.codex/` and launch the `codex` CLI.

**Tech Stack:** Rust, clap (CLI parsing), serde/serde_json (serialization), anyhow (error handling), dirs (path resolution)

---

## File Structure Overview

**New Files:**
- `src/codex/mod.rs` - Module exports
- `src/codex/types.rs` - `CodexConfiguration` data structure
- `src/codex/storage.rs` - Storage extension methods
- `src/codex/commands.rs` - Command handlers
- `tests/codex_tests.rs` - Integration tests

**Modified Files:**
- `src/cli/cli.rs` - Add `Codex` command variant
- `src/config/types.rs` - Extend `ConfigStorage` struct
- `src/config/config_storage.rs` - Update save/load methods
- `src/lib.rs` - Export codex module
- `src/cli/main.rs` - Route codex commands

---

## Task 1: CodexConfiguration Data Structure

**Files:**
- Create: `src/codex/types.rs`
- Create: `src/codex/mod.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Create codex module directory and mod.rs**

```bash
mkdir -p src/codex
```

Create `src/codex/mod.rs`:

```rust
pub mod types;
pub mod storage;
pub mod commands;

pub use types::CodexConfiguration;
```

- [ ] **Step 2: Write CodexConfiguration struct test**

Create `src/codex/types.rs` with failing test:

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CodexConfiguration {
    pub alias_name: String,
    pub auth_mode: String,
    pub openai_api_key: Option<String>,
    pub id_token: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub account_id: Option<String>,
    pub last_refresh: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codex_configuration_serialization() {
        let config = CodexConfiguration {
            alias_name: "work".to_string(),
            auth_mode: "chatgpt".to_string(),
            openai_api_key: None,
            id_token: Some("test_id_token".to_string()),
            access_token: Some("test_access_token".to_string()),
            refresh_token: Some("test_refresh_token".to_string()),
            account_id: Some("test_account_id".to_string()),
            last_refresh: Some("2026-05-16T00:00:00Z".to_string()),
        };

        let json = serde_json::to_string(&config).expect("Should serialize");
        let deserialized: CodexConfiguration = serde_json::from_str(&json).expect("Should deserialize");
        
        assert_eq!(config.alias_name, deserialized.alias_name);
        assert_eq!(config.auth_mode, deserialized.auth_mode);
        assert_eq!(config.access_token, deserialized.access_token);
    }

    #[test]
    fn test_codex_configuration_apikey_mode() {
        let config = CodexConfiguration {
            alias_name: "apikey_config".to_string(),
            auth_mode: "apikey".to_string(),
            openai_api_key: Some("sk-ant-test-key".to_string()),
            id_token: None,
            access_token: None,
            refresh_token: None,
            account_id: None,
            last_refresh: None,
        };

        let json = serde_json::to_string(&config).expect("Should serialize");
        assert!(json.contains("apikey"));
        assert!(json.contains("sk-ant-test-key"));
    }
}
```

- [ ] **Step 3: Run test to verify it compiles and passes**

```bash
cargo test codex_configuration --lib
```

Expected: PASS

- [ ] **Step 4: Export codex module from lib.rs**

Modify `src/lib.rs`:

```rust
// Add at top with other modules
pub mod codex;

// Update public exports
pub use codex::CodexConfiguration;
```

- [ ] **Step 5: Commit**

```bash
git add src/codex/mod.rs src/codex/types.rs src/lib.rs
git commit -m "feat(codex): add CodexConfiguration data structure

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

## Task 2: Extend ConfigStorage for Codex Configurations

**Files:**
- Modify: `src/config/types.rs:442-451`
- Modify: `src/config/config_storage.rs`
- Create: `src/codex/storage.rs`

- [ ] **Step 1: Write failing test for codex storage methods**

Add test to `src/codex/storage.rs` (create this file):

```rust
#[cfg(test)]
mod tests {
    use crate::config::ConfigStorage;
    use crate::codex::CodexConfiguration;

    #[test]
    fn test_add_and_get_codex_configuration() {
        let mut storage = ConfigStorage::default();
        
        let config = CodexConfiguration {
            alias_name: "test".to_string(),
            auth_mode: "chatgpt".to_string(),
            openai_api_key: None,
            id_token: Some("id_token".to_string()),
            access_token: Some("access_token".to_string()),
            refresh_token: Some("refresh_token".to_string()),
            account_id: Some("account_id".to_string()),
            last_refresh: None,
        };

        storage.add_codex_configuration(config.clone());
        let retrieved = storage.get_codex_configuration("test");
        
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().alias_name, "test");
    }

    #[test]
    fn test_remove_codex_configuration() {
        let mut storage = ConfigStorage::default();
        
        let config = CodexConfiguration {
            alias_name: "test".to_string(),
            auth_mode: "chatgpt".to_string(),
            openai_api_key: None,
            id_token: None,
            access_token: None,
            refresh_token: None,
            account_id: None,
            last_refresh: None,
        };

        storage.add_codex_configuration(config);
        assert!(storage.get_codex_configuration("test").is_some());
        
        let removed = storage.remove_codex_configuration("test");
        assert!(removed);
        assert!(storage.get_codex_configuration("test").is_none());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test add_and_get_codex_configuration --lib
```

Expected: FAIL - `add_codex_configuration` method not found

- [ ] **Step 3: Extend ConfigStorage struct**

Modify `src/config/types.rs` around line 442:

```rust
#[derive(Serialize, Deserialize, Default)]
pub struct ConfigStorage {
    /// Map of alias names to configuration objects (Claude)
    pub configurations: ConfigMap,
    /// Map of alias names to Codex configuration objects
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codex_configurations: Option<std::collections::BTreeMap<String, crate::codex::CodexConfiguration>>,
    /// Custom directory for Claude settings (optional)
    pub claude_settings_dir: Option<String>,
    /// Default storage mode for writing configurations (None = use env mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_storage_mode: Option<StorageMode>,
}
```

- [ ] **Step 4: Implement storage methods in codex/storage.rs**

Create `src/codex/storage.rs`:

```rust
use crate::config::ConfigStorage;
use crate::codex::CodexConfiguration;
use std::collections::BTreeMap;

impl ConfigStorage {
    /// Add a new Codex configuration to storage
    pub fn add_codex_configuration(&mut self, config: CodexConfiguration) {
        if self.codex_configurations.is_none() {
            self.codex_configurations = Some(BTreeMap::new());
        }
        self.codex_configurations
            .as_mut()
            .unwrap()
            .insert(config.alias_name.clone(), config);
    }

    /// Get a Codex configuration by alias name
    pub fn get_codex_configuration(&self, alias_name: &str) -> Option<&CodexConfiguration> {
        self.codex_configurations
            .as_ref()
            .and_then(|configs| configs.get(alias_name))
    }

    /// Remove a Codex configuration by alias name
    pub fn remove_codex_configuration(&mut self, alias_name: &str) -> bool {
        if let Some(configs) = self.codex_configurations.as_mut() {
            configs.remove(alias_name).is_some()
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_get_codex_configuration() {
        let mut storage = ConfigStorage::default();
        
        let config = CodexConfiguration {
            alias_name: "test".to_string(),
            auth_mode: "chatgpt".to_string(),
            openai_api_key: None,
            id_token: Some("id_token".to_string()),
            access_token: Some("access_token".to_string()),
            refresh_token: Some("refresh_token".to_string()),
            account_id: Some("account_id".to_string()),
            last_refresh: None,
        };

        storage.add_codex_configuration(config.clone());
        let retrieved = storage.get_codex_configuration("test");
        
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().alias_name, "test");
    }

    #[test]
    fn test_remove_codex_configuration() {
        let mut storage = ConfigStorage::default();
        
        let config = CodexConfiguration {
            alias_name: "test".to_string(),
            auth_mode: "chatgpt".to_string(),
            openai_api_key: None,
            id_token: None,
            access_token: None,
            refresh_token: None,
            account_id: None,
            last_refresh: None,
        };

        storage.add_codex_configuration(config);
        assert!(storage.get_codex_configuration("test").is_some());
        
        let removed = storage.remove_codex_configuration("test");
        assert!(removed);
        assert!(storage.get_codex_configuration("test").is_none());
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cargo test codex_configuration --lib
```

Expected: PASS (both tests)

- [ ] **Step 6: Commit**

```bash
git add src/config/types.rs src/codex/storage.rs
git commit -m "feat(codex): extend ConfigStorage with codex methods

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

## Task 3: Add Codex CLI Command Structure

**Files:**
- Modify: `src/cli/cli.rs:69-254`

- [ ] **Step 1: Add CodexCommands enum**

Modify `src/cli/cli.rs` after line 67 (after `Commands` enum):

```rust
/// Available subcommands for Codex configuration management
#[derive(Subcommand)]
pub enum CodexCommands {
    /// Add a new Codex (OpenAI CLI) configuration
    Add {
        /// Configuration alias name
        #[arg(help = "Configuration alias name")]
        alias_name: String,

        /// OpenAI API key (for apikey auth mode)
        #[arg(long = "api-key", help = "OpenAI API key (optional)")]
        api_key: Option<String>,

        /// Force overwrite existing configuration
        #[arg(long = "force", short = 'f', help = "Overwrite existing configuration")]
        force: bool,

        /// Interactive mode for entering configuration values
        #[arg(long = "interactive", short = 'i', help = "Enter configuration values interactively")]
        interactive: bool,

        /// Import configuration from existing auth.json file
        #[arg(long = "from-file", help = "Import from existing auth.json file")]
        from_file: Option<String>,
    },
    /// List all stored Codex configurations
    List {
        /// Output in plain text format (default is JSON)
        #[arg(long = "plain", short = 'p')]
        plain: bool,
    },
    /// Switch to a Codex configuration and launch Codex
    #[command(trailing_var_arg = true)]
    Use {
        /// Configuration alias name to switch to
        alias_name: String,

        /// Continue the most recent Codex session
        #[arg(long = "continue", short = 'c')]
        r#continue: bool,

        /// Resume a Codex session by ID
        #[arg(long = "resume", short = 'r')]
        resume: Option<String>,

        /// Prompt to send to Codex (all remaining arguments)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        prompt: Vec<String>,
    },
    /// Remove one or more Codex configurations by alias name
    Remove {
        /// Configuration alias name(s) to remove
        #[arg(required = true)]
        alias_names: Vec<String>,
    },
}
```

- [ ] **Step 2: Add Codex variant to Commands enum**

Modify `src/cli/cli.rs` in the `Commands` enum (around line 254):

```rust
pub enum Commands {
    // ... existing variants ...
    
    /// Manage Codex (OpenAI CLI) configurations
    #[command(subcommand)]
    Codex(CodexCommands),
}
```

- [ ] **Step 3: Update long_about text**

Modify the `long_about` in `Cli` struct to include Codex examples:

```rust
long_about = "cc-switch helps you manage multiple Claude API configurations and switch between them easily.

EXAMPLES:
    cc-switch add my-config sk-ant-xxx https://api.anthropic.com
    cc-switch list
    cc-switch remove config1 config2 config3
    cc-switch current  # Interactive mode to view and switch configurations
    cc-switch  # Enter interactive mode (same as 'current' without arguments)

CODEX CONFIGURATIONS:
    cc-switch codex add work --from-file ~/.codex/auth.json
    cc-switch codex add personal -i  # Interactive mode
    cc-switch codex list
    cc-switch codex use work  # Switch and launch Codex
    cc-switch codex remove work

SHELL COMPLETION AND ALIASES:
    cc-switch completion fish  # Generates shell completions
    cc-switch alias fish       # Generates aliases for eval
    ..."
```

- [ ] **Step 4: Verify compilation**

```bash
cargo check
```

Expected: Compiles without errors

- [ ] **Step 5: Commit**

```bash
git add src/cli/cli.rs
git commit -m "feat(codex): add CLI command structure

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

## Task 4: Auth JSON Writer Utility

**Files:**
- Create: `src/codex/auth_writer.rs`
- Modify: `src/codex/mod.rs`

- [ ] **Step 1: Write failing test for auth.json writer**

Create `src/codex/auth_writer.rs`:

```rust
use crate::codex::CodexConfiguration;
use anyhow::{Result, anyhow};
use serde_json::json;
use std::fs;
use std::path::PathBuf;

/// Write CodexConfiguration to ~/.codex/auth.json
pub fn write_auth_json(config: &CodexConfiguration) -> Result<()> {
    let codex_dir = dirs::home_dir()
        .ok_or_else(|| anyhow!("Could not find home directory"))?
        .join(".codex");

    // Create directory if it doesn't exist
    if !codex_dir.exists() {
        fs::create_dir_all(&codex_dir)
            .map_err(|e| anyhow!("Failed to create .codex directory: {}", e))?;
    }

    let auth_path = codex_dir.join("auth.json");

    let json_value = if config.auth_mode == "apikey" {
        json!({
            "auth_mode": "apikey",
            "OPENAI_API_KEY": config.openai_api_key,
            "tokens": null
        })
    } else {
        json!({
            "auth_mode": "chatgpt",
            "OPENAI_API_KEY": config.openai_api_key,
            "tokens": {
                "id_token": config.id_token,
                "access_token": config.access_token,
                "refresh_token": config.refresh_token,
                "account_id": config.account_id
            },
            "last_refresh": config.last_refresh
        })
    };

    let json_string = serde_json::to_string_pretty(&json_value)
        .map_err(|e| anyhow!("Failed to serialize auth.json: {}", e))?;

    fs::write(&auth_path, json_string)
        .map_err(|e| anyhow!("Failed to write auth.json: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_write_chatgpt_mode() {
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let auth_path = temp_dir.path().join("auth.json");

        let config = CodexConfiguration {
            alias_name: "test".to_string(),
            auth_mode: "chatgpt".to_string(),
            openai_api_key: None,
            id_token: Some("test_id".to_string()),
            access_token: Some("test_access".to_string()),
            refresh_token: Some("test_refresh".to_string()),
            account_id: Some("test_account".to_string()),
            last_refresh: Some("2026-05-16T00:00:00Z".to_string()),
        };

        // Temporarily override home directory
        std::env::set_var("HOME", temp_dir.path());
        
        let result = write_auth_json(&config);
        assert!(result.is_ok());

        let content = fs::read_to_string(&auth_path).expect("Should read file");
        let parsed: serde_json::Value = serde_json::from_str(&content).expect("Should parse");
        
        assert_eq!(parsed["auth_mode"], "chatgpt");
        assert_eq!(parsed["tokens"]["id_token"], "test_id");
        assert_eq!(parsed["tokens"]["access_token"], "test_access");
    }

    #[test]
    fn test_write_apikey_mode() {
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let auth_path = temp_dir.path().join("auth.json");

        let config = CodexConfiguration {
            alias_name: "test".to_string(),
            auth_mode: "apikey".to_string(),
            openai_api_key: Some("sk-ant-test-key".to_string()),
            id_token: None,
            access_token: None,
            refresh_token: None,
            account_id: None,
            last_refresh: None,
        };

        std::env::set_var("HOME", temp_dir.path());
        
        let result = write_auth_json(&config);
        assert!(result.is_ok());

        let content = fs::read_to_string(&auth_path).expect("Should read file");
        let parsed: serde_json::Value = serde_json::from_str(&content).expect("Should parse");
        
        assert_eq!(parsed["auth_mode"], "apikey");
        assert_eq!(parsed["OPENAI_API_KEY"], "sk-ant-test-key");
        assert!(parsed["tokens"].is_null());
    }
}
```

- [ ] **Step 2: Run tests to verify they pass**

```bash
cargo test write_auth_json --lib
```

Expected: PASS (both tests)

- [ ] **Step 3: Export auth_writer module**

Modify `src/codex/mod.rs`:

```rust
pub mod types;
pub mod storage;
pub mod commands;
pub mod auth_writer;

pub use types::CodexConfiguration;
pub use auth_writer::write_auth_json;
```

- [ ] **Step 4: Commit**

```bash
git add src/codex/auth_writer.rs src/codex/mod.rs
git commit -m "feat(codex): add auth.json writer utility

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

## Task 5: Codex Command Handlers

**Files:**
- Create: `src/codex/commands.rs`
- Modify: `src/codex/mod.rs`

- [ ] **Step 1: Implement codex command handlers**

Create `src/codex/commands.rs`:

```rust
use crate::codex::{CodexConfiguration, write_auth_json};
use crate::config::ConfigStorage;
use anyhow::{Result, anyhow};
use std::fs;
use std::process::Command;

/// Handle adding a Codex configuration
pub fn handle_codex_add(
    alias_name: String,
    api_key: Option<String>,
    force: bool,
    interactive: bool,
    from_file: Option<String>,
    storage: &mut ConfigStorage,
) -> Result<()> {
    // Check if alias already exists
    if storage.get_codex_configuration(&alias_name).is_some() && !force {
        eprintln!("Codex configuration '{}' already exists.", alias_name);
        eprintln!("Use --force to overwrite or choose a different alias name.");
        return Ok(());
    }

    let config = if let Some(file_path) = from_file {
        // Import from file
        parse_auth_json_file(&file_path, &alias_name)?
    } else if interactive {
        // Interactive mode
        parse_interactive_codex_config(&alias_name)?
    } else {
        // API key mode (non-interactive)
        let key = api_key.ok_or_else(|| {
            anyhow!("API key is required. Use --api-key flag or --interactive mode")
        })?;
        
        CodexConfiguration {
            alias_name: alias_name.clone(),
            auth_mode: "apikey".to_string(),
            openai_api_key: Some(key),
            id_token: None,
            access_token: None,
            refresh_token: None,
            account_id: None,
            last_refresh: None,
        }
    };

    storage.add_codex_configuration(config);
    storage.save()?;

    println!("Codex configuration '{}' added successfully", alias_name);
    if force {
        println!("(Overwrote existing configuration)");
    }

    Ok(())
}

/// Parse an existing auth.json file
fn parse_auth_json_file(file_path: &str, alias_name: &str) -> Result<CodexConfiguration> {
    let content = fs::read_to_string(file_path)
        .map_err(|e| anyhow!("Failed to read file '{}': {}", file_path, e))?;

    let json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| anyhow!("Failed to parse JSON from file '{}': {}", file_path, e))?;

    let auth_mode = json["auth_mode"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing auth_mode in file '{}'", file_path))?
        .to_string();

    let openai_api_key = json["OPENAI_API_KEY"].as_str().map(|s| s.to_string());

    let (id_token, access_token, refresh_token, account_id) = if auth_mode == "chatgpt" {
        let tokens = json["tokens"]
            .as_object()
            .ok_or_else(|| anyhow!("Missing tokens object in file '{}'", file_path))?;

        (
            tokens["id_token"].as_str().map(|s| s.to_string()),
            tokens["access_token"].as_str().map(|s| s.to_string()),
            tokens["refresh_token"].as_str().map(|s| s.to_string()),
            tokens["account_id"].as_str().map(|s| s.to_string()),
        )
    } else {
        (None, None, None, None)
    };

    let last_refresh = json["last_refresh"].as_str().map(|s| s.to_string());

    Ok(CodexConfiguration {
        alias_name: alias_name.to_string(),
        auth_mode,
        openai_api_key,
        id_token,
        access_token,
        refresh_token,
        account_id,
        last_refresh,
    })
}

/// Parse Codex configuration interactively
fn parse_interactive_codex_config(alias_name: &str) -> Result<CodexConfiguration> {
    use crate::interactive::{read_input, read_sensitive_input};

    println!("Adding Codex configuration: {}", alias_name);
    
    let auth_mode = read_input("Auth mode (chatgpt/apikey) [chatgpt]: ")?;
    let auth_mode = if auth_mode.is_empty() { "chatgpt" } else { &auth_mode };

    match auth_mode {
        "chatgpt" => {
            let id_token = read_sensitive_input("Enter id_token: ")?;
            let access_token = read_sensitive_input("Enter access_token: ")?;
            let refresh_token = read_sensitive_input("Enter refresh_token: ")?;
            let account_id = read_input("Enter account_id: ")?;

            Ok(CodexConfiguration {
                alias_name: alias_name.to_string(),
                auth_mode: "chatgpt".to_string(),
                openai_api_key: None,
                id_token: Some(id_token),
                access_token: Some(access_token),
                refresh_token: Some(refresh_token),
                account_id: Some(account_id),
                last_refresh: None,
            })
        }
        "apikey" => {
            let api_key = read_sensitive_input("Enter OpenAI API key: ")?;

            Ok(CodexConfiguration {
                alias_name: alias_name.to_string(),
                auth_mode: "apikey".to_string(),
                openai_api_key: Some(api_key),
                id_token: None,
                access_token: None,
                refresh_token: None,
                account_id: None,
                last_refresh: None,
            })
        }
        _ => Err(anyhow!("Invalid auth mode: {}", auth_mode)),
    }
}

/// Handle listing Codex configurations
pub fn handle_codex_list(plain: bool, storage: &ConfigStorage) -> Result<()> {
    let configs = storage.codex_configurations.as_ref();

    if plain {
        match configs {
            None | Some(configs) if configs.is_empty() => {
                println!("No Codex configurations stored");
            }
            Some(configs) => {
                println!("Stored Codex configurations:");
                for (alias_name, config) in configs {
                    let mut info = format!("auth_mode={}", config.auth_mode);
                    if let Some(ref key) = config.openai_api_key {
                        info.push_str(&format!(", api_key={}...", &key[..8.min(key.len())]));
                    }
                    if let Some(ref account_id) = config.account_id {
                        info.push_str(&format!(", account_id={}...", &account_id[..8.min(account_id.len())]));
                    }
                    println!("  {}: {}", alias_name, info);
                }
            }
        }
    } else {
        match configs {
            None | Some(configs) if configs.is_empty() => {
                println!("{}");
            }
            Some(configs) => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(configs)
                        .map_err(|e| anyhow!("Failed to serialize configurations: {}", e))?
                );
            }
        }
    }

    Ok(())
}

/// Handle using a Codex configuration
pub fn handle_codex_use(
    alias_name: String,
    continue_flag: bool,
    resume: Option<String>,
    prompt: Vec<String>,
    storage: &mut ConfigStorage,
) -> Result<()> {
    let config = storage
        .get_codex_configuration(&alias_name)
        .ok_or_else(|| anyhow!("Codex configuration '{}' not found", alias_name))?
        .clone();

    // Write auth.json
    write_auth_json(&config)?;

    println!("Switched to Codex configuration '{}'", alias_name);
    println!("  Auth mode: {}", config.auth_mode);
    if let Some(ref account_id) = config.account_id {
        println!("  Account ID: {}...", &account_id[..8.min(account_id.len())]);
    }

    // Launch codex
    launch_codex(continue_flag, resume.as_deref(), &prompt)?;

    Ok(())
}

/// Launch Codex CLI with given arguments
fn launch_codex(continue_flag: bool, resume: Option<&str>, prompt: &[String]) -> Result<()> {
    let mut cmd = Command::new("codex");

    if continue_flag {
        cmd.arg("--continue");
    }

    if let Some(resume_id) = resume {
        cmd.arg("--resume").arg(resume_id);
    }

    if !prompt.is_empty() {
        cmd.args(prompt);
    }

    let status = cmd
        .status()
        .map_err(|e| anyhow!("Failed to launch Codex: {}", e))?;

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}

/// Handle removing Codex configurations
pub fn handle_codex_remove(alias_names: Vec<String>, storage: &mut ConfigStorage) -> Result<()> {
    let mut removed_count = 0;
    let mut not_found_aliases = Vec::new();

    for alias_name in &alias_names {
        if storage.remove_codex_configuration(alias_name) {
            removed_count += 1;
            println!("Codex configuration '{}' removed successfully", alias_name);
        } else {
            not_found_aliases.push(alias_name.clone());
            println!("Codex configuration '{}' not found", alias_name);
        }
    }

    if removed_count > 0 {
        storage.save()?;
    }

    if !not_found_aliases.is_empty() {
        eprintln!(
            "Warning: The following configurations were not found: {}",
            not_found_aliases.join(", ")
        );
    }

    if removed_count > 0 {
        println!("Successfully removed {} configuration(s)", removed_count);
    }

    Ok(())
}
```

- [ ] **Step 2: Export commands module**

Modify `src/codex/mod.rs`:

```rust
pub mod types;
pub mod storage;
pub mod commands;
pub mod auth_writer;

pub use types::CodexConfiguration;
pub use auth_writer::write_auth_json;
pub use commands::*;
```

- [ ] **Step 3: Verify compilation**

```bash
cargo check
```

Expected: Compiles without errors

- [ ] **Step 4: Commit**

```bash
git add src/codex/commands.rs src/codex/mod.rs
git commit -m "feat(codex): implement command handlers

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

## Task 6: Wire Up Codex Commands in Main

**Files:**
- Modify: `src/cli/main.rs`

- [ ] **Step 1: Import codex handlers**

Add to top of `src/cli/main.rs`:

```rust
use crate::codex::{
    handle_codex_add, handle_codex_list, handle_codex_remove, handle_codex_use,
};
```

- [ ] **Step 2: Add Codex command routing**

In the `run()` function, add a new match arm for `Commands::Codex`. Find the match statement around line 595 and add before the closing brace:

```rust
Commands::Codex(codex_cmd) => {
    match codex_cmd {
        crate::cli::CodexCommands::Add {
            alias_name,
            api_key,
            force,
            interactive,
            from_file,
        } => {
            handle_codex_add(alias_name, api_key, force, interactive, from_file, &mut storage)?;
        }
        crate::cli::CodexCommands::List { plain } => {
            handle_codex_list(plain, &storage)?;
        }
        crate::cli::CodexCommands::Use {
            alias_name,
            r#continue,
            resume,
            prompt,
        } => {
            handle_codex_use(alias_name, r#continue, resume, prompt, &mut storage)?;
        }
        crate::cli::CodexCommands::Remove { alias_names } => {
            handle_codex_remove(alias_names, &mut storage)?;
        }
    }
}
```

- [ ] **Step 3: Verify compilation**

```bash
cargo build
```

Expected: Builds successfully

- [ ] **Step 4: Test CLI help output**

```bash
cargo run -- codex --help
```

Expected: Shows Codex subcommand help

- [ ] **Step 5: Commit**

```bash
git add src/cli/main.rs
git commit -m "feat(codex): wire up codex commands in main

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

## Task 7: Integration Tests

**Files:**
- Create: `tests/codex_tests.rs`

- [ ] **Step 1: Write integration tests**

Create `tests/codex_tests.rs`:

```rust
use cc_switch::codex::CodexConfiguration;
use cc_switch::config::ConfigStorage;
use tempfile::TempDir;
use std::fs;

#[test]
fn test_add_codex_config_from_file() {
    let temp_dir = TempDir::new().expect("Should create temp dir");
    let config_path = temp_dir.path().join("test_auth.json");
    
    let auth_json = r#"{
        "auth_mode": "chatgpt",
        "OPENAI_API_KEY": null,
        "tokens": {
            "id_token": "test_id",
            "access_token": "test_access",
            "refresh_token": "test_refresh",
            "account_id": "test_account"
        },
        "last_refresh": "2026-05-16T00:00:00Z"
    }"#;
    
    fs::write(&config_path, auth_json).expect("Should write file");

    let mut storage = ConfigStorage::default();
    let config = crate::codex::commands::parse_auth_json_file(
        config_path.to_str().unwrap(),
        "test_alias"
    ).expect("Should parse file");

    storage.add_codex_configuration(config);
    
    let retrieved = storage.get_codex_configuration("test_alias");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().auth_mode, "chatgpt");
}

#[test]
fn test_list_codex_configurations() {
    let mut storage = ConfigStorage::default();
    
    let config1 = CodexConfiguration {
        alias_name: "work".to_string(),
        auth_mode: "chatgpt".to_string(),
        openai_api_key: None,
        id_token: Some("id1".to_string()),
        access_token: Some("access1".to_string()),
        refresh_token: Some("refresh1".to_string()),
        account_id: Some("account1".to_string()),
        last_refresh: None,
    };

    let config2 = CodexConfiguration {
        alias_name: "personal".to_string(),
        auth_mode: "apikey".to_string(),
        openai_api_key: Some("sk-test-key".to_string()),
        id_token: None,
        access_token: None,
        refresh_token: None,
        account_id: None,
        last_refresh: None,
    };

    storage.add_codex_configuration(config1);
    storage.add_codex_configuration(config2);

    assert_eq!(storage.codex_configurations.as_ref().unwrap().len(), 2);
    assert!(storage.get_codex_configuration("work").is_some());
    assert!(storage.get_codex_configuration("personal").is_some());
}

#[test]
fn test_remove_codex_configurations() {
    let mut storage = ConfigStorage::default();
    
    let config = CodexConfiguration {
        alias_name: "test".to_string(),
        auth_mode: "chatgpt".to_string(),
        openai_api_key: None,
        id_token: Some("id".to_string()),
        access_token: Some("access".to_string()),
        refresh_token: Some("refresh".to_string()),
        account_id: Some("account".to_string()),
        last_refresh: None,
    };

    storage.add_codex_configuration(config);
    assert!(storage.get_codex_configuration("test").is_some());

    let removed = storage.remove_codex_configuration("test");
    assert!(removed);
    assert!(storage.get_codex_configuration("test").is_none());

    let removed_again = storage.remove_codex_configuration("test");
    assert!(!removed_again);
}
```

- [ ] **Step 2: Run integration tests**

```bash
cargo test --test codex_tests
```

Expected: All tests pass

- [ ] **Step 3: Commit**

```bash
git add tests/codex_tests.rs
git commit -m "test(codex): add integration tests

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

## Task 8: End-to-End Manual Testing

- [ ] **Step 1: Test add from file**

```bash
# Create a test auth.json file
echo '{"auth_mode":"chatgpt","OPENAI_API_KEY":null,"tokens":{"id_token":"test","access_token":"test","refresh_token":"test","account_id":"test"},"last_refresh":"2026-05-16T00:00:00Z"}' > /tmp/test_auth.json

# Add configuration
cargo run -- codex add test --from-file /tmp/test_auth.json

# Verify
cargo run -- codex list
```

Expected: Shows the test configuration in JSON format

- [ ] **Step 2: Test add with API key**

```bash
cargo run -- codex add apikey_test --api-key sk-test-key-12345

# Verify
cargo run -- codex list -p
```

Expected: Shows both configurations in plain text

- [ ] **Step 3: Test use command (without launching codex)**

```bash
# First, backup your real ~/.codex/auth.json if it exists
cp ~/.codex/auth.json ~/.codex/auth.json.backup 2>/dev/null || true

# Switch to test configuration
cargo run -- codex use test

# Verify auth.json was written
cat ~/.codex/auth.json

# Restore backup
mv ~/.codex/auth.json.backup ~/.codex/auth.json 2>/dev/null || true
```

Expected: auth.json contains the test configuration

- [ ] **Step 4: Test remove command**

```bash
cargo run -- codex remove test apikey_test

# Verify removal
cargo run -- codex list
```

Expected: Shows "No Codex configurations stored"

- [ ] **Step 5: Test force overwrite**

```bash
# Add a configuration
cargo run -- codex add test --api-key sk-key-1

# Try to add again without force (should fail gracefully)
cargo run -- codex add test --api-key sk-key-2

# Add with force (should succeed)
cargo run -- codex add test --api-key sk-key-2 --force

# Verify
cargo run -- codex list

# Cleanup
cargo run -- codex remove test
```

Expected: Second add shows warning, third add with --force succeeds

- [ ] **Step 6: Verify all existing tests still pass**

```bash
cargo test
```

Expected: All tests pass (no regressions)

- [ ] **Step 7: Run clippy and fmt**

```bash
cargo clippy -- -W warnings
cargo fmt --check
```

Expected: No warnings, code formatted

- [ ] **Step 8: Commit**

```bash
git add .
git commit -m "feat(codex): complete Codex configuration switching feature

Implemented:
- CodexConfiguration data structure with OAuth and API key modes
- Storage methods for managing Codex configurations
- CLI commands: add, list, use, remove
- Auth.json writer for ~/.codex/auth.json
- Integration tests

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

## Summary

This implementation plan adds complete Codex configuration management to cc-switch:

1. **Data Model** - `CodexConfiguration` struct supporting both OAuth and API key modes
2. **Storage** - Extended `ConfigStorage` with codex-specific methods
3. **CLI** - New `codex` top-level command with `add`, `list`, `use`, `remove` subcommands
4. **Auth Writer** - Utility to write `~/.codex/auth.json` in the correct format
5. **Command Handlers** - Full implementation of all subcommands
6. **Tests** - Integration tests covering all functionality

The feature is backward compatible (existing Claude configurations unaffected) and follows the same patterns as the existing Claude configuration management.
