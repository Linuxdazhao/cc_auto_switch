# Codex Configuration Management

**[English](codex_EN.md) | [中文](codex.md)**

`cc-switch codex` manages multiple OpenAI Codex CLI authentication configurations, supporting both OAuth (chatgpt) and API Key modes.

## Quick Start

```bash
# Import from an existing auth.json
cc-switch codex add work --from-file ~/.codex/auth.json

# Create interactively
cc-switch codex add personal -i

# Create with an API key
cc-switch codex add api-test --api-key sk-xxx

# Enter interactive mode
cc-switch codex

# Switch to a configuration and launch Codex
cc-switch codex use work
```

## Command Reference

| Command | What it does |
|---------|--------------|
| `cc-switch codex` | Enter interactive mode (TUI) |
| `cc-switch codex add <name>` | Add new configuration |
| `cc-switch codex list` | List all configurations |
| `cc-switch codex use <name>` | Switch and launch Codex |
| `cc-switch codex remove <name...>` | Delete configurations |

## Adding Configurations

### Import from existing auth.json

```bash
# Import — the filename (without extension) becomes the alias
cc-switch codex add work --from-file ~/.codex/auth.json
```

### Interactive creation

```bash
cc-switch codex add my-config -i

# Prompts:
# Auth mode (chatgpt/apikey) [chatgpt]:
# ID Token:
# Access Token:
# Refresh Token:
# Account ID:
```

### API Key mode

```bash
cc-switch codex add api-only --api-key sk-xxxxxxxx
```

### Force overwrite

```bash
cc-switch codex add work --from-file ~/.codex/auth.json -f
```

## Interactive Mode (TUI)

```bash
# Enter the interactive selection interface
cc-switch codex
```

Navigation:

- `↑↓` / `j` `k`: move up / down
- `1-9`: quick-select configuration on current page
- `N` / `PageDown`: next page
- `P` / `PageUp`: previous page
- `Enter`: confirm selection, switch and launch Codex
- `E`: edit the selected configuration
- `Q`: quit without saving
- `Esc`: cancel

Each configuration displays:
- Auth mode (apikey / chatgpt)
- Account ID (chatgpt mode)
- API Key prefix (apikey mode)
- Last refresh time (if any)

## Using Configurations

```bash
# Switch and launch Codex
cc-switch codex use work

# Switch and send a prompt
cc-switch codex use work "Write a Python script for me"

# Switch and continue the most recent session
cc-switch codex use work -c

# Switch and resume a specific session
cc-switch codex use work -r <session-id>
```

## Editing Configurations

In interactive mode, select a configuration and press `E` to enter edit mode.

Editable fields:

| # | Field | Description |
|---|-------|-------------|
| 1 | alias_name | Alias |
| 2 | auth_mode | Auth mode (chatgpt / apikey) |
| 3 | OPENAI_API_KEY | API key |
| 4 | id_token | ID token |
| 5 | access_token | Access token |
| 6 | refresh_token | Refresh token |
| 7 | account_id | Account ID |
| 8 | last_refresh | Last refresh time |

Edit mode controls:
- Enter a field number to modify it
- Press Enter to keep the current value; enter a space to clear optional fields
- `S`: save changes
- `Q`: discard and go back

## Listing and Removing Configurations

```bash
# List all Codex configurations (JSON format)
cc-switch codex list

# Plain text format
cc-switch codex list -p

# Remove a single configuration
cc-switch codex remove work

# Remove multiple configurations
cc-switch codex remove work personal test
```

## Auth Mode Reference

### chatgpt mode (OAuth)

Uses OpenAI account OAuth authentication with the following tokens:

- `id_token` - identity token
- `access_token` - access token
- `refresh_token` - refresh token
- `account_id` - account ID

Best for users with a ChatGPT Plus / Team / Enterprise subscription.

### apikey mode

Uses an OpenAI API Key:

- `OPENAI_API_KEY` - API key

Best for users on the pay-as-you-go API.

## Data Storage

Codex configurations are stored alongside Claude configurations in `~/.claude/cc_auto_switch_setting.json`.

When switching, the tool writes `~/.codex/auth.json`, which the Codex CLI reads for authentication.
