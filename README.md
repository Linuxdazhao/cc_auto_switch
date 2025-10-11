# cc-switch

[![Crates.io](https://img.shields.io/crates/v/cc-switch.svg)](https://crates.io/crates/cc-switch)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**[English README](README_EN.md) | 中文文档**

一个简单的 CLI 工具，用于管理多个 Claude API 配置。

在不同项目或环境中使用 Claude API 时，经常需要切换 API 令牌和设置。cc-switch 让这个过程变得轻松：

- 用易记的名称存储多个配置
- 在它们之间即时切换
- 自动启动带有正确环境变量的 Claude
- 保持 API 密钥的安全和组织

## 快速开始

```bash
# 安装
cargo install cc-switch

# 添加你的第一个配置
cc-switch add work sk-ant-work-xxx https://api.anthropic.com

# 添加另一个配置
cc-switch add personal sk-ant-personal-xxx https://api.anthropic.com

# 切换到工作配置
cc-switch use work

# 切换到个人配置
cc-switch use personal

# 查看所有配置
cc-switch list
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
| `cc-switch add <名称> <令牌> <URL>` | 添加新配置 |
| `cc-switch use <名称>` | 切换到配置 |
| `cc-switch list` | 显示所有配置 |
| `cc-switch remove <名称>` | 删除配置 |
| `cc-switch current` | 交互菜单 |

## 高级用法

### 交互模式
```bash
# 进入交互模式（无需参数）
cc-switch

# 或通过 current 命令访问
cc-switch current
```

### 添加模型配置
```bash
# 添加自定义模型配置
cc-switch add work -t sk-ant-xxx -u https://api.anthropic.com -m claude-3-5-sonnet-20241022

# 添加后台任务用的快速模型
cc-switch add work --small-fast-model claude-3-haiku-20240307
```

### Shell 集成
```bash
# 生成 Shell 补全
cc-switch completion fish > ~/.config/fish/completions/cc-switch.fish

# 创建便捷别名
eval "$(cc-switch alias fish)"
# 现在可以使用：
cs use work     # 代替 cc-switch use work
ccd             # 快速启动 Claude
```

## 工作原理

cc-switch 将配置存储在 `~/.cc-switch/configurations.json` 中，并启动带有适当环境变量的 Claude。这意味着：

- ✅ 不修改全局配置
- ✅ 配置之间完全隔离
- ✅ 安全的 API 密钥管理
- ✅ 适用于任何 Claude 安装

## 环境变量

工具在启动 Claude 时设置这些环境变量：

- `ANTHROPIC_AUTH_TOKEN` - 你的 API 令牌
- `ANTHROPIC_BASE_URL` - API 端点 URL
- `ANTHROPIC_MODEL` - 自定义模型（可选）
- `ANTHROPIC_SMALL_FAST_MODEL` - 后台任务快速模型（可选）

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