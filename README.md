# cc-switch

**[English README](README_EN.md) | 中文文档**

[![Crates.io](https://img.shields.io/crates/v/cc-switch.svg)](https://crates.io/crates/cc-switch)
[![GitHub Packages](https://img.shields.io/badge/GitHub-Packages-green)](https://github.com/Linuxdazhao/cc_auto_switch/packages)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Build Status](https://github.com/Linuxdazhao/cc_auto_switch/workflows/CI/badge.svg)](https://github.com/Linuxdazhao/cc_auto_switch/actions)
[![Release](https://github.com/Linuxdazhao/cc_auto_switch/workflows/Release/badge.svg)](https://github.com/Linuxdazhao/cc_auto_switch/releases)
[![codecov](https://codecov.io/gh/Linuxdazhao/cc_auto_switch/branch/main/graph/badge.svg)](https://codecov.io/gh/Linuxdazhao/cc_auto_switch)

一个强大的命令行工具，用于管理多个 Claude API 配置并通过环境变量在它们之间轻松切换。

如果您曾经在不同环境（开发、测试、生产，或者不同客户账户）中使用 Claude API，您一定深刻理解手动设置环境变量或管理不同配置的痛苦。cc-switch 通过提供集中化环境变量管理解决方案消除了这种痛苦：

* **存储多个 API 配置**，使用易于记忆的别名
* **一键切换配置**，通过环境变量安全启动 Claude
* **为不同项目或环境维护独立设置**
* **环境变量隔离**，每次执行使用独立的环境配置

## 🏗️ 核心架构

该工具采用清晰的模块化架构，有效分离关注点：

应用程序遵循简单而强大的设计模式，主入口点将任务委托给处理所有 CLI 操作的命令模块。`ConfigStorage` 管理配置的持久化，而 `EnvironmentConfig` 处理与环境变量的集成，通过为每个命令执行设置特定的环境变量来确保配置隔离。

## 🎯 核心功能

cc-switch 功能丰富，让 API 配置管理变得轻松：

| 功能 | 描述 | 优势 |
|------|------|------|
| **多配置管理** | 使用自定义别名存储无限数量的 API 配置 | 保持所有环境井然有序 |
| **环境变量切换** | 使用 `cc-switch use <别名>` 通过环境变量启动 Claude | 安全隔离，不影响全局设置 |
| **交互式选择模式** | 带实时配置预览的可视化菜单，支持数字键快速选择 | 切换前浏览配置的完整详情，按数字键1-9直接选择 |
| **Shell 自动补全** | 内置对 fish、zsh、bash 等的补全支持 | 加速命令输入和自动补全 |
| **动态别名补全** | 为 use/remove 命令自动补全配置名称 | 减少错误和输入工作量 |
| **Shell 别名生成** | 生成兼容 eval 的别名以快速访问 | 通过便捷快捷方式简化工作流 |
| **安全存储** | 配置安全存储在 `~/.cc-switch/` 目录 | 您的 API 密钥保持独立和有序 |
| **跨平台支持** | 支持 Linux 和 macOS | 在所有主要开发环境中使用同一工具 |
| **环境变量隔离** | 每次执行使用独立环境变量 | 避免全局配置冲突 |

## ⚡ 3分钟快速开始

cc-switch 的美妙之处在于其简洁性。以下是快速启动和运行的步骤：

1. **安装工具**（约30秒）：
   ```bash
   # 使用 Homebrew（推荐）
   brew tap Linuxdazhao/cc-switch && brew install cc-switch
   
   # 或使用 Cargo
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
   这将使用您的配置启动 Claude CLI，而不是修改任何全局设置。

4. **验证是否工作**（约10秒）：
   ```bash
   cc-switch current
   ```

**重要变化：** `cc-switch use` 命令现在通过环境变量启动 Claude，而不是修改全局配置文件。这确保了完全的配置隔离和安全性。

**提示：** 直接运行 `cc-switch`（不带任何参数）会进入交互式主菜单模式，让您可以快速访问所有功能！

## 🔒 环境变量模式

cc-switch 现在使用环境变量模式，提供更好的安全性和隔离：

- ✅ **隔离性**：每次执行使用独立的环境变量，不影响系统全局设置
- ✅ **安全性**：API 密钥不会写入任何配置文件，只在命令执行时使用
- ✅ **简洁性**：无需管理复杂的配置文件或担心配置冲突
- ✅ **便捷性**：`cc-switch use <alias>` 自动设置环境变量并启动 Claude

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

# 根据需要在环境间切换（每次都启动新的 Claude 实例）
cc-switch use dev      # 开发工作
cc-switch use staging  # 测试  
cc-switch use prod     # 生产部署
cc-switch use cc       # 使用默认设置启动
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

该工具采用**零配置**理念设计 - 开箱即用具有合理默认值，专注于环境变量模式的简洁性和安全性。

## 🌐 平台支持

cc-switch 现在专注于主要的开发平台：

- ✅ **Linux** (x86_64, aarch64)
- ✅ **macOS** (Intel, Apple Silicon)
- ❌ **Windows** (已移除支持，专注于 Unix-like 系统)

## 🚀 安装

### 使用 Homebrew（推荐）

最简单的安装方式是使用 Homebrew：

```bash
# 添加 tap
brew tap Linuxdazhao/cc-switch

# 安装 cc-switch
brew install cc-switch
```

更新：
```bash
brew update && brew upgrade cc-switch
```

支持平台：
- ✅ macOS Intel (x86_64)
- ✅ macOS Apple Silicon (ARM64/M1/M2)  
- ✅ Linux x86_64
- ✅ Linux ARM64

### 从 Crates.io

如果您有 Rust 开发环境：

```bash
cargo install cc-switch
```

### 二进制包下载

从 GitHub Releases 下载预编译的二进制包：

```bash
# 下载适合您平台的包
wget https://github.com/Linuxdazhao/cc_auto_switch/releases/latest/download/cc-switch-x86_64-apple-darwin.tar.gz

# 解压并安装
tar -xzf cc-switch-x86_64-apple-darwin.tar.gz
cp cc-switch ~/.local/bin/
```

### 从源代码

```bash
git clone https://github.com/Linuxdazhao/cc_auto_switch.git
cd cc-switch
cargo build --release
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
```

#### 切换配置

```bash
# 使用特定配置启动 Claude
cc-switch use my-config

# 使用默认设置启动 Claude（无自定义 API 配置）
cc-switch use cc
```

**注意：** `use` 命令现在通过环境变量启动 Claude CLI，确保配置隔离和安全性。

#### 当前配置交互菜单

```bash
# 显示环境变量模式信息和交互菜单
cc-switch current

# 或直接运行（无参数时进入交互式主菜单）
cc-switch
```

`current` 命令提供交互菜单，包含：
- 显示环境变量模式的信息说明
- 选项 1：执行 `claude --dangerously-skip-permissions`
- 选项 2：切换配置（带实时预览并启动 Claude）
- 选项 3：退出

导航：
- 使用 **↑↓** 箭头键进行菜单导航
- **数字键快速选择**：按 **1-9** 直接选择对应配置项，无需箭头键导航
- **智能分页**：配置超过 9 个时自动分页显示，每页最多 9 个配置
- **页面导航**：**PageUp/PageDown** 或 **N/P** 键快速翻页
- **快捷操作**：**R** 键快速重置为官方配置，**E** 键直接退出
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
- **数字键1-9** 直接选择对应配置项，提升选择效率
- **智能分页系统**：配置超过9个时自动分页，支持 **N/P** 键或 **PageUp/PageDown** 翻页
- 查看所选配置的详细信息（令牌、URL、模型、小型快速模型）
- 按 **Enter** 选择并使用该配置启动 Claude
- 按 **R** 键快速重置为官方配置（在任何页面都可用）
- 按 **E** 键直接退出程序
- 按 **Esc** 取消选择
- 包括"使用默认设置"选项以无自定义配置启动 Claude
- 如果终端不支持高级功能，智能回退到编号菜单

交互模式提供可视化方式浏览和选择配置，选择后使用指定配置的环境变量自动启动 Claude CLI。

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

项目包含支持 Linux 和 macOS 跨平台编译的全面构建流程：

- **CI 管道**：自动在 Ubuntu 和 macOS 上测试
- **跨架构支持**：支持 x86_64 和 aarch64 架构
- **发布自动化**：自动构建和发布多个目标平台的二进制文件

这确保 cc-switch 可以在所有支持的平台上分发并保持一致的行为。

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

- 🐛 **错误报告**：[GitHub Issues](https://github.com/Linuxdazhao/cc_auto_switch/issues)
- 💡 **功能请求**：[GitHub Issues](https://github.com/Linuxdazhao/cc_auto_switch/issues)
- 📧 **问题**：[GitHub Discussions](https://github.com/Linuxdazhao/cc_auto_switch/discussions)

---
**由 [Linuxdazhao](https://github.com/Linuxdazhao) 用 ❤️ 制作**