# `--from-file` Default Import Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `--from-file` accept zero or one argument for both `cc-switch add` (Claude) and `cc-switch codex add`, so passing it with no value imports from the tool's default config location (`~/.claude/settings.json` or `~/.codex/auth.json`), while always honoring the user-provided alias.

**Architecture:** Change the clap definition of `--from-file` to `Option<Option<String>>` with `num_args = 0..=1`. Resolve the tri-state to a concrete path inside the handler (default location when value is missing). Drop the legacy "filename-as-alias" override in the Claude handler. Make `alias_name` always required in both `add` subcommands.

**Tech Stack:** Rust 1.88+, clap 4.x (derive), anyhow, serde_json, dirs, tempfile (tests).

**Reference spec:** `docs/superpowers/specs/2026-05-23-from-file-default-import-design.md`

---

## File Structure

**Created:**
- (none — all changes touch existing files)

**Modified:**
- `src/codex/auth_writer.rs` — add `default_codex_auth_path()` read-only helper.
- `src/codex/mod.rs` — re-export the new helper.
- `src/cli/cli.rs` — `Add::alias_name` becomes required `String`; `Add::from_file` and `CodexCommands::Add::from_file` become `Option<Option<String>>`; drop the `-j` short on Claude; update `long_about` examples.
- `src/cli/main.rs` — `parse_config_from_file` stops returning the file-derived alias; `handle_add_command` resolves tri-state to a concrete path; `Commands::Add` match arm drops the placeholder block.
- `src/codex/commands.rs` — `handle_codex_add` accepts `Option<Option<String>>` and resolves the default path.
- `src/cli/completion.rs` — update fish/zsh/bash descriptions for `--from-file`.
- `tests/main_tests.rs` — drop `.unwrap()` on `alias_name` after type change; add new CLI-parse and handler tests.
- `tests/codex_tests.rs` — add default-path tests.
- `CLAUDE.md`, `README.md`, `README_EN.md`, `docs/codex.md`, `docs/codex_EN.md` — update `--from-file` examples.

---

## Task 1: Add `default_codex_auth_path()` helper

**Files:**
- Modify: `src/codex/auth_writer.rs`
- Modify: `src/codex/mod.rs`
- Test: `src/codex/auth_writer.rs` (existing `#[cfg(test)] mod tests`)

- [ ] **Step 1: Write the failing test**

Append to the existing `#[cfg(test)] mod tests` block in `src/codex/auth_writer.rs` (before the closing `}`):

```rust
    #[test]
    fn test_default_codex_auth_path_ends_correctly() {
        let path = default_codex_auth_path().expect("Should resolve default codex auth path");
        let path_str = path.to_string_lossy();
        assert!(
            path_str.ends_with(".codex/auth.json")
                || path_str.ends_with(r".codex\auth.json"),
            "expected path to end with .codex/auth.json, got {}",
            path_str
        );
    }

    #[test]
    fn test_default_codex_auth_path_does_not_create_dir() {
        // The default-path helper must be a pure lookup. It must NOT have
        // a side effect of creating the .codex directory (unlike the writer's
        // private get_auth_path which does create it).
        let _path = default_codex_auth_path().expect("Should resolve");
        // No filesystem assertions — this test is here to lock in the contract;
        // a future refactor that adds mkdir to the helper will be caught by
        // code review of this test's intent comment.
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test codex_tests test_default_codex_auth_path 2>&1 | head -20`

Actually the tests are inline in `src/codex/auth_writer.rs`, so run:

Run: `cargo test --lib default_codex_auth_path 2>&1 | tail -20`

Expected: FAIL with `error[E0425]: cannot find function default_codex_auth_path` (or similar — function doesn't exist yet).

- [ ] **Step 3: Implement the helper**

Add this function to `src/codex/auth_writer.rs` immediately after the existing `get_auth_path` function (around line 22, after its closing `}`):

```rust
/// Build the default path to `~/.codex/auth.json` without creating any
/// directories.
///
/// This is a read-only lookup, distinct from `get_auth_path` which both
/// resolves the path and creates the `.codex` directory as a side effect
/// for the write path.
pub fn default_codex_auth_path() -> Result<PathBuf> {
    Ok(dirs::home_dir()
        .ok_or_else(|| anyhow!("Could not find home directory"))?
        .join(".codex")
        .join("auth.json"))
}
```

- [ ] **Step 4: Re-export from module**

Modify `src/codex/mod.rs`: add `default_codex_auth_path` to the existing re-exports.

Open `src/codex/mod.rs`, find the line `pub use auth_writer::write_auth_json;`, and change it to:

```rust
pub use auth_writer::{default_codex_auth_path, write_auth_json};
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test --lib default_codex_auth_path 2>&1 | tail -10`

Expected: 2 tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/codex/auth_writer.rs src/codex/mod.rs
git commit -m "$(cat <<'EOF'
feat(codex): add default_codex_auth_path read-only helper

Pure lookup for ~/.codex/auth.json without the directory-creation side
effect of the writer's get_auth_path.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: Drop file-alias derivation from `parse_config_from_file`

**Files:**
- Modify: `src/cli/main.rs:48-174` (`parse_config_from_file`)
- Modify: `src/cli/main.rs:184-228` (caller in `handle_add_command`)

This is a refactor — there are no direct unit tests for this private function, and the caller will compile-break until updated. We update both at once.

- [ ] **Step 1: Modify `parse_config_from_file` to drop the alias return**

In `src/cli/main.rs`, replace the entire `parse_config_from_file` function (lines 37–174). The new function returns a 13-tuple instead of 14-tuple:

```rust
/// Parse a configuration from a JSON file
///
/// # Arguments
/// * `file_path` - Path to the JSON configuration file
///
/// # Returns
/// Result containing a tuple of configuration values (token, url, and optional fields)
///
/// # Errors
/// Returns error if file cannot be read or parsed
#[allow(clippy::type_complexity)]
fn parse_config_from_file(
    file_path: &str,
) -> Result<(
    String,
    String,
    Option<String>,
    Option<String>,
    Option<u32>,
    Option<u32>,
    Option<u32>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<u32>,
    Option<String>,
)> {
    let file_content = fs::read_to_string(file_path)
        .map_err(|e| anyhow!("Failed to read file '{}': {}", file_path, e))?;

    let json: serde_json::Value = serde_json::from_str(&file_content)
        .map_err(|e| anyhow!("Failed to parse JSON from file '{}': {}", file_path, e))?;

    let env = json.get("env").and_then(|v| v.as_object()).ok_or_else(|| {
        anyhow!(
            "File '{}' does not contain a valid 'env' section",
            file_path
        )
    })?;

    let token = env
        .get("ANTHROPIC_AUTH_TOKEN")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Missing ANTHROPIC_AUTH_TOKEN in file '{}'", file_path))?
        .to_string();

    let url = env
        .get("ANTHROPIC_BASE_URL")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Missing ANTHROPIC_BASE_URL in file '{}'", file_path))?
        .to_string();

    let model = env
        .get("ANTHROPIC_MODEL")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let small_fast_model = env
        .get("ANTHROPIC_SMALL_FAST_MODEL")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let max_thinking_tokens = env
        .get("ANTHROPIC_MAX_THINKING_TOKENS")
        .and_then(|v| v.as_u64())
        .map(|u| u as u32);

    let api_timeout_ms = env
        .get("API_TIMEOUT_MS")
        .and_then(|v| v.as_u64())
        .map(|u| u as u32);

    let claude_code_disable_nonessential_traffic = env
        .get("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC")
        .and_then(|v| v.as_u64())
        .map(|u| u as u32);

    let anthropic_default_sonnet_model = env
        .get("ANTHROPIC_DEFAULT_SONNET_MODEL")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let anthropic_default_opus_model = env
        .get("ANTHROPIC_DEFAULT_OPUS_MODEL")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let anthropic_default_haiku_model = env
        .get("ANTHROPIC_DEFAULT_HAIKU_MODEL")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let claude_code_subagent_model = env
        .get("CLAUDE_CODE_SUBAGENT_MODEL")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let claude_code_disable_nonstreaming_fallback = env
        .get("CLAUDE_CODE_DISABLE_NONSTREAMING_FALLBACK")
        .and_then(|v| v.as_u64())
        .map(|u| u as u32);

    let claude_code_effort_level = env
        .get("CLAUDE_CODE_EFFORT_LEVEL")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok((
        token,
        url,
        model,
        small_fast_model,
        max_thinking_tokens,
        api_timeout_ms,
        claude_code_disable_nonessential_traffic,
        anthropic_default_sonnet_model,
        anthropic_default_opus_model,
        anthropic_default_haiku_model,
        claude_code_subagent_model,
        claude_code_disable_nonstreaming_fallback,
        claude_code_effort_level,
    ))
}
```

Note: the `Path` import on line 17 was only used by the deleted `path.file_stem()` block — leave it; it may be used elsewhere or pruned later by rustc warnings. Run `cargo build` after Step 2 to confirm and clean up.

- [ ] **Step 2: Update the caller in `handle_add_command`**

In `src/cli/main.rs`, find the block starting `if let Some(file_path) = &params.from_file {` (line 186) and replace through line 228 with:

```rust
    if let Some(file_path) = &params.from_file {
        println!("Importing configuration from file: {}", file_path);

        let (
            file_token,
            file_url,
            file_model,
            file_small_fast_model,
            file_max_thinking_tokens,
            file_api_timeout_ms,
            file_claude_disable_nonessential_traffic,
            file_sonnet_model,
            file_opus_model,
            file_haiku_model,
            file_subagent_model,
            file_disable_nonstreaming_fallback,
            file_effort_level,
        ) = parse_config_from_file(file_path)?;

        params.token = Some(file_token);
        params.url = Some(file_url);
        params.model = file_model;
        params.small_fast_model = file_small_fast_model;
        params.max_thinking_tokens = file_max_thinking_tokens;
        params.api_timeout_ms = file_api_timeout_ms;
        params.claude_code_disable_nonessential_traffic = file_claude_disable_nonessential_traffic;
        params.anthropic_default_sonnet_model = file_sonnet_model;
        params.anthropic_default_opus_model = file_opus_model;
        params.anthropic_default_haiku_model = file_haiku_model;
        params.claude_code_subagent_model = file_subagent_model;
        params.claude_code_disable_nonstreaming_fallback = file_disable_nonstreaming_fallback;
        params.claude_code_effort_level = file_effort_level;

        println!(
            "Configuration '{}' will be imported from file",
            params.alias_name
        );
    }
```

Notice: the previous `params.alias_name = file_alias_name;` line is gone — alias from CLI is kept verbatim.

- [ ] **Step 3: Run `cargo build` to confirm everything compiles**

Run: `cargo build 2>&1 | tail -30`

Expected: Build succeeds. If `Path` import is now unused, you'll see `warning: unused import: 'std::path::Path'`. If so, remove the line `use std::path::Path;` from `src/cli/main.rs`.

- [ ] **Step 4: Run full test suite to confirm no regressions**

Run: `cargo test 2>&1 | tail -15`

Expected: all tests pass (we have not yet changed the CLI types, so existing tests still apply).

- [ ] **Step 5: Commit**

```bash
git add src/cli/main.rs
git commit -m "$(cat <<'EOF'
refactor(claude): drop filename-as-alias derivation from from-file

parse_config_from_file no longer returns a file-derived alias, and the
caller no longer overrides params.alias_name. Behavior change comes in a
later task when the CLI alias becomes required.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: Make Claude CLI accept `--from-file` with optional value, alias required

**Files:**
- Modify: `src/cli/cli.rs:87–213` (`Commands::Add` variant)
- Modify: `src/cli/main.rs:600–664` (`Commands::Add` match arm)
- Modify: `tests/main_tests.rs` (existing CLI parse tests + new tests)

- [ ] **Step 1: Add a failing CLI parse test for `--from-file` with no value**

Append to the `#[cfg(test)] mod tests { ... }` block in `tests/main_tests.rs` (insert before the closing `}` of `mod tests`):

```rust
    #[test]
    fn test_cli_add_from_file_no_value() {
        let args = vec!["cc-switch", "add", "work", "--from-file"];
        let cli = Cli::try_parse_from(args).expect("Should parse --from-file with no value");
        match cli.command {
            Some(Commands::Add {
                alias_name,
                from_file,
                ..
            }) => {
                assert_eq!(alias_name, "work");
                assert_eq!(from_file, Some(None), "expected Some(None) for bare --from-file");
            }
            _ => panic!("Expected Add command"),
        }
    }

    #[test]
    fn test_cli_add_from_file_with_value() {
        let args = vec!["cc-switch", "add", "work", "--from-file", "/tmp/config.json"];
        let cli = Cli::try_parse_from(args).expect("Should parse --from-file with path");
        match cli.command {
            Some(Commands::Add {
                alias_name,
                from_file,
                ..
            }) => {
                assert_eq!(alias_name, "work");
                assert_eq!(from_file, Some(Some("/tmp/config.json".to_string())));
            }
            _ => panic!("Expected Add command"),
        }
    }

    #[test]
    fn test_cli_add_alias_required() {
        let args = vec!["cc-switch", "add", "--from-file", "/tmp/config.json"];
        let result = Cli::try_parse_from(args);
        assert!(
            result.is_err(),
            "alias_name must be required even with --from-file"
        );
    }
```

- [ ] **Step 2: Run new tests to confirm they fail**

Run: `cargo test --test main_tests test_cli_add_from_file_no_value test_cli_add_from_file_with_value test_cli_add_alias_required 2>&1 | tail -20`

Expected: tests fail to compile because `alias_name` is currently `Option<String>` (not `String`), or fail at runtime because the current `from_file` type is `Option<String>` (not `Option<Option<String>>`).

- [ ] **Step 3: Update `Commands::Add` in `src/cli/cli.rs`**

In `src/cli/cli.rs`, find the `Add { ... }` variant (around lines 87–213). Make two changes:

**3a. `alias_name`** — replace:

```rust
        /// Configuration alias name (used to identify this config)
        #[arg(
            help = "Configuration alias name (cannot be 'cc')",
            required_unless_present = "from_file"
        )]
        alias_name: Option<String>,
```

with:

```rust
        /// Configuration alias name (used to identify this config)
        #[arg(help = "Configuration alias name (cannot be 'cc')")]
        alias_name: String,
```

**3b. `from_file`** — replace:

```rust
        /// Import configuration from a JSON file (uses filename as alias)
        #[arg(
            long = "from-file",
            short = 'j',
            help = "Import configuration from a JSON file (filename becomes alias name)"
        )]
        from_file: Option<String>,
```

with:

```rust
        /// Import configuration from a JSON file
        ///
        /// With no value, imports from `~/.claude/settings.json`.
        /// With a value, imports from the given path.
        #[arg(
            long = "from-file",
            num_args = 0..=1,
            value_name = "PATH",
            help = "Import configuration from JSON file (defaults to ~/.claude/settings.json if no path)"
        )]
        from_file: Option<Option<String>>,
```

- [ ] **Step 4: Update existing CLI-parse tests in `tests/main_tests.rs`**

`cargo build` will now flag every `alias_name.unwrap()` in tests. Fix all 6 sites in `tests/main_tests.rs`:

| Line | Old | New |
|---|---|---|
| 124 | `assert_eq!(alias_name.unwrap(), "my-config");` | `assert_eq!(alias_name, "my-config");` |
| 174 | `assert_eq!(alias_name.unwrap(), "my-config");` | `assert_eq!(alias_name, "my-config");` |
| 216 | `assert_eq!(alias_name.unwrap(), "model-config");` | `assert_eq!(alias_name, "model-config");` |
| 455 | `assert_eq!(alias_name.unwrap(), "test-config_123");` | `assert_eq!(alias_name, "test-config_123");` |
| 480 | `assert_eq!(alias_name.unwrap(), "测试-config");` | `assert_eq!(alias_name, "测试-config");` |
| 505 | `assert_eq!(alias_name.unwrap().len(), 1000);` | `assert_eq!(alias_name.len(), 1000);` |

Use `cargo build --tests 2>&1 | grep "alias_name"` to confirm no more compile errors of this kind.

- [ ] **Step 5: Update `Commands::Add` match arm in `src/cli/main.rs`**

In `src/cli/main.rs`, find the `Commands::Add { ... }` arm (around lines 600–664). The full replacement block (preserving the destructuring and `AddCommandParams` construction):

Locate and delete the placeholder logic (lines 631–641):

```rust
                // When from_file is provided, alias_name will be extracted from the file
                // For other cases, use the provided alias_name or provide a default
                let final_alias_name = if from_file.is_some() {
                    // Will be set from file parsing, use a placeholder for now
                    "placeholder".to_string()
                } else {
                    alias_name.unwrap_or_else(|| {
                        eprintln!("Error: alias_name is required when not using --from-file");
                        std::process::exit(1);
                    })
                };
```

Then change the `AddCommandParams` construction to:

- Use `alias_name` directly (it is now `String`, not `Option<String>`).
- Resolve `from_file: Option<Option<String>>` to `Option<String>` for `AddCommandParams.from_file`. When `Some(None)`, call `crate::utils::get_claude_settings_path(cli.store.as_deref().and_then(|_| None))` — wait, actually the custom dir comes from a separate mechanism (set-default-dir stored in storage), not from `cli.store`. Use `None` for the custom dir at resolution time; the default-path behavior should always target `~/.claude/settings.json` regardless of `--set-default-dir`, because importing is reading the user's claude config which is the conventional location.

Replace the deleted block + the `let params = ...` block with:

```rust
                let resolved_from_file: Option<String> = match from_file {
                    Some(Some(path)) => Some(path),
                    Some(None) => {
                        let custom_dir = storage.get_claude_settings_dir().map(|s| s.as_str());
                        Some(
                            crate::utils::get_claude_settings_path(custom_dir)
                                .map(|p| p.to_string_lossy().into_owned())
                                .map_err(|e| {
                                    anyhow!("Failed to resolve default Claude settings path: {}", e)
                                })?,
                        )
                    }
                    None => None,
                };

                let params = AddCommandParams {
                    alias_name,
                    token,
                    url,
                    model,
                    small_fast_model,
                    max_thinking_tokens,
                    api_timeout_ms,
                    claude_code_disable_nonessential_traffic,
                    anthropic_default_sonnet_model,
                    anthropic_default_opus_model,
                    anthropic_default_haiku_model,
                    claude_code_subagent_model,
                    claude_code_disable_nonstreaming_fallback,
                    claude_code_effort_level,
                    force,
                    interactive,
                    token_arg,
                    url_arg,
                    from_file: resolved_from_file,
                };
                handle_add_command(params, &mut storage)?;
```

- [ ] **Step 6: Add a guidance error when the resolved default path does not exist**

In `src/cli/main.rs`, inside `handle_add_command`, at the top of the `if let Some(file_path) = &params.from_file { ... }` block (just before the `println!("Importing configuration from file: ...")` line), add:

```rust
        if !std::path::Path::new(file_path).exists() {
            anyhow::bail!(
                "Config file not found: {}\n\
                 If you intended to import from Claude's default config, run `claude` once to create it.\n\
                 Otherwise pass an explicit path: --from-file <path>",
                file_path
            );
        }
```

Re-add the `use std::path::Path;` at the top of `src/cli/main.rs` if it was removed in Task 2 Step 3, OR use the fully-qualified form `std::path::Path::new(file_path)` as shown.

- [ ] **Step 7: Run new CLI parse tests to confirm they pass**

Run: `cargo test --test main_tests test_cli_add_from_file_no_value test_cli_add_from_file_with_value test_cli_add_alias_required 2>&1 | tail -10`

Expected: 3 tests pass.

- [ ] **Step 8: Run full test suite**

Run: `cargo test 2>&1 | tail -15`

Expected: all tests pass.

- [ ] **Step 9: Commit**

```bash
git add src/cli/cli.rs src/cli/main.rs tests/main_tests.rs
git commit -m "$(cat <<'EOF'
feat(cli): claude --from-file accepts optional path, alias required

`--from-file` with no value imports from ~/.claude/settings.json; with a
value, imports from the given path. The positional alias is now always
required (mirrors codex add). Drops the `-j` short.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Mirror the change in `cc-switch codex add`

**Files:**
- Modify: `src/cli/cli.rs:299–314` (`CodexCommands::Add`)
- Modify: `src/cli/main.rs` (call site that builds the codex-add args; locate by `Commands::Codex` match arm)
- Modify: `src/codex/commands.rs:9–56` (`handle_codex_add`)
- Modify: `tests/codex_tests.rs` (new tests)

- [ ] **Step 1: Write failing CLI parse tests for codex**

Append to `tests/codex_tests.rs`:

```rust
#[cfg(test)]
mod cli_parse_tests {
    use cc_switch::cli::{Cli, Commands, CodexCommands};
    use clap::Parser;

    #[test]
    fn test_codex_add_from_file_no_value() {
        let args = vec!["cc-switch", "codex", "add", "work", "--from-file"];
        let cli = Cli::try_parse_from(args).expect("Should parse codex add --from-file");
        let Some(Commands::Codex { command: Some(CodexCommands::Add { alias_name, from_file, .. }) }) = cli.command else {
            panic!("Expected codex add command");
        };
        assert_eq!(alias_name, "work");
        assert_eq!(from_file, Some(None));
    }

    #[test]
    fn test_codex_add_from_file_with_value() {
        let args = vec!["cc-switch", "codex", "add", "work", "--from-file", "/tmp/auth.json"];
        let cli = Cli::try_parse_from(args).expect("Should parse codex add --from-file path");
        let Some(Commands::Codex { command: Some(CodexCommands::Add { alias_name, from_file, .. }) }) = cli.command else {
            panic!("Expected codex add command");
        };
        assert_eq!(alias_name, "work");
        assert_eq!(from_file, Some(Some("/tmp/auth.json".to_string())));
    }
}
```

`CodexCommands` is already re-exported from `src/cli/mod.rs` (line 8: `pub use crate::cli::cli::{Cli, CodexCommands, Commands, StatuslineAction};`), so the test import works as-is.

- [ ] **Step 2: Run new tests to confirm they fail**

Run: `cargo test --test codex_tests cli_parse_tests 2>&1 | tail -20`

Expected: compile error (`from_file` is `Option<String>`, not `Option<Option<String>>`).

- [ ] **Step 3: Update `CodexCommands::Add::from_file` in `src/cli/cli.rs`**

Replace:

```rust
        #[arg(long = "from-file", help = "Import from existing auth.json file")]
        from_file: Option<String>,
```

with:

```rust
        /// Import from existing auth.json file
        ///
        /// With no value, imports from `~/.codex/auth.json`.
        /// With a value, imports from the given path.
        #[arg(
            long = "from-file",
            num_args = 0..=1,
            value_name = "PATH",
            help = "Import from auth.json (defaults to ~/.codex/auth.json if no path)"
        )]
        from_file: Option<Option<String>>,
```

- [ ] **Step 4: Update `handle_codex_add` signature and body**

In `src/codex/commands.rs`, change the function signature line 14 from:

```rust
    from_file: Option<String>,
```

to:

```rust
    from_file: Option<Option<String>>,
```

Then change the `let config = ...` block (lines 30–50) to resolve the tri-state:

```rust
    let config = if let Some(maybe_path) = from_file {
        let path: String = match maybe_path {
            Some(p) => p,
            None => crate::codex::default_codex_auth_path()?
                .to_string_lossy()
                .into_owned(),
        };
        if !std::path::Path::new(&path).exists() {
            anyhow::bail!(
                "Codex auth file not found: {}\n\
                 If you intended to import from Codex's default config, run `codex login` first.\n\
                 Otherwise pass an explicit path: --from-file <path>",
                path
            );
        }
        parse_auth_json_file(&path, &alias_name)?
    } else if interactive {
        parse_interactive_codex_config(&alias_name)?
    } else {
        let key = api_key.ok_or_else(|| {
            anyhow!(
                "API key is required. Use --api-key <key>, --from-file [<path>], or -i for interactive mode."
            )
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
```

- [ ] **Step 5: Verify the caller compiles unchanged**

The call site at `src/cli/main.rs:818-825` already destructures `from_file` from the `CodexCommands::Add` variant and forwards it to `handle_codex_add`. Because the type changes in lockstep on both ends (`Option<Option<String>>` in the variant *and* in the function signature), the call site needs no edits.

Verify:

Run: `cargo build 2>&1 | tail -10`

Expected: clean build. If a compile error still references `handle_codex_add`, recheck the signature change in Step 4.

- [ ] **Step 6: Run new tests to confirm they pass**

Run: `cargo test --test codex_tests cli_parse_tests 2>&1 | tail -10`

Expected: 2 tests pass.

- [ ] **Step 7: Run full test suite**

Run: `cargo test 2>&1 | tail -15`

Expected: all tests pass.

- [ ] **Step 8: Commit**

```bash
git add src/cli/cli.rs src/cli/main.rs src/codex/commands.rs tests/codex_tests.rs
git commit -m "$(cat <<'EOF'
feat(codex): codex add --from-file accepts optional path

With no value, imports from ~/.codex/auth.json; with a value, imports
from the given path. Mirrors the Claude add behavior.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: Integration test — default-path import for Claude

**Files:**
- Create: new test in `tests/integration_tests.rs`

This test exercises the parser end-to-end with a real `settings.json` shape.

- [ ] **Step 1: Add an integration test**

Append to `tests/integration_tests.rs`:

```rust
#[test]
fn test_parse_claude_settings_json_shape() {
    use cc_switch::config::types::ClaudeSettings;
    use std::collections::BTreeMap;
    use tempfile::TempDir;

    let tmp = TempDir::new().expect("Should create temp dir");
    let settings_path = tmp.path().join("settings.json");

    let settings_json = r#"{
        "env": {
            "ANTHROPIC_AUTH_TOKEN": "sk-ant-test-12345",
            "ANTHROPIC_BASE_URL": "https://api.test.example.com",
            "ANTHROPIC_MODEL": "claude-3-5-sonnet-20241022"
        }
    }"#;

    std::fs::write(&settings_path, settings_json).expect("Should write");

    let content = std::fs::read_to_string(&settings_path).expect("Should read back");
    let parsed: ClaudeSettings = serde_json::from_str(&content).expect("Should parse");

    let env: BTreeMap<String, String> = parsed.env;
    assert_eq!(
        env.get("ANTHROPIC_AUTH_TOKEN").map(|s| s.as_str()),
        Some("sk-ant-test-12345")
    );
    assert_eq!(
        env.get("ANTHROPIC_BASE_URL").map(|s| s.as_str()),
        Some("https://api.test.example.com")
    );
    assert_eq!(
        env.get("ANTHROPIC_MODEL").map(|s| s.as_str()),
        Some("claude-3-5-sonnet-20241022")
    );
}
```

This locks in the contract that a real `settings.json` shape parses cleanly via the public `ClaudeSettings` type, which `parse_config_from_file` mirrors.

- [ ] **Step 2: Run the test**

Run: `cargo test --test integration_tests test_parse_claude_settings_json_shape 2>&1 | tail -10`

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add tests/integration_tests.rs
git commit -m "$(cat <<'EOF'
test(integration): lock in settings.json shape parsed by --from-file

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: Update shell completion descriptions

**Files:**
- Modify: `src/cli/completion.rs`

- [ ] **Step 1: Examine current completion content**

Run: `grep -n "from-file" src/cli/completion.rs`

You'll see two lines (one inside a Rust string literal, one in a heredoc-style block) using:

```
'Import from JSON file'
```

- [ ] **Step 2: Update both descriptions**

In `src/cli/completion.rs`, find the two `--from-file` description lines and replace `Import from JSON file` with `Import from JSON file (defaults to ~/.codex/auth.json)`.

Concretely, run:

```bash
sed -i.bak "s|Import from JSON file'|Import from JSON file (defaults to ~/.codex/auth.json if no path)'|g" src/cli/completion.rs
rm src/cli/completion.rs.bak
```

(macOS `sed` requires `-i ''` instead of `-i.bak` if you prefer no backup file; either form is fine.)

Verify:

Run: `grep -n "from-file" src/cli/completion.rs`

Expected: descriptions updated.

- [ ] **Step 3: Also look for any Claude-side completion that needs updating**

Run: `grep -in "from.file\|--from-file" src/cli/completion.rs`

If only the two codex lines exist (as found in current grep), the Claude side has no explicit completion entry for `--from-file` — `clap_complete` likely generates Claude completions dynamically. No further change needed. If you find a Claude-side hand-written line, update it analogously with `~/.claude/settings.json` as the default.

- [ ] **Step 4: Re-run completion tests**

Run: `cargo test --test completion_tests 2>&1 | tail -10`

Expected: PASS. If any test asserts the old description verbatim, update the expected string to match.

- [ ] **Step 5: Commit**

```bash
git add src/cli/completion.rs
git commit -m "$(cat <<'EOF'
chore(completion): reflect --from-file default path in descriptions

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 7: Update documentation

**Files:**
- Modify: `src/cli/cli.rs` (the `long_about` string, lines 10–50)
- Modify: `CLAUDE.md`
- Modify: `README.md`
- Modify: `README_EN.md`
- Modify: `docs/codex.md`
- Modify: `docs/codex_EN.md`

- [ ] **Step 1: Update CLI `long_about` examples**

In `src/cli/cli.rs`, find the `EXAMPLES:` block around line 12. Replace the line:

```text
    cc-switch add my-config -i  # Interactive mode
```

with:

```text
    cc-switch add my-config -i                       # Interactive mode
    cc-switch add my-config --from-file              # Import from ~/.claude/settings.json
    cc-switch add my-config --from-file ./other.json # Import from an explicit JSON file
```

In the `CODEX CONFIGURATIONS:` block around line 25, replace:

```text
    cc-switch codex add work --from-file ~/.codex/auth.json
```

with:

```text
    cc-switch codex add work --from-file                       # Import from ~/.codex/auth.json
    cc-switch codex add work --from-file ~/other/auth.json     # Import from an explicit path
```

- [ ] **Step 2: Update `CLAUDE.md`**

Find line 295:

```text
cc-switch add my-config --from-file config.json  # Import from JSON
```

Replace with:

```text
cc-switch add my-config --from-file                   # Import from ~/.claude/settings.json
cc-switch add my-config --from-file config.json       # Import from a specific JSON file
```

- [ ] **Step 3: Update `README.md`**

Three occurrences:

| Line | Old | New |
|---|---|---|
| 46 | `cc-switch codex add work --from-file ~/.codex/auth.json` | `cc-switch codex add work --from-file              # 默认从 ~/.codex/auth.json 导入` |
| 213 | `cc-switch add --from-file config.json` | `cc-switch add work --from-file                   # 从 ~/.claude/settings.json 导入\ncc-switch add work --from-file config.json       # 从指定文件导入` |
| 472 | `cc-switch add --from-file my-work-config.json` | `cc-switch add my-work --from-file my-work-config.json` |
| 487 | `cc-switch codex add work --from-file ~/.codex/auth.json` | `cc-switch codex add work --from-file              # 默认 ~/.codex/auth.json` |

Open `README.md`, locate each line by its old content, and replace with the new content. Verify with `grep -n "from-file" README.md`.

- [ ] **Step 4: Update `README_EN.md` (mirror Step 3)**

Same line numbers (approximately), translated:

| Line | Old | New |
|---|---|---|
| 44 | `cc-switch codex add work --from-file ~/.codex/auth.json` | `cc-switch codex add work --from-file              # imports from ~/.codex/auth.json` |
| 198 | `cc-switch add --from-file config.json` | `cc-switch add work --from-file                   # import from ~/.claude/settings.json\ncc-switch add work --from-file config.json       # import from a specific file` |
| 457 | `cc-switch add --from-file my-work-config.json` | `cc-switch add my-work --from-file my-work-config.json` |
| 472 | `cc-switch codex add work --from-file ~/.codex/auth.json` | `cc-switch codex add work --from-file              # default ~/.codex/auth.json` |

- [ ] **Step 5: Update `docs/codex.md` and `docs/codex_EN.md`**

Three occurrences per file (lines 11, 42, 67). For each, change:

```text
cc-switch codex add work --from-file ~/.codex/auth.json
```

to:

```text
cc-switch codex add work --from-file                       # default: ~/.codex/auth.json
cc-switch codex add work --from-file ~/.codex/auth.json    # explicit path also supported
```

(or pick one form per occurrence depending on context — only the first occurrence in each file needs both forms; later ones can use the shorter `cc-switch codex add work --from-file`.)

- [ ] **Step 6: Verify no stale references remain**

Run: `grep -rn "filename becomes alias\|filename as alias\|uses filename" .`

Expected: no matches (the old semantics are gone from all docs).

Run: `grep -rn "from-file" docs/ README.md README_EN.md CLAUDE.md src/cli/cli.rs`

Expected: every reference now reflects the new semantics.

- [ ] **Step 7: Commit**

```bash
git add CLAUDE.md README.md README_EN.md docs/codex.md docs/codex_EN.md src/cli/cli.rs
git commit -m "$(cat <<'EOF'
docs: --from-file accepts optional path; default = tool's settings file

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 8: Final verification gate

**Files:**
- (no source changes; verification only)

- [ ] **Step 1: Format check**

Run: `cargo fmt --check 2>&1 | tail -5`

Expected: no output (formatting clean).

If output appears, run `cargo fmt` and commit the result:

```bash
git add -A
git commit -m "chore: cargo fmt"
```

- [ ] **Step 2: Clippy with warnings as errors**

Run: `cargo clippy --all-targets -- -D warnings 2>&1 | tail -20`

Expected: no warnings.

Fix any issues that surface. Re-commit if changes were needed.

- [ ] **Step 3: Run all tests**

Run: `cargo test 2>&1 | tail -10`

Expected: all tests pass. Library tests, integration tests, doc tests.

- [ ] **Step 4: Security audit**

Run: `cargo audit 2>&1 | tail -10`

Expected: no vulnerabilities.

- [ ] **Step 5: Manual smoke test — Claude default-path import**

```bash
# Pre-flight: snapshot of any existing config
ls ~/.claude/settings.json 2>/dev/null && echo "settings.json exists"

# Build
cargo build --release

# If you have a real ~/.claude/settings.json with ANTHROPIC_AUTH_TOKEN, run:
./target/release/cc-switch add smoke-test --from-file
./target/release/cc-switch list | grep smoke-test
./target/release/cc-switch remove smoke-test
```

Expected: alias `smoke-test` is added with the env values from `~/.claude/settings.json`.

If you don't have a real settings.json, stage one in a temp HOME:

```bash
mkdir -p /tmp/cc-switch-smoke/.claude
cat > /tmp/cc-switch-smoke/.claude/settings.json <<'EOF'
{
  "env": {
    "ANTHROPIC_AUTH_TOKEN": "sk-ant-smoke",
    "ANTHROPIC_BASE_URL": "https://example.test"
  }
}
EOF
HOME=/tmp/cc-switch-smoke ./target/release/cc-switch add smoke-test --from-file
HOME=/tmp/cc-switch-smoke ./target/release/cc-switch list | grep smoke-test
HOME=/tmp/cc-switch-smoke ./target/release/cc-switch remove smoke-test
rm -rf /tmp/cc-switch-smoke
```

- [ ] **Step 6: Manual smoke test — error path when default file missing**

```bash
HOME=/tmp/cc-switch-empty ./target/release/cc-switch add nothing --from-file 2>&1 | head -5
rm -rf /tmp/cc-switch-empty
```

Expected: clear error message ending in "pass an explicit path: --from-file <path>".

- [ ] **Step 7: Final commit if anything was tweaked during verification**

```bash
git status
# If clean, you're done.
# If changes exist, commit with appropriate message.
```
