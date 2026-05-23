# `--from-file` Default Import Design

**Date**: 2026-05-23
**Status**: Approved

## Goal

Unify the `--from-file` semantics for both `cc-switch add` (Claude) and
`cc-switch codex add` so that:

- Passing `--from-file` **without a path** imports from the tool's default
  config file (`~/.claude/settings.json` for Claude, `~/.codex/auth.json` for
  Codex).
- Passing `--from-file <path>` still imports from the specified file.
- The user-provided alias is **always** used. The old Claude behavior of
  deriving alias from the file's basename is removed.

This mirrors Codex's existing import pattern while extending it to default
locations, eliminating an inconsistency between the two `add` commands.

## Motivation

The current Claude `--from-file` flag has two quirks that diverge from
Codex's:

1. It requires an explicit path even though the canonical Claude config file
   lives at a well-known location (`~/.claude/settings.json`).
2. It overrides the positional alias with the file's stem, making the alias
   argument effectively useless when `--from-file` is present
   (`required_unless_present = "from_file"`).

After this change, both commands share one mental model: "alias name is always
required; `--from-file` optionally takes a path, otherwise reads from the
tool's default config."

## Behavior Matrix

| Command | Before | After |
|---|---|---|
| `cc-switch add work --from-file <path>` | Path required. Alias replaced by filename stem. | Alias kept. Reads `<path>`. |
| `cc-switch add work --from-file` | Not supported (path required). | Reads `~/.claude/settings.json`. |
| `cc-switch add --from-file <path>` (no alias) | Allowed — alias from filename. | **Error** — alias required. |
| `cc-switch codex add work --from-file <path>` | Path required. | Unchanged in semantics; path now optional. |
| `cc-switch codex add work --from-file` | Not supported. | Reads `~/.codex/auth.json`. |

## CLI Changes

### `src/cli/cli.rs`

**`Commands::Add`**:

- `alias_name`: drop `required_unless_present = "from_file"` and change the field type from `Option<String>` to `String` (positional becomes required by default, so the wrapping `Option` is no longer needed).
- `from_file` field changes type:

  ```rust
  #[arg(
      long = "from-file",
      num_args = 0..=1,
      value_name = "PATH",
      help = "Import from JSON file (defaults to ~/.claude/settings.json if no path given)"
  )]
  from_file: Option<Option<String>>,
  ```

- Drop the existing `short = 'j'`.

**`CodexCommands::Add`**:

- `from_file` field changes to `Option<Option<String>>` with the same
  `num_args = 0..=1` pattern, default location `~/.codex/auth.json`.
- No short flag.

Tri-state interpretation everywhere:

| Value | Meaning |
|---|---|
| `None` | `--from-file` not present; normal path. |
| `Some(None)` | `--from-file` present, no path; use default location. |
| `Some(Some(path))` | `--from-file <path>` present; use given path. |

### `src/cli/cli.rs` `long_about`

Update `EXAMPLES` and `CODEX CONFIGURATIONS` blocks:

```text
cc-switch add my-config --from-file               # import from ~/.claude/settings.json
cc-switch add my-config --from-file ./other.json  # import from a specific file

cc-switch codex add work --from-file              # import from ~/.codex/auth.json
cc-switch codex add work --from-file ~/other.json
```

## Handler Changes

### `src/cli/main.rs`

**`parse_config_from_file`**:

- Remove the `file_alias_name` return value and the `path.file_stem()` block
  that derives it. The function returns the configuration tuple only.
- Signature simplifies — caller no longer destructures an alias string.

**`handle_add_command`**:

- Replace the current `if let Some(file_path) = &params.from_file` block with a
  match over the tri-state:

  ```rust
  if let Some(maybe_path) = &params.from_file {
      let resolved_path = match maybe_path {
          Some(p) => p.clone(),
          None => get_claude_settings_path(custom_dir)?.to_string_lossy().into_owned(),
      };
      // parse resolved_path, populate params (NOT alias_name)
  }
  ```

- Delete the line `params.alias_name = file_alias_name;`.
- Keep the `Cannot use --interactive mode with --from-file` guard, adjusted
  for the new type (`params.from_file.is_some()`).
- The `AddCommandParams` struct's `from_file: Option<String>` field type stays
  — only the CLI parsing layer holds the tri-state. The handler resolves it
  to a concrete path string (or `None` for "not provided") before populating
  `params`.

**Call sites that build `AddCommandParams`** (`run()` around lines 600–664):

- The `from_file` placeholder block (lines 631–641) — `let final_alias_name = if from_file.is_some() { "placeholder".to_string() } else { ... }` — disappears entirely. With `alias_name: String` from clap, pass it through directly.
- Resolve `from_file: Option<Option<String>>` from clap down to `Option<String>` (the path, or `None` when the user passed `--from-file` with no value the resolver substitutes the default location string) before constructing `AddCommandParams`. Alternative: change `AddCommandParams::from_file` to `Option<Option<String>>` and resolve inside `handle_add_command` — pick whichever keeps the handler signature cleaner; the spec leaves this to the implementer.

### `src/codex/commands.rs`

**`handle_codex_add`**:

- `from_file` parameter type changes to `Option<Option<String>>`.
- Resolve path inside the function:

  ```rust
  let config = if let Some(maybe_path) = from_file {
      let path = match maybe_path {
          Some(p) => p,
          None => default_codex_auth_path()?,
      };
      parse_auth_json_file(&path, &alias_name)?
  } else if interactive {
      ...
  };
  ```

- Error message for missing API key updated:
  `"API key is required. Use --api-key <key>, --from-file [<path>], or -i for interactive mode."`

### Default-path helpers

**`src/utils.rs`** — already exports `get_claude_settings_path(custom_dir: Option<&str>) -> Result<PathBuf>`.

**`src/codex/auth_writer.rs`** — has a private `get_auth_path(base_dir: Option<&PathBuf>) -> Result<PathBuf>` that *also creates* the `.codex` directory as a side effect, which is wrong for a read-only default-location lookup. Add a sibling **read-only** helper:

```rust
pub fn default_codex_auth_path() -> Result<PathBuf> {
    Ok(dirs::home_dir()
        .ok_or_else(|| anyhow!("Could not find home directory"))?
        .join(".codex")
        .join("auth.json"))
}
```

Do not change `get_auth_path` — it's still correct for the write path.

## Error Handling

- **Default file missing**: emit a guidance message:
  - Claude: `~/.claude/settings.json not found. Run \`claude\` once to create it, or pass an explicit path: --from-file <path>`
  - Codex: `~/.codex/auth.json not found. Run \`codex login\` first, or pass an explicit path: --from-file <path>`
- **Field missing** (e.g., no `env` section, missing `ANTHROPIC_AUTH_TOKEN`, missing `auth_mode`): preserve existing error messages from `parse_config_from_file` / `parse_auth_json_file`.

## Shell Completion

`src/cli/completion.rs`:

- Update `--from-file` description in fish/zsh/bash completion blocks for both
  `add` and `codex add` to reflect the optional value semantics.
- Since `--from-file` now optionally takes a path, the completion should still
  offer file path completion, but the flag is also valid standalone. The fish
  `-r` (requires argument) flag on the relevant lines should be reviewed; if
  fish doesn't have a clean "optional value" mode, leave file completion on
  and document the no-arg form in help text.

## Tests

New tests (in `tests/`):

1. **Claude — default path import**: stage a temp `~/.claude/settings.json`
   (using `--set-default-dir` or test-env override), run
   `cc-switch add work --from-file`, assert config is imported with alias `work`.
2. **Claude — explicit path import**: write a temp JSON, run
   `cc-switch add work --from-file <tmp>`, assert alias is `work` (not the
   filename stem).
3. **Claude — alias required**: `cc-switch add --from-file <tmp>` (no alias)
   should exit non-zero with clap's required-argument error.
4. **Claude — default path missing**: when target file doesn't exist, assert
   the guidance error message.
5. **Codex — default path import**: stage temp `~/.codex/auth.json`, run
   `cc-switch codex add work --from-file`, assert success.
6. **Codex — explicit path import**: existing path-based tests stay valid.

Update tests that previously relied on the filename-as-alias behavior to pass
alias explicitly.

## Documentation

- `CLAUDE.md` — update the "CLI Usage Patterns" `--from-file` example.
- `README.md` and `README_zh.md` — update import examples to show both
  no-path and explicit-path forms for both `add` and `codex add`.

## Out of Scope

- No deprecation warning for the removed behavior — `废弃` means hard removal.
  Users who pass `--from-file` without an alias get a clap error message that
  clearly states alias is required.
- No new `--from-default` flag; reusing `--from-file` keeps the surface small.
- No change to the interactive mode (`-i`), the JSON output, the storage
  model, or how `switch_to_config` writes settings.json.
