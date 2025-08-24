# cc-switch

**[English README](README_EN.md) | 中文文档**

[![Crates.io](https://img.shields.io/crates/v/cc-switch.svg)](https://crates.io/crates/cc-switch)
[![GitHub Packages](https://img.shields.io/badge/GitHub-Packages-green)](https://github.com/jingzhao/cc_auto_switch/packages)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Build Status](https://github.com/jingzhao/cc_auto_switch/workflows/CI/badge.svg)](https://github.com/jingzhao/cc_auto_switch/actions)
[![Release](https://github.com/jingzhao/cc_auto_switch/workflows/Release/badge.svg)](https://github.com/jingzhao/cc_auto_switch/releases)

一个强大的命令行工具，用于管理多个 Claude API 配置并在它们之间轻松切换。

如果您曾经在不同环境（开发、测试、生产，或者不同客户账户）中使用 Claude API，您一定深刻理解手动编辑配置文件或设置环境变量的痛苦。cc-switch 通过提供集中化解决方案消除了这种痛苦：

* **存储多个 API 配置**，使用易于记忆的别名
* **一键切换配置**，单个命令即可完成
* **为不同项目或环境维护独立设置**
* **保留现有 Claude 设置**，仅修改 API 相关配置

## 🏗️ 核心架构

该工具采用清晰的模块化架构，有效分离关注点：

应用程序遵循简单而强大的设计模式，主入口点将任务委托给处理所有 CLI 操作的命令模块。`ConfigStorage` 管理配置的持久化，而 `ClaudeSettings` 处理与 Claude 原生配置系统的集成。

## 🎯 核心功能

cc-switch 功能丰富，让 API 配置管理变得轻松：

| 功能 | 描述 | 优势 |
|------|------|------|
| **多配置管理** | 使用自定义别名存储无限数量的 API 配置 | 保持所有环境井然有序 |
| **即时切换** | 使用 `cc-switch use <别名>` 切换配置 | 节省手动配置更改的时间 |
| **交互式选择模式** | 带实时配置预览的可视化菜单 | 切换前浏览配置的完整详情 |
| **Shell 自动补全** | 内置对 fish、zsh、bash 等的补全支持 | 加速命令输入和自动补全 |
| **动态别名补全** | 为 use/remove 命令自动补全配置名称 | 减少错误和输入工作量 |
| **Shell 别名生成** | 生成兼容 eval 的别名以快速访问 | 通过便捷快捷方式简化工作流 |
| **安全存储** | 配置安全存储在 `~/.cc-switch/` 目录 | 您的 API 密钥保持独立和有序 |
| **跨平台支持** | 支持 Linux、macOS 和 Windows | 在所有开发环境中使用同一工具 |
| **自定义目录支持** | 支持自定义 Claude 设置目录 | 为非标准安装提供灵活性 |

## ⚡ 3分钟快速开始

cc-switch 的美妙之处在于其简洁性。以下是快速启动和运行的步骤：

1. **安装工具**（约30秒）：
   ```bash
   cargo install cc-switch
   ```

2. **添加第一个配置**（约15秒）：
   ```bash
   cc-switch add my-project sk-ant-xxx https://api.anthropic.com
   ```

3. **切换到您的配置**（约5秒）：
   ```bash
   cc-switch use my-project
   ```

4. **验证是否工作**（约10秒）：
   ```bash
   cc-switch current
   ```

**提示：** 直接运行 `cc-switch`（不带任何参数）会进入交互式主菜单模式，让您可以快速访问所有功能！

就是这样！您现在像专家一样管理 Claude API 配置了。

## 🐚 Shell 集成

cc-switch 提供强大的 shell 集成功能来简化您的工作流：

### Shell 别名

生成便捷的别名以便更快访问：

```bash
# 为您的 shell 生成别名（fish、zsh、bash）
cc-switch alias fish

# 在当前会话中立即加载别名
eval "$(cc-switch alias fish)"
```

可用别名：
- `cs='cc-switch'` - 快速访问 cc-switch 命令
- `ccd='claude --dangerously-skip-permissions'` - 快速启动 Claude

**使用别名的示例：**
```bash
# 替代：cc-switch use my-config
cs use my-config

# 交互式当前菜单
cs current

# 替代：claude --dangerously-skip-permissions
ccd
```

### Shell 补全

为您的 shell 设置自动补全：

```bash
# Fish shell
cc-switch completion fish > ~/.config/fish/completions/cc-switch.fish

# Zsh shell  
cc-switch completion zsh > ~/.zsh/completions/_cc-switch

# Bash shell
cc-switch completion bash > ~/.bash_completion.d/cc-switch
```

### 永久设置

对于永久别名设置，添加到您的 shell 配置：

**Fish (~/.config/fish/config.fish):**
```bash
alias cs='cc-switch'
alias ccd='claude --dangerously-skip-permissions'
```

**Zsh (~/.zshrc):**
```bash
alias cs='cc-switch'
alias ccd='claude --dangerously-skip-permissions'
```

**Bash (~/.bashrc 或 ~/.bash_profile):**
```bash
alias cs='cc-switch'
alias ccd='claude --dangerously-skip-permissions'
```

## 🌟 实际应用场景

cc-switch 在几个常见开发场景中表现出色：

### 多环境开发

```bash
# 设置不同环境
cc-switch add dev sk-ant-dev-xxx https://api.anthropic.com
cc-switch add staging sk-ant-staging-xxx https://api.anthropic.com
cc-switch add prod sk-ant-prod-xxx https://api.anthropic.com

# 根据需要在环境间切换
cc-switch use dev      # 开发工作
cc-switch use staging  # 测试
cc-switch use prod     # 生产部署
cc-switch use cc       # 重置为默认
```

### 客户项目管理

对于需要不同 API 凭据处理多个客户的开发者：

```bash
cc-switch add client-a sk-ant-client-a https://api.anthropic.com
cc-switch add client-b sk-ant-client-b https://api.anthropic.com
cc-switch add personal sk-ant-personal https://api.anthropic.com
```

### 团队协作

团队成员可以共享配置别名，在团队特定设置间快速切换，无需手动编辑文件。

## 🔧 技术基础

cc-switch 使用现代 Rust 实践构建，并利用几个关键库：

* **crossterm** 用于跨平台终端操作和交互式 UI
* **clap** 用于强大的命令行参数解析和自动生成帮助
* **clap_complete** 用于 shell 补全脚本生成
* **serde** 用于可靠的 JSON 序列化/反序列化
* **dirs** 用于跨平台目录管理
* **anyhow** 用于全面的错误处理
* **colored** 用于终端输出格式化

该工具采用**零配置**理念设计 - 开箱即用具有合理默认值，但在需要时提供自定义选项。

## 🚀 安装

### 从 Crates.io（推荐）

```bash
cargo install cc-switch
```

### 从源代码

```bash
git clone https://github.com/jingzhao/cc_auto_switch.git
cd cc-switch
cargo build --release
```

二进制文件将位于 `target/release/cc-switch`。您可以将其复制到您的 PATH：

```bash
cp target/release/cc-switch ~/.local/bin/
```

## 📖 使用方法

### 基本命令

#### 添加配置

```bash
# 添加新的 Claude API 配置（位置参数）
cc-switch add my-config sk-ant-xxx https://api.anthropic.com

# 使用标志添加（更明确）
cc-switch add my-config -t sk-ant-xxx -u https://api.anthropic.com

# 指定模型添加（设置 ANTHROPIC_MODEL 环境变量）
cc-switch add my-config -t sk-ant-xxx -u https://api.anthropic.com -m claude-3-5-sonnet-20241022

# 为后台任务添加小型快速模型（设置 ANTHROPIC_SMALL_FAST_MODEL 环境变量）
cc-switch add my-config -t sk-ant-xxx -u https://api.anthropic.com --small-fast-model claude-3-haiku-20240307

# 同时添加两个模型（设置两个模型环境变量）
cc-switch add my-config -t sk-ant-xxx -u https://api.anthropic.com -m claude-3-5-sonnet-20241022 --small-fast-model claude-3-haiku-20240307

# 交互模式添加（安全）
cc-switch add my-config -i

# 强制覆写添加
cc-switch add my-config -t sk-ant-xxx -u https://api.anthropic.com --force
```

#### 列出所有配置

```bash
# 列出所有存储的配置
cc-switch list
```

输出：
```
存储的配置：
  my-config: token=sk-ant-xxx, url=https://api.anthropic.com
  work-config: token=sk-ant-work-123, url=https://api.anthropic.com
Claude 设置目录：~/.claude/（默认）
```

#### 切换配置

```bash
# 切换到特定配置
cc-switch use my-config

# 重置为默认（移除 API 配置）
cc-switch use cc
```

#### 当前配置交互菜单

```bash
# 显示当前配置和交互菜单
cc-switch current

# 或直接运行（无参数时进入交互式主菜单）
cc-switch
```

`current` 命令提供交互菜单，包含：
- 显示当前 API 令牌和 URL
- 选项 1：执行 `claude --dangerously-skip-permissions`
- 选项 2：切换配置（带实时预览）
- 选项 3：退出

导航：
- 使用 **↑↓** 箭头键进行菜单导航（或数字键作为回退）
- 按 **Enter** 选择
- 按 **Esc** 退出

#### 交互式选择模式

使用交互式选择以实时预览可视化浏览配置：

```bash
# 通过 current 命令的菜单选项访问
cc-switch current  # 然后选择选项 2

# 直接进入交互式主菜单
cc-switch  # 不带参数

# 直接访问（如果您的版本支持）
cc-switch use  # 未指定别名时为交互模式
```

在交互选择模式中：
- 使用 **↑↓** 箭头键浏览配置
- 查看所选配置的详细信息（令牌、URL、模型、小型快速模型）
- 按 **Enter** 选择并自动启动 Claude
- 按 **Esc** 取消选择
- 包括"重置为默认"选项以移除 API 配置
- 如果终端不支持高级功能，智能回退到编号菜单

交互模式提供可视化方式浏览和选择配置，切换前提供完整详情预览，切换后自动启动 Claude CLI。

#### 移除配置

```bash
# 移除单个配置
cc-switch remove my-config

# 一次移除多个配置
cc-switch remove config1 config2 config3
```

#### 生成 Shell 别名

```bash
# 生成用于 eval 立即使用的别名
cc-switch alias fish

# 为不同 shell 生成别名
cc-switch alias zsh
cc-switch alias bash

# 立即加载别名（推荐）
eval "$(cc-switch alias fish)"
```

#### 生成 Shell 补全

```bash
# 为您的 shell 生成补全脚本
cc-switch completion fish  > ~/.config/fish/completions/cc-switch.fish
cc-switch completion zsh   > ~/.zsh/completions/_cc-switch
cc-switch completion bash  > ~/.bash_completion.d/cc-switch
```

## 🛠️ 开发和构建流程

项目包含支持跨平台编译的全面构建流程，使为多个目标构建变得简单：

这确保 cc-switch 可以在所有主要平台上分发并保持一致的行为。

## 🤝 贡献

我们欢迎贡献！详情请查看我们的[贡献指南](CONTRIBUTING.md)。

### 开发工作流

1. Fork 仓库
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 进行更改
4. 运行测试并确保代码质量 (`cargo test && cargo clippy`)
5. 提交更改 (`git commit -m 'Add amazing feature'`)
6. 推送到分支 (`git push origin feature/amazing-feature`)
7. 打开 Pull Request

## 📄 许可证

本项目采用 MIT 许可证 - 详情请查看 [LICENSE](LICENSE) 文件。

## 🙏 致谢

- [Claude](https://claude.ai/) 提供令人惊叹的 AI 助手
- [Rust](https://www.rust-lang.org/) 编程语言
- [Clap](https://github.com/clap-rs/clap) 用于命令行参数解析
- [Serde](https://github.com/serde-rs/serde) 用于 JSON 序列化

## 📞 支持

- 🐛 **错误报告**：[GitHub Issues](https://github.com/jingzhao/cc_auto_switch/issues)
- 💡 **功能请求**：[GitHub Issues](https://github.com/jingzhao/cc_auto_switch/issues)
- 📧 **问题**：[GitHub Discussions](https://github.com/jingzhao/cc_auto_switch/discussions)

---
**由 [jingzhao](https://github.com/jingzhao) 用 ❤️ 制作**