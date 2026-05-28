# cc-switch

**English | [中文](README.md)**

[![Crates.io](https://img.shields.io/crates/v/cc-switch.svg)](https://crates.io/crates/cc-switch)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Platforms](https://img.shields.io/badge/platforms-Linux%20%7C%20macOS%20%7C%20Windows-blue)](#installation)

**A CLI tool for managing multiple Claude / Codex configurations and automatically switching between them. Cross-platform: Linux, macOS, Windows (x86_64 + ARM64).**

> ⚡ **Zero background processes**: cc-switch flips configurations, launches Claude / Codex, then **exits immediately** — no daemon, no listening port, no resident footprint.
>
> 📘 **Codex users**: full configuration management docs live in [docs/codex_EN.md](docs/codex_EN.md).

---

## Highlights

- 🚀 **Zero background** — exits as soon as Claude / Codex starts; never resident
- 🛡️ **Bypass permissions on by default** — cc-switch launches Claude with `--dangerously-skip-permissions`, so tool calls don't prompt each time (details below)
- 🧩 **Multi-config switching** — interactive TUI plus a fast `use` command
- ⌨️ **Native Vim keybindings** — `j` / `k` to move, `n` / `p` to page through configs; built for Vim geeks
- 🎯 **StatusLine integration** — shows the active alias in Claude Code's status bar (see [StatusLine integration](#statusline-integration))
- ⚡ **Completion in every shell** — Fish / Zsh / Bash / PowerShell / Elvish all get full completion; Fish adds live alias completion on top
- 📂 **Codex support** — manage Claude and OpenAI Codex auth from one tool
- 🌍 **Cross-platform** — macOS / Linux / Windows (x86_64 + ARM64); transparently handles npm-installed `claude.cmd` / `codex.cmd` shims on Windows
- 📊 **Daemon mode + aggregate dashboard** — optional background proxy that transparently captures all Claude API traffic, with a web dashboard for real-time request inspection, structured conversation views, and token statistics (see [Daemon Mode](#daemon-mode))

> 🛡️ **About the default bypass-permissions behavior**
>
> Both `cc-switch use <name>` and interactive selection launch Claude **with `--dangerously-skip-permissions` automatically applied** — file reads/writes, Bash calls, and other tool actions no longer prompt one by one. This is the opinionated default for geek / power users.

## Quick Start

```bash
# Install
cargo install cc-switch

# ===== Claude configurations =====
cc-switch add work sk-ant-work-xxx https://api.anthropic.com
cc-switch                       # interactive menu, pick 'work'
cc-switch use work              # or switch + launch Claude (cc-switch then exits)

# ===== Codex configurations =====  (full docs: docs/codex_EN.md)
cc-switch codex add work --from-file              # imports from ~/.codex/auth.json
cc-switch codex use work        # switch + launch Codex

# List all configurations
cc-switch list                  # Claude
cc-switch codex list            # Codex
```

## Installation

### macOS / Linux

**Option 1 — Homebrew (recommended):**

```bash
brew install Linuxdazhao/cc-switch/cc-switch
# Equivalent explicit form:
# brew tap Linuxdazhao/cc-switch && brew install cc-switch
```

**Option 2 — Cargo:**

```bash
cargo install cc-switch
```

**Option 3 — Pre-built binaries:** download the matching `.tar.gz` from [Releases](https://github.com/Linuxdazhao/cc_auto_switch/releases) and put `cc-switch` on your `PATH`.

### Windows

**Option 1 — Scoop (recommended, v0.1.18+):**

```powershell
scoop bucket add cc-switch https://github.com/Linuxdazhao/scoop-cc-switch
scoop install cc-switch
```

**Option 2 — Cargo:**

```powershell
cargo install cc-switch
```

**Option 3 — Pre-built binaries:** download the matching `.zip` from [Releases](https://github.com/Linuxdazhao/cc_auto_switch/releases) and put `cc-switch.exe` on your `PATH`.

## Main Commands

### Claude Configuration Management

| Command | What it does |
|---------|--------------|
| `cc-switch add <name>` | Add new configuration |
| `cc-switch list` | Show all configurations (JSON or plain text) |
| `cc-switch remove <name...>` | Delete one or more configurations |
| `cc-switch use <name>` | Switch and launch Claude (cc-switch then exits) |
| `cc-switch` | Enter interactive mode |

### Codex Configuration Management

| Command | What it does |
|---------|--------------|
| `cc-switch codex add <name>` | Add new configuration |
| `cc-switch codex list` | Show all configurations |
| `cc-switch codex remove <name...>` | Delete configurations |
| `cc-switch codex use <name>` | Switch and launch Codex |
| `cc-switch codex` | Enter interactive mode |

Full documentation: [docs/codex_EN.md](docs/codex_EN.md).

### Daemon Management

| Command | What it does |
|---------|--------------|
| `cc-switch daemon start` | Start the background proxy (one ccs-proxy per upstream) |
| `cc-switch daemon stop` | Stop the daemon |
| `cc-switch daemon status` | Show status, proxy list, and dashboard URL |
| `cc-switch daemon restart` | Restart (picks up config changes) |

### Common Commands

| Command | What it does |
|---------|--------------|
| `cc-switch statusline install` | Install the Claude Code statusLine wrapper (shows current alias) |
| `cc-switch statusline uninstall` | Remove the statusLine wrapper |
| `cc-switch completion <shell>` | Generate shell completion scripts |

## Why "zero background"?

cc-switch is a **one-shot command**:

1. You run `cc-switch use work`
2. cc-switch updates `~/.claude/settings.json` (or writes `~/.codex/auth.json`)
3. cc-switch `exec`s `claude --dangerously-skip-permissions` (or `codex`) with the right env vars
4. The cc-switch process **exits immediately** — everything that follows is Claude / Codex itself

Which means:

- ✅ No long-lived process, no port, no PID lockfile
- ✅ No `cc-switch start` / `cc-switch stop` ceremony
- ✅ Close Claude and the environment ends with it — no leftover state
- ✅ Once it exits, nothing of cc-switch is visible on the system except its config file
- 🛡️ The launched Claude has bypass-permissions enabled by default (see Highlights above)

## Advanced Usage

### Interactive Mode

```bash
# Claude interactive mode
cc-switch

# Codex interactive mode
cc-switch codex

# Navigation (arrows AND Vim-style keys both work):
# - ↑↓ or k/j: move up / down
# - 1-9: jump straight to that configuration
# - N/PageDown: next page (when >9 configs)
# - P/PageUp: previous page
# - R: reset to default Claude (Claude mode only)
# - E: edit configuration
# - Q: quit
```

### Quick Switch (use command)

```bash
# Switch to a configuration and launch Claude
cc-switch use work

# Switch and send a prompt
cc-switch use work "Write a Python script for me"

# Switch and resume a previous session
cc-switch use work --resume c8439f36-44a9-4d85-9e88-de35e004fdd4
cc-switch use work -r c8439f36-44a9-4d85-9e88-de35e004fdd4

# Switch and continue the most recent session
cc-switch use work --continue
cc-switch use work -c
```

### Add with Full Configuration

```bash
# Every option at once
cc-switch add work -t sk-ant-xxx -u https://api.anthropic.com \
  -m claude-3-5-sonnet-20241022 \
  --small-fast-model claude-3-haiku-20240307 \
  --max-thinking-tokens 8192 \
  --api-timeout-ms 300000 \
  --disable-nonessential-traffic 1 \
  --default-sonnet-model claude-3-5-sonnet-20241022 \
  --default-opus-model claude-3-opus-20240229 \
  --default-haiku-model claude-3-haiku-20240307

# Force overwrite
cc-switch add work -t sk-ant-xxx -u https://api.anthropic.com -f

# Interactive add
cc-switch add work -i

# Import from JSON file (alias required)
cc-switch add work --from-file                   # import from ~/.claude/settings.json
cc-switch add work --from-file config.json       # import from a specific file
```

### Storage Modes

> ⚠️ **If you run multiple Claude instances at once, use `env` mode (the default).**
>
> - **`env` mode (default, recommended)**: writes to the `env` field of `settings.json`. Claude reads these into the process environment at launch and **does not watch the file afterward**. Each window / session is fully isolated.
> - **`config` mode**: writes to root-level camelCase config fields in `settings.json`. Claude **hot-reads** these at runtime — so flipping cc-switch once will silently switch **every Claude instance already running** onto the new configuration. Avoid this unless you have exactly one window open and *want* the change to apply live.

```bash
cc-switch --store env    # Write to env field (default, safe with multiple instances)
cc-switch --store config # Write root-level camelCase (mutates live instances)
```

### List Configurations

```bash
cc-switch list           # JSON format (default)
cc-switch list -p        # Plain text format
```

### Remove Multiple Configurations

```bash
cc-switch remove work
cc-switch remove work personal test-config
```

### Configuration Migration

```bash
# Migrate from old path (~/.cc_auto_switch/) to the new one
cc-switch --migrate
```

## Daemon Mode

cc-switch offers an **optional** Daemon mode: it automatically starts a local ccs-proxy instance for each configured upstream URL, transparently capturing all Claude API requests/responses, and serves an aggregate web dashboard.

```bash
# Start the daemon
cc-switch daemon start

# Check status (includes dashboard URL)
cc-switch daemon status
# Example output:
#   ccs-daemon: RUNNING (pid 12345, uptime 5m 30s)
#   dashboard: http://127.0.0.1:55571

# Run in foreground (useful for debugging)
cc-switch daemon start --foreground

# Adjust log level
cc-switch daemon start --log-level debug
cc-switch daemon start -vvv   # trace level

# Restart (picks up configuration changes)
cc-switch daemon restart

# Stop
cc-switch daemon stop
```

### Dashboard Features

Open the dashboard URL shown by `cc-switch daemon status` in your browser to see:

- **Request list** — all API requests sorted by time, with live SSE updates
- **Upstream filter** — filter requests by upstream
- **Time window** — quick 1h / 24h / 7d / all filters
- **Token stats** — total requests, input/output tokens, average latency, error count
- **Structured detail view** (click any request row):
  - **Overview** — metadata grid (session, model, duration, TTFT, token usage)
  - **Request** — structured Anthropic Messages API view (System / Tools / message thread) with Markdown rendering and syntax highlighting
  - **Response** — assistant content blocks, tool_use/thinking blocks, stop_reason
  - **Structured ⇄ Raw** toggle to switch between structured and raw JSON views

> 💡 The daemon is entirely optional — all other cc-switch features work without it. After starting the daemon, your cc-switch workflow stays exactly the same (`use`, interactive mode, etc. all work as usual) — the daemon just captures traffic transparently in the background.

## StatusLine Integration

cc-switch can show the **current configuration alias** at the start of Claude Code's status bar so you can tell at a glance which API set you're on.

```bash
# Install (run after first install or after upgrading cc-switch)
cc-switch statusline install

# Uninstall
cc-switch statusline uninstall
```

How it works:

- cc-switch writes a shell wrapper to `~/.claude/cc_auto_switch_statusline.sh`
- It auto-detects [`ccstatusline`](https://www.npmjs.com/package/ccstatusline) (prefers `bunx`, falls back to `npx`) and uses it as the underlying statusLine command
- The status bar gains an `[alias]` prefix, e.g. `[work] /Users/you/project | claude-sonnet-4-6 | $0.12`
- If you already have a statusLine command in `settings.json`, it is **wrapped**, not replaced
- Uninstall restores the original command

Requires `bun` or `npm` on `PATH` (to run ccstatusline).

## Shell Integration

### Generate completion scripts

> **Re-generate completion scripts after upgrading** to pick up new subcommands (`codex`, `statusline`).

#### Fish / Zsh / Bash

```bash
# Fish (recommended — the only shell with dynamic alias completion)
cc-switch completion fish > ~/.config/fish/completions/cc-switch.fish

# Zsh
cc-switch completion zsh > ~/.zsh/completions/_cc-switch
echo 'fpath=(~/.zsh/completions $fpath)' >> ~/.zshrc

# Bash (incl. Git Bash on Windows)
cc-switch completion bash > ~/.bash_completion.d/cc-switch
echo 'source ~/.bash_completion.d/cc-switch' >> ~/.bashrc

# Elvish also supported
cc-switch completion elvish
```

#### PowerShell (Windows)

**Don't** redirect the completion script directly into `$PROFILE` — that overwrites your existing aliases, modules, and theme. Write it to a dedicated file and dot-source it:

```powershell
$completionDir = Split-Path -Parent $PROFILE
if (-not (Test-Path $completionDir)) { New-Item -ItemType Directory -Path $completionDir | Out-Null }
$completionPath = Join-Path $completionDir 'cc-switch.completion.ps1'
cc-switch completion powershell | Out-File -Encoding utf8 $completionPath

$line = ". '$completionPath'"
if (-not ((Test-Path $PROFILE) -and (Select-String -Path $PROFILE -Pattern ([regex]::Escape($line)) -Quiet))) {
    Add-Content -Path $PROFILE -Value $line
}
```

The snippet is idempotent — safe to re-run.

#### CMD (Windows)

CMD has no completion mechanism; just use the commands directly.

### Completion support matrix

**Fish / Zsh / Bash / PowerShell / Elvish all get full completion for subcommands, args, and flags.** *Dynamic alias completion* — pressing `<Tab>` to enumerate every config name live — is available in every shell: Fish ships it out-of-the-box; the others just need the snippets below.

| Shell | Static (commands / args / flags) | Dynamic alias completion (`use <Tab>` lists configs) |
|-------|----------------------------------|------------------------------------------------------|
| **Fish** | ✅ auto | ✅ **auto** (Claude + Codex) |
| **Zsh** | ✅ auto | ⚙️ snippet needed ([see below](#zsh-dynamic-alias-completion)) |
| **Bash** | ✅ auto | ⚙️ snippet needed ([see below](#bash-dynamic-alias-completion)) |
| **PowerShell** | ✅ auto | ⚙️ snippet needed ([see below](#powershell-dynamic-alias-completion)) |
| **Elvish** | ✅ auto | ⚙️ build your own with `edit:completion:arg-completer` |

> Mechanism: `cc-switch --list-aliases` and `cc-switch --list-codex-aliases` print every configured alias and are shell-agnostic. Fish's generated script already wires them into completion; for the other shells you just append a few lines.

### Enabling dynamic alias completion in other shells

> All snippets below are **additive** — paste them *after* the script produced by `cc-switch completion <shell>`. Static completion keeps working.

#### Zsh dynamic alias completion

Append to `~/.zshrc` (**after** the completion script is sourced and `compinit` has run):

```zsh
_cc_switch_dynamic_aliases() {
  local -a aliases
  local words_count=$#words
  local cmd1=$words[2]
  local cmd2=$words[3]

  # cc-switch codex use|remove <TAB>
  if [[ "$cmd1" == "codex" && ("$cmd2" == "use" || "$cmd2" == "remove") && $words_count -ge 4 ]]; then
    aliases=("${(@f)$(cc-switch --list-codex-aliases 2>/dev/null)}")
    compadd -a aliases
    return 0
  fi

  # cc-switch use|switch|remove <TAB>
  if [[ ("$cmd1" == "use" || "$cmd1" == "switch" || "$cmd1" == "remove") && $words_count -ge 3 ]]; then
    aliases=("${(@f)$(cc-switch --list-aliases 2>/dev/null)}")
    compadd -a aliases
    return 0
  fi
}
# Override the clap-generated _cc-switch with the dynamic version
compdef _cc_switch_dynamic_aliases cc-switch
```

#### Bash dynamic alias completion

Append to `~/.bashrc` (**after** `source ~/.bash_completion.d/cc-switch`):

```bash
_cc_switch_with_aliases() {
  # First run the clap-generated completion (handles subcommands, flags, etc.)
  _cc-switch "$@"

  # Then override COMPREPLY at alias positions
  local cur="${COMP_WORDS[COMP_CWORD]}"
  local prev="${COMP_WORDS[COMP_CWORD-1]}"
  local cmd1="${COMP_WORDS[1]:-}"

  case "$prev" in
    use|switch|remove)
      if [[ "$cmd1" == "codex" ]]; then
        COMPREPLY=($(compgen -W "$(cc-switch --list-codex-aliases 2>/dev/null)" -- "$cur"))
      else
        COMPREPLY=($(compgen -W "$(cc-switch --list-aliases 2>/dev/null)" -- "$cur"))
      fi
      ;;
  esac
}
complete -F _cc_switch_with_aliases -o nosort cc-switch
```

#### PowerShell dynamic alias completion

PowerShell's `Register-ArgumentCompleter` **composes** with existing completion — no replacement needed. Append to your PowerShell `$PROFILE` (or to the `cc-switch.completion.ps1` file from earlier):

```powershell
Register-ArgumentCompleter -CommandName cc-switch -Native -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $tokens = $commandAst.CommandElements | ForEach-Object { $_.ToString() }
    $count = $tokens.Count
    if ($count -lt 2) { return }

    $cmd1 = $tokens[1]
    $cmd2 = if ($count -ge 3) { $tokens[2] } else { '' }

    # cc-switch codex use|remove <TAB>
    if ($cmd1 -eq 'codex' -and ($cmd2 -eq 'use' -or $cmd2 -eq 'remove')) {
        cc-switch --list-codex-aliases 2>$null | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
            [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
        }
        return
    }

    # cc-switch use|switch|remove <TAB>
    if ($cmd1 -in 'use', 'switch', 'remove') {
        cc-switch --list-aliases 2>$null | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
            [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
        }
    }
}
```

### Built-in aliases (recommended)

`cc-switch completion <shell>` prints recommended aliases alongside the completion script. You can also add them manually:

| Alias | Expands to | Purpose |
|-------|------------|---------|
| `cs` | `cc-switch` | short form of the main command (typing `cs` drops you into interactive mode) |
| `ccd` | `claude --dangerously-skip-permissions` | launch Claude without permission prompts |
| `cx` | `cc-switch codex` | short form of the Codex subcommand |

```bash
# Fish
echo "alias cs='cc-switch'" >> ~/.config/fish/config.fish
echo "alias ccd='claude --dangerously-skip-permissions'" >> ~/.config/fish/config.fish
echo "alias cx='cc-switch codex'" >> ~/.config/fish/config.fish

# Zsh
echo "alias cs='cc-switch'" >> ~/.zshrc
echo "alias ccd='claude --dangerously-skip-permissions'" >> ~/.zshrc
echo "alias cx='cc-switch codex'" >> ~/.zshrc

# Bash
echo "alias cs='cc-switch'" >> ~/.bashrc
echo "alias ccd='claude --dangerously-skip-permissions'" >> ~/.bashrc
echo "alias cx='cc-switch codex'" >> ~/.bashrc

# PowerShell
Set-Alias -Name cs -Value cc-switch
function ccd { claude --dangerously-skip-permissions @args }
```

Then:

```bash
cs              # = cc-switch (interactive mode)
cs use work     # = cc-switch use work
ccd             # quick Claude launch (permission prompts skipped)
cx              # = cc-switch codex (Codex interactive mode)
cx use work     # = cc-switch codex use work
```

> 💡 **Fish tip**: dynamic completion still works through the `cs` alias — Fish expands the alias before completing.

## Import / Export

### Claude configurations from JSON

```bash
# Provide an explicit alias before --from-file
cc-switch add my-work --from-file my-work-config.json

# Expected JSON format:
# {
#   "env": {
#     "ANTHROPIC_AUTH_TOKEN": "sk-ant-xxx",
#     "ANTHROPIC_BASE_URL": "https://api.anthropic.com",
#     "ANTHROPIC_MODEL": "claude-3-5-sonnet-20241022"
#   }
# }
```

### Codex configurations from auth.json

```bash
cc-switch codex add work --from-file              # default ~/.codex/auth.json
```

Full documentation: [docs/codex_EN.md](docs/codex_EN.md).

## How it Works

cc-switch stores configurations in `~/.claude/cc_auto_switch_setting.json`:

- **Claude configurations**: update Claude's `settings.json` with the right environment variables
- **Codex configurations**: write `~/.codex/auth.json`, which the Codex CLI reads for auth

Which means:

- ✅ No global configuration changes
- ✅ Complete isolation between configurations
- ✅ Safe and secure API key management
- ✅ Works with any Claude / Codex installation
- ✅ Preserves other settings
- ✅ Supports a custom settings directory

## Environment Variables

### Claude Configurations

cc-switch sets the following when switching configuration:

- `ANTHROPIC_AUTH_TOKEN` - your API token
- `ANTHROPIC_BASE_URL` - API endpoint URL
- `ANTHROPIC_MODEL` - custom model (optional)
- `ANTHROPIC_SMALL_FAST_MODEL` - fast model for background tasks (optional)
- `ANTHROPIC_MAX_THINKING_TOKENS` - maximum thinking tokens (optional)
- `API_TIMEOUT_MS` - API timeout in ms (optional)
- `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` - disable non-essential traffic (optional)
- `ANTHROPIC_DEFAULT_SONNET_MODEL` - default Sonnet model (optional)
- `ANTHROPIC_DEFAULT_OPUS_MODEL` - default Opus model (optional)
- `ANTHROPIC_DEFAULT_HAIKU_MODEL` - default Haiku model (optional)
- `CLAUDE_CODE_SUBAGENT_MODEL` - subagent model (optional)
- `CLAUDE_CODE_DISABLE_NONSTREAMING_FALLBACK` - disable non-streaming fallback (optional)
- `CLAUDE_CODE_EFFORT_LEVEL` - effort level (optional, e.g. 'max')
- `CC_SWITCH_CURRENT_ALIAS` - current alias (injected by cc-switch for the statusLine wrapper)

### Codex Configurations

Codex configurations live in `~/.codex/auth.json` and support two auth modes:

**chatgpt (OAuth)**: `id_token`, `access_token`, `refresh_token`, `account_id`

**apikey**: `OPENAI_API_KEY`

Full documentation: [docs/codex_EN.md](docs/codex_EN.md).

## Development

```bash
# Clone
git clone https://github.com/Linuxdazhao/cc_auto_switch.git
cd cc-switch

# Build
cargo build --release

# Test
cargo test
```

## License

MIT License — see [LICENSE](LICENSE).

---

**Made with ❤️ by [Linuxdazhao](https://github.com/Linuxdazhao)**
