# cc-switch

[![Crates.io](https://img.shields.io/crates/v/cc-switch.svg)](https://crates.io/crates/cc-switch)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**一个 CLI 工具，用于管理多个 Claude / Codex 配置并在它们之间自动切换。**

在不同项目或环境中使用 Claude API 或 OpenAI Codex CLI 时，经常需要切换 API 令牌和设置。cc-switch 让这个过程变得轻松：

- 用易记的名称存储多个配置
- 在它们之间即时切换
- 自动启动带有正确环境变量的 Claude / Codex
- 保持 API 密钥的安全和组织
- 从 JSON 文件导入/导出配置
- 完整的交互模式，支持键盘导航

## 快速开始

```bash
# 安装
cargo install cc-switch

# ===== Claude 配置 =====

# 添加你的第一个配置
cc-switch add work sk-ant-work-xxx https://api.anthropic.com

# 切换到工作配置
cc-switch
# 然后从交互菜单中选择 'work'

# ===== Codex 配置 =====

# 从现有 auth.json 导入
cc-switch codex add work --from-file ~/.codex/auth.json

# 切换到工作配置并启动 Codex
cc-switch codex use work

# 进入 Codex 交互模式
cc-switch codex
# 然后从交互菜单中选择 'work'

# 查看所有配置
cc-switch list        # Claude 配置
cc-switch codex list  # Codex 配置
```

## 安装

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

**方式 3 — 预编译二进制：** 从 [Releases](https://github.com/Linuxdazhao/cc_auto_switch/releases) 下载对应架构的 `.zip`，将 `cc-switch.exe` 放到 PATH 中。

### Linux

**方式 1 — Homebrew（推荐）：**

```bash
# 简短形式可用，因 tap 仓库名为 `homebrew-cc-switch`
brew install Linuxdazhao/cc-switch/cc-switch
# 或显式：
# brew tap Linuxdazhao/cc-switch && brew install cc-switch
```

**方式 2 — Cargo：**

```bash
cargo install cc-switch
```

**方式 3 — 预编译二进制：** 从 [Releases](https://github.com/Linuxdazhao/cc_auto_switch/releases) 下载对应架构的 `.tar.gz`。

### macOS

**方式 1 — Homebrew（推荐）：**

```bash
brew install Linuxdazhao/cc-switch/cc-switch
```

**方式 2 — Cargo：**

```bash
cargo install cc-switch
```

**方式 3 — 预编译二进制：** 从 [Releases](https://github.com/Linuxdazhao/cc_auto_switch/releases) 下载对应架构的 `.tar.gz`。

## 主要命令

### Claude 配置管理

| 命令 | 作用 |
|------|------|
| `cc-switch add <名称>` | 添加新配置 |
| `cc-switch list` | 显示所有配置（JSON 或纯文本） |
| `cc-switch remove <名称...>` | 删除一个或多个配置 |
| `cc-switch use <名称>` | 快速切换配置并启动 Claude |
| `cc-switch` | 进入交互模式 |

### Codex 配置管理

| 命令 | 作用 |
|------|------|
| `cc-switch codex add <名称>` | 添加新配置 |
| `cc-switch codex list` | 显示所有配置 |
| `cc-switch codex remove <名称...>` | 删除配置 |
| `cc-switch codex use <名称>` | 切换配置并启动 Codex |
| `cc-switch codex` | 进入交互模式 |

详细文档请查看 [Codex 配置管理](docs/codex.md)。

### 通用命令

| 命令 | 作用 |
|------|------|
| `cc-switch completion <shell>` | 生成 Shell 补全脚本 |

## 高级用法

### 交互模式
```bash
# Claude 交互模式
cc-switch

# Codex 交互模式
cc-switch codex

# 导航操作：
# - ↑↓ 箭头或 1-9 键：选择配置
# - N/P：下一页/上一页（当配置 >9 个时）
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
```bash
# 设置默认存储模式
cc-switch --store env    # 写入到 env 字段（默认）
cc-switch --store config # 写入到根级别，使用 camelCase
```

### 列出配置
```bash
# 以 JSON 格式列出（默认）
cc-switch list

# 以纯文本格式列出
cc-switch list -p
```

### 移除多个配置
```bash
# 移除一个配置
cc-switch remove work

# 一次性移除多个配置
cc-switch remove work personal test-config
```

### 配置迁移
```bash
# 从旧路径迁移（~/.cc_auto_switch/）到新路径
cc-switch --migrate
```

## 导入/导出

### Claude 配置从 JSON 导入
```bash
# 从 JSON 文件导入配置
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
# 从现有 auth.json 导入
cc-switch codex add work --from-file ~/.codex/auth.json
```

详细文档请查看 [Codex 配置管理](docs/codex.md)。

## Shell 集成

### 生成补全

> **升级后请重新生成补全脚本**，以获取新子命令（如 `codex`）的补全支持。

#### Fish / Zsh / Bash

```bash
# Fish（推荐，支持动态别名补全）
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

CMD 没有补全机制，直接使用命令即可：

```cmd
cc-switch add my-config -t sk-ant-xxx -u https://api.anthropic.com
cc-switch list
```

### 创建别名
```bash
# 将别名永久添加到 shell 配置

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

# 现在可以使用：
cs              # 代替 cc-switch（进入交互模式）
ccd             # 快速启动 Claude
cx              # 进入 Codex 交互模式
```

## 工作原理

cc-switch 将配置存储在 `~/.claude/cc_auto_switch_setting.json` 中：

**Claude 配置**：更新 Claude 的 `settings.json` 文件，设置适当的环境变量。

**Codex 配置**：写入 `~/.codex/auth.json` 文件，Codex CLI 从该文件读取认证信息。

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

### Codex 配置

Codex 配置存储在 `~/.codex/auth.json`，支持两种认证模式：

**chatgpt 模式（OAuth）**：
- `id_token` - 身份令牌
- `access_token` - 访问令牌
- `refresh_token` - 刷新令牌
- `account_id` - 账户 ID

**apikey 模式**：
- `OPENAI_API_KEY` - API 密钥

详细文档请查看 [Codex 配置管理](docs/codex.md)。

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