# cc-switch

[![Crates.io](https://img.shields.io/crates/v/cc-switch.svg)](https://crates.io/crates/cc-switch)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**[English README](README_EN.md) | 中文文档**

一个 CLI 工具，用于管理多个 Claude API 配置并在它们之间自动切换。

在不同项目或环境中使用 Claude API 时，经常需要切换 API 令牌和设置。cc-switch 让这个过程变得轻松：

- 用易记的名称存储多个配置
- 在它们之间即时切换
- 自动启动带有正确环境变量的 Claude
- 保持 API 密钥的安全和组织
- 从 JSON 文件导入/导出配置
- 完整的交互模式，支持键盘导航

## 快速开始

```bash
# 安装
cargo install cc-switch

# 添加你的第一个配置
cc-switch add work sk-ant-work-xxx https://api.anthropic.com

# 添加另一个配置
cc-switch add personal sk-ant-personal-xxx https://api.anthropic.com

# 切换到工作配置
cc-switch
# 然后从交互菜单中选择 'work'

# 切换到个人配置
cc-switch
# 然后从交互菜单中选择 'personal'

# 查看所有配置
cc-switch list

# 进入交互模式（同上）
cc-switch
```

## 安装

### Cargo（推荐）
```bash
cargo install cc-switch
```

### Homebrew
```bash
brew tap Linuxdazhao/cc-switch
brew install cc-switch
```

## 主要命令

| 命令 | 作用 |
|------|------|
| `cc-switch add <名称>` | 添加新配置 |
| `cc-switch list` | 显示所有配置（JSON 或纯文本） |
| `cc-switch remove <名称...>` | 删除一个或多个配置 |
| `cc-switch completion <shell>` | 生成 Shell 补全脚本 |
| `cc-switch` | 进入交互模式 |

## 高级用法

### 交互模式
```bash
# 进入交互模式（无需参数）
cc-switch

# 导航操作：
# - ↑↓ 箭头或 1-9 键：选择配置
# - N/P：下一页/上一页（当配置 >9 个时）
# - R：重置为默认 Claude
# - E：退出
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

## Shell 集成

### 生成补全
```bash
# Fish（推荐）
cc-switch completion fish > ~/.config/fish/completions/cc-switch.fish

# Zsh
cc-switch completion zsh > ~/.zsh/completions/_cc-switch
echo 'fpath=(~/.zsh/completions $fpath)' >> ~/.zshrc

# Bash
cc-switch completion bash > ~/.bash_completion.d/cc-switch
echo 'source ~/.bash_completion.d/cc-switch' >> ~/.bashrc

# Elvish 或 PowerShell 也受支持
cc-switch completion elvish
cc-switch completion powershell
```

### 创建别名
```bash
# 将别名永久添加到 shell 配置

# Fish
echo "alias cs='cc-switch'" >> ~/.config/fish/config.fish
echo "alias ccd='claude --dangerously-skip-permissions'" >> ~/.config/fish/config.fish

# Zsh
echo "alias cs='cc-switch'" >> ~/.zshrc
echo "alias ccd='claude --dangerously-skip-permissions'" >> ~/.zshrc

# Bash
echo "alias cs='cc-switch'" >> ~/.bashrc
echo "alias ccd='claude --dangerously-skip-permissions'" >> ~/.bashrc

# 现在可以使用：
cs              # 代替 cc-switch（进入交互模式）
ccd             # 快速启动 Claude
```

## 工作原理

cc-switch 将配置存储在 `~/.cc-switch/configurations.json` 中，并更新 Claude 的 `settings.json` 文件，设置适当的环境变量。这意味着：

- ✅ 不修改全局配置
- ✅ 配置之间完全隔离
- ✅ 安全的 API 密钥管理
- ✅ 适用于任何 Claude 安装
- ✅ 保留其他 Claude 设置
- ✅ 支持自定义 Claude 设置目录

## 环境变量

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

## 导入/导出

### 从 JSON 导入
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