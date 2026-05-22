# cc-switch

**[English](README_EN.md) | 中文**

[![Crates.io](https://img.shields.io/crates/v/cc-switch.svg)](https://crates.io/crates/cc-switch)
[![Downloads](https://img.shields.io/crates/d/cc-switch.svg)](https://crates.io/crates/cc-switch)
[![CI](https://github.com/Linuxdazhao/cc_auto_switch/actions/workflows/ci.yml/badge.svg)](https://github.com/Linuxdazhao/cc_auto_switch/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Platforms](https://img.shields.io/badge/platforms-Linux%20%7C%20macOS%20%7C%20Windows-blue)](#安装)

**一个 CLI 工具，用于管理多个 Claude / Codex 配置并在它们之间自动切换。全平台支持：Linux、macOS、Windows（x86_64 + ARM64）。**

> ⚡ **零后台进程**：cc-switch 切换配置、启动 Claude / Codex 后**立即退出**——不留守护进程、不监听端口、不占用资源。
>
> 📘 **Codex 用户**：完整配置管理文档请直接查看 [docs/codex.md](docs/codex.md)。

---

## 核心特性

- 🚀 **零后台** — 启动 Claude / Codex 后立即退出，绝不驻留
- 🛡️ **默认 Bypass Permissions On** — cc-switch 启动 Claude 时自动加 `--dangerously-skip-permissions`，工具调用不再每次确认（详见下方说明）
- 🧩 **多配置切换** — 交互式 TUI + `use` 快捷命令
- ⌨️ **天然支持 Vim 键位** — 交互模式下 `j` / `k` 上下移动、`n` / `p` 翻页，专为 Vim 极客而生
- 🎯 **StatusLine 集成** — 在 Claude Code 状态栏实时显示当前别名（详见 [StatusLine 集成](#statusline-集成)）
- ⚡ **全 Shell 补全 + Fish 动态别名补全** — Fish / Zsh / Bash / PowerShell / Elvish 全部支持，Fish 额外提供 `<Tab>` 实时列出配置名
- 📂 **Codex 支持** — 同一工具管理 Claude 和 OpenAI Codex 两套认证
- 🌍 **全平台** — macOS / Linux / Windows（x86_64 + ARM64），自动处理 Windows 上的 npm `claude.cmd` / `codex.cmd` shim

> 🛡️ **关于默认 Bypass Permissions**
>
> 无论是 `cc-switch use <name>` 还是交互模式选择配置，cc-switch 都会**自动以 `--dangerously-skip-permissions` 启动 Claude**——文件读写、Bash 执行等操作不再逐条弹出确认。这是为极客 / 重度用户优化的默认行为。

## 快速开始

```bash
# 安装
cargo install cc-switch

# ===== Claude 配置 =====
cc-switch add work sk-ant-work-xxx https://api.anthropic.com
cc-switch                       # 进入交互菜单，选择 'work'
cc-switch use work              # 或直接切换并启动 Claude（cc-switch 随后退出）

# ===== Codex 配置 =====（完整文档：docs/codex.md）
cc-switch codex add work --from-file ~/.codex/auth.json
cc-switch codex use work        # 切换并启动 Codex

# 列出所有配置
cc-switch list                  # Claude
cc-switch codex list            # Codex
```

## 安装

### macOS / Linux

**方式 1 — Homebrew（推荐）：**

```bash
brew install Linuxdazhao/cc-switch/cc-switch
# 等价的显式写法：
# brew tap Linuxdazhao/cc-switch && brew install cc-switch
```

**方式 2 — Cargo：**

```bash
cargo install cc-switch
```

**方式 3 — 预编译二进制：** 从 [Releases](https://github.com/Linuxdazhao/cc_auto_switch/releases) 下载对应架构的 `.tar.gz`，将 `cc-switch` 放到 `PATH` 中。

### Windows

**方式 1 — Scoop（推荐，v0.1.18+）：**

```powershell
scoop bucket add cc-switch https://github.com/Linuxdazhao/scoop-cc-switch
scoop install cc-switch
```

**方式 2 — Cargo：**

```powershell
cargo install cc-switch
```

**方式 3 — 预编译二进制：** 从 [Releases](https://github.com/Linuxdazhao/cc_auto_switch/releases) 下载对应架构的 `.zip`，将 `cc-switch.exe` 放到 `PATH` 中。

## 主要命令

### Claude 配置管理

| 命令 | 作用 |
|------|------|
| `cc-switch add <名称>` | 添加新配置 |
| `cc-switch list` | 显示所有配置（JSON 或纯文本） |
| `cc-switch remove <名称...>` | 删除一个或多个配置 |
| `cc-switch use <名称>` | 快速切换配置并启动 Claude（启动后 cc-switch 退出） |
| `cc-switch` | 进入交互模式 |

### Codex 配置管理

| 命令 | 作用 |
|------|------|
| `cc-switch codex add <名称>` | 添加新配置 |
| `cc-switch codex list` | 显示所有配置 |
| `cc-switch codex remove <名称...>` | 删除配置 |
| `cc-switch codex use <名称>` | 切换配置并启动 Codex |
| `cc-switch codex` | 进入交互模式 |

完整文档：[docs/codex.md](docs/codex.md)。

### 通用命令

| 命令 | 作用 |
|------|------|
| `cc-switch statusline install` | 安装 Claude Code statusLine 包装器（显示当前别名） |
| `cc-switch statusline uninstall` | 卸载 statusLine 包装器 |
| `cc-switch completion <shell>` | 生成 Shell 补全脚本 |

## 工作模式：为什么是"零后台"

cc-switch 是一个**一次性命令**：

1. 你执行 `cc-switch use work`
2. cc-switch 修改 `~/.claude/settings.json`（或写入 `~/.codex/auth.json`）
3. cc-switch 用新环境变量 `exec` 启动 `claude --dangerously-skip-permissions`（或 `codex`）
4. cc-switch 进程**立即退出**——后续完全是 Claude / Codex 自己在跑

这意味着：

- ✅ 没有常驻进程、没有端口监听、没有 PID 锁
- ✅ 不需要 `cc-switch start` / `cc-switch stop`
- ✅ 关掉 Claude，环境也跟着结束，无残留
- ✅ 退出后系统看不到任何 cc-switch 痕迹（除了配置文件）
- 🛡️ 启动的 Claude 默认开启 bypass permissions（见上文核心特性中的说明）

## 高级用法

### 交互模式

```bash
# Claude 交互模式
cc-switch

# Codex 交互模式
cc-switch codex

# 导航操作（同时支持箭头键和 Vim 键位）：
# - ↑↓ 或 k/j：上下移动
# - 1-9：直接跳转到对应配置
# - N/PageDown：下一页（>9 个配置时）
# - P/PageUp：上一页
# - R：重置为默认 Claude（仅 Claude 模式）
# - E：编辑配置
# - Q：退出
```

### 快速切换（use 命令）

```bash
# 切换到指定配置并启动 Claude
cc-switch use work

# 切换并发送提示词
cc-switch use work "帮我写一个 Python 脚本"

# 切换并恢复之前的会话
cc-switch use work --resume c8439f36-44a9-4d85-9e88-de35e004fdd4
cc-switch use work -r c8439f36-44a9-4d85-9e88-de35e004fdd4

# 切换并继续最近的会话
cc-switch use work --continue
cc-switch use work -c
```

### 完整配置添加

```bash
# 添加包含所有选项的配置
cc-switch add work -t sk-ant-xxx -u https://api.anthropic.com \
  -m claude-3-5-sonnet-20241022 \
  --small-fast-model claude-3-haiku-20240307 \
  --max-thinking-tokens 8192 \
  --api-timeout-ms 300000 \
  --disable-nonessential-traffic 1 \
  --default-sonnet-model claude-3-5-sonnet-20241022 \
  --default-opus-model claude-3-opus-20240229 \
  --default-haiku-model claude-3-haiku-20240307

# 使用 DeepSeek API
cc-switch add deepseek \
  -t $DEEPSEEK_API_KEY \
  -u https://api.deepseek.com/anthropic \
  -m deepseek-v4-pro[1m] \
  --default-opus-model deepseek-v4-pro \
  --default-sonnet-model deepseek-v4-pro \
  --default-haiku-model deepseek-v4-flash \
  --subagent-model deepseek-v4-pro \
  --disable-nonessential-traffic 1 \
  --disable-nonstreaming-fallback 1 \
  --effort-level max

# 强制覆盖添加
cc-switch add work -t sk-ant-xxx -u https://api.anthropic.com -f

# 交互模式添加
cc-switch add work -i

# 从 JSON 文件导入（文件名作为别名）
cc-switch add --from-file config.json
```

### 存储模式

> ⚠️ **多开 Claude 实例时务必使用 `env` 模式（默认值）。**
>
> - **`env` 模式（默认，推荐）**：写入 `settings.json` 的 `env` 字段。Claude 在启动时把这些值读入进程环境变量，之后**不再监听文件变化**。多开多窗口、多会话各自独立，互不影响。
> - **`config` 模式**：写入 `settings.json` 的根级配置字段（camelCase）。Claude 在运行时会**热读取**最新值——这意味着你切换一次配置，**所有正在运行的 Claude 实例都会被改成新配置**。除非你确实只开一个窗口并希望切换立即生效，否则不要用。

```bash
cc-switch --store env    # 写入到 env 字段（默认，多开安全）
cc-switch --store config # 写入到根级别 camelCase（会影响正在运行的实例）
```

### 列出配置

```bash
cc-switch list           # JSON 格式（默认）
cc-switch list -p        # 纯文本格式
```

### 移除多个配置

```bash
cc-switch remove work
cc-switch remove work personal test-config
```

### 配置迁移

```bash
# 从旧路径迁移（~/.cc_auto_switch/）到新路径
cc-switch --migrate
```

## StatusLine 集成

cc-switch 可以在 Claude Code 的状态栏左侧**实时显示当前配置别名**，方便你随时确认正在使用哪套 API。

```bash
# 安装（首次或升级后运行）
cc-switch statusline install

# 卸载
cc-switch statusline uninstall
```

工作方式：

- cc-switch 生成一个 shell 包装脚本（`~/.claude/cc_auto_switch_statusline.sh`）
- 自动检测 [`ccstatusline`](https://www.npmjs.com/package/ccstatusline)（优先 `bunx`，回退到 `npx`），并把它作为底层 statusLine 命令
- 状态栏前缀会显示 `[别名]`，例如 `[work] /Users/you/project | claude-sonnet-4-6 | $0.12`
- 如果你的 `settings.json` 里已有 statusLine 命令，会被**包装**而非覆盖
- 卸载时自动还原为原始命令

依赖：系统需安装 `bun` 或 `npm`（用来运行 ccstatusline）。

## Shell 集成

### 生成补全脚本

> **升级后请重新生成补全脚本**，以获取新子命令（如 `codex`、`statusline`）的补全支持。

#### Fish / Zsh / Bash

```bash
# Fish（推荐，唯一支持动态别名补全）
cc-switch completion fish > ~/.config/fish/completions/cc-switch.fish

# Zsh
cc-switch completion zsh > ~/.zsh/completions/_cc-switch
echo 'fpath=(~/.zsh/completions $fpath)' >> ~/.zshrc

# Bash（含 Windows 上的 Git Bash）
cc-switch completion bash > ~/.bash_completion.d/cc-switch
echo 'source ~/.bash_completion.d/cc-switch' >> ~/.bashrc

# Elvish 也受支持
cc-switch completion elvish
```

#### PowerShell（Windows）

**不要**直接将补全脚本重定向到 `$PROFILE`——这会覆盖已有的别名、模块或主题配置。请写入独立文件后再从 `$PROFILE` 中 dot-source：

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

该脚本是幂等的，可以反复执行。

#### CMD（Windows）

CMD 没有补全机制，直接使用命令即可。

### 补全支持矩阵

**Fish / Zsh / Bash / PowerShell / Elvish 全部支持完整的子命令、参数、标志补全。** 此外，所有 shell 都可启用"动态别名补全"——按 `<Tab>` 实时列出当前所有配置名（Fish 开箱即用；其他 shell 复制下文片段即可）。

| Shell | 静态补全（命令 / 参数 / 标志） | 动态别名补全（`use <Tab>` 列出配置） |
|-------|------------------------------|--------------------------------------|
| **Fish** | ✅ 自动 | ✅ **自动**（Claude + Codex 双模式） |
| **Zsh** | ✅ 自动 | ⚙️ 需手动添加片段（[见下](#zsh-动态别名补全)） |
| **Bash** | ✅ 自动 | ⚙️ 需手动添加片段（[见下](#bash-动态别名补全)） |
| **PowerShell** | ✅ 自动 | ⚙️ 需手动添加片段（[见下](#powershell-动态别名补全)） |
| **Elvish** | ✅ 自动 | ⚙️ 可用 `edit:completion:arg-completer` 自行实现 |

> 工作原理：`cc-switch --list-aliases` 和 `cc-switch --list-codex-aliases` 这两个标志会输出当前所有配置名，**任何 shell 都可以调用**。Fish 生成的脚本已经把它们接入补全；其他 shell 只需追加几行片段即可。

### 在其他 shell 中启用动态别名补全

> 以下片段都是**追加**在 `cc-switch completion <shell>` 生成的脚本之后，**不会破坏**静态补全。

#### Zsh 动态别名补全

把下面内容加到 `~/.zshrc`（**必须在补全脚本被 source、`compinit` 完成之后**）：

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
# 在 clap 生成的 _cc-switch 之前优先匹配
compdef _cc_switch_dynamic_aliases cc-switch
```

#### Bash 动态别名补全

把下面内容加到 `~/.bashrc`（**必须在 `source ~/.bash_completion.d/cc-switch` 之后**）：

```bash
_cc_switch_with_aliases() {
  # 先让 clap 生成的补全跑一遍（处理子命令、flag 等）
  _cc-switch "$@"

  # 然后在别名位置覆盖 COMPREPLY
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

#### PowerShell 动态别名补全

PowerShell 的 `Register-ArgumentCompleter` 可以**和现有补全共存**，无需替换。加到你的 PowerShell `$PROFILE`（或前述 `cc-switch.completion.ps1` 末尾）：

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

### 内置别名（推荐添加）

`cc-switch completion <shell>` 在生成补全的同时会输出推荐的 shell 别名定义。也可以手动添加：

| 别名 | 等价命令 | 用途 |
|------|----------|------|
| `cs` | `cc-switch` | 主命令的短别名（输入 `cs` 即进入交互模式） |
| `ccd` | `claude --dangerously-skip-permissions` | 跳过权限确认直接启动 Claude |
| `cx` | `cc-switch codex` | Codex 子命令的短别名 |

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

添加后即可：

```bash
cs              # = cc-switch（进入交互模式）
cs use work     # = cc-switch use work
ccd             # 快速启动 Claude（跳过权限确认）
cx              # = cc-switch codex（进入 Codex 交互模式）
cx use work     # = cc-switch codex use work
```

> 💡 **Fish 用户提示**：使用 `cs` 别名时，按 `Tab` 同样能享受动态补全——Fish 会把别名解开到原命令进行补全。

## 导入 / 导出

### Claude 配置从 JSON 导入

```bash
# 文件名（不含扩展名）成为别名
cc-switch add --from-file my-work-config.json

# 期望的 JSON 格式：
# {
#   "env": {
#     "ANTHROPIC_AUTH_TOKEN": "sk-ant-xxx",
#     "ANTHROPIC_BASE_URL": "https://api.anthropic.com",
#     "ANTHROPIC_MODEL": "claude-3-5-sonnet-20241022"
#   }
# }
```

### Codex 配置从 auth.json 导入

```bash
cc-switch codex add work --from-file ~/.codex/auth.json
```

完整文档：[docs/codex.md](docs/codex.md)。

## 工作原理

cc-switch 将配置存储在 `~/.claude/cc_auto_switch_setting.json` 中：

- **Claude 配置**：更新 Claude 的 `settings.json` 文件，设置适当的环境变量
- **Codex 配置**：写入 `~/.codex/auth.json` 文件，Codex CLI 从该文件读取认证信息

这意味着：

- ✅ 不修改全局配置
- ✅ 配置之间完全隔离
- ✅ 安全的 API 密钥管理
- ✅ 适用于任何 Claude / Codex 安装
- ✅ 保留其他设置
- ✅ 支持自定义设置目录

## 环境变量

### Claude 配置

工具在切换配置时设置以下环境变量：

- `ANTHROPIC_AUTH_TOKEN` - 你的 API 令牌
- `ANTHROPIC_BASE_URL` - API 端点 URL
- `ANTHROPIC_MODEL` - 自定义模型（可选）
- `ANTHROPIC_SMALL_FAST_MODEL` - 后台任务快速模型（可选）
- `ANTHROPIC_MAX_THINKING_TOKENS` - 最大思考令牌限制（可选）
- `API_TIMEOUT_MS` - API 超时时间（毫秒）（可选）
- `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` - 禁用非必要流量标志（可选）
- `ANTHROPIC_DEFAULT_SONNET_MODEL` - 默认 Sonnet 模型（可选）
- `ANTHROPIC_DEFAULT_OPUS_MODEL` - 默认 Opus 模型（可选）
- `ANTHROPIC_DEFAULT_HAIKU_MODEL` - 默认 Haiku 模型（可选）
- `CLAUDE_CODE_SUBAGENT_MODEL` - 子代理模型（可选）
- `CLAUDE_CODE_DISABLE_NONSTREAMING_FALLBACK` - 禁用非流式回退标志（可选）
- `CLAUDE_CODE_EFFORT_LEVEL` - 努力级别（可选，如 'max'）
- `CC_SWITCH_CURRENT_ALIAS` - 当前别名（由 cc-switch 自动注入，供 statusLine 读取）

### Codex 配置

Codex 配置存储在 `~/.codex/auth.json`，支持两种认证模式：

**chatgpt 模式（OAuth）**：
- `id_token`、`access_token`、`refresh_token`、`account_id`

**apikey 模式**：
- `OPENAI_API_KEY`

完整文档：[docs/codex.md](docs/codex.md)。

## 开发

```bash
# 克隆
git clone https://github.com/Linuxdazhao/cc_auto_switch.git
cd cc-switch

# 构建
cargo build --release

# 测试
cargo test
```

## 许可证

MIT 许可证 - 详见 [LICENSE](LICENSE) 文件。

---

**由 [Linuxdazhao](https://github.com/Linuxdazhao) 用 ❤️ 制作**
