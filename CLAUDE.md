# CLAUDE.md

本文件为 Claude Code (claude.ai/code) 提供在此代码库中工作的指导。

**重要说明：始终使用简体中文和我对话**

## 项目概述

`cc-switch` 是一个用于管理多个 Claude API 配置并自动切换的 Rust CLI 工具。该工具允许用户存储不同的 API 配置（别名、令牌、URL），并通过修改 Claude 的 settings.json 文件在它们之间切换。这对于需要使用多个 Claude API 端点或需要在不同账户间切换的开发者特别有用。

## 开发命令

### 构建和运行
```bash
# 构建项目
cargo build

# 运行项目
cargo run

# 发布模式构建
cargo build --release

# 发布模式运行
cargo run --release
```

### 测试
```bash
# 运行所有测试
cargo test

# 运行测试并显示输出
cargo test -- --nocapture

# 运行特定测试
cargo test test_name
```

### 代码质量
```bash
# 检查编译错误
cargo check

# 格式化代码
cargo fmt

# 代码检查
cargo clippy

# 带所有警告的代码检查
cargo clippy -- -W warnings

# 运行安全审计
cargo audit
```

### 提交前钩子
```bash
# 设置提交前钩子（一次性设置）
./scripts/setup-pre-commit.sh

# 手动运行提交前钩子
pre-commit run --all-files

# 对特定文件运行提交前钩子
pre-commit run --files src/main.rs

# 更新提交前钩子
pre-commit autoupdate

# 卸载提交前钩子
pre-commit uninstall
```

### 版本管理和发布

项目包含自动化版本管理和发布到 crates.io 的功能：

**完整发布工作流程**：
```bash
# 运行完整发布工作流程（版本递增 + 提交 + 发布）
./scripts/release.sh
```

**手动版本管理**：
```bash
# 手动递增版本
./scripts/increment-version.sh

# 手动发布到 crates.io
./scripts/publish.sh
```

**版本格式**：使用语义版本控制 (x.y.z)，其中：
- 主版本 (x)：破坏性更改
- 次版本 (y)：新功能
- 修补版本 (z)：错误修复和补丁

**自动化工作流程**：
1. 进行代码更改
2. `./scripts/release.sh` - 处理版本递增、提交和发布
3. Cargo.toml 中的版本自动递增
4. 测试自动运行
5. 包自动发布到 crates.io

**手动工作流程**：
1. 进行代码更改
2. `./scripts/increment-version.sh` - 递增版本
3. `git add . && git commit -m "您的消息"`
4. `cargo test` - 运行测试
5. `./scripts/publish.sh` - 发布到 crates.io

### 依赖管理
```bash
# 更新依赖
cargo update

# 检查过时的依赖
cargo outdated

# 添加新依赖
cargo add dependency_name

# 移除依赖
cargo remove dependency_name
```

## 项目结构

```
cc_auto_switch/
├── Cargo.toml              # 项目配置和依赖
├── Cargo.lock              # 依赖锁定文件
├── src/
│   ├── main.rs             # 主应用程序入口点（最小化）
│   └── cmd/
│       ├── main.rs         # 核心 CLI 逻辑和编排
│       ├── mod.rs          # 模块声明
│       ├── cli.rs          # 命令行接口定义
│       ├── types.rs        # 核心数据结构和类型
│       ├── config.rs       # 配置管理逻辑
│       ├── config_storage.rs # 配置持久化和存储
│       ├── interactive.rs  # 交互式菜单和终端 UI
│       ├── completion.rs   # Shell 补全逻辑
│       ├── shell_completion.rs # Shell 特定补全处理器
│       ├── utils.rs        # 工具函数
│       ├── tests.rs        # 核心功能单元测试
│       ├── error_handling_tests.rs  # 错误处理边界情况
│       └── integration_tests.rs      # 集成测试
├── .github/
│   └── workflows/
│       ├── ci.yml          # CI 管道和跨平台构建
│       └── release.yml     # GitHub 发布工作流
└── target/                 # 构建输出目录（git 忽略）
```

## 架构概览

### 核心组件

**配置管理** (`config.rs`, `config_storage.rs`, `types.rs`)：
- `ConfigStorage`：管理多个 API 配置在 `~/.cc-switch/configurations.json` 中的持久化
- `Configuration`：表示单个 API 配置，包括别名、令牌、URL、模型和 small_fast_model
- `ClaudeSettings`：处理 Claude 的 settings.json 文件的环境变量配置
- `AddCommandParams`：用于 add 命令的参数结构，支持交互式模式

**CLI 接口** (`cli.rs`)：
- `Cli`：使用 clap 的主命令解析器，支持子命令和隐藏补全标志
- `Commands`：定义可用子命令的枚举 (add, remove, list, set-default-dir, completion, alias, use, current)
- 丰富的帮助文本，包含示例和 Shell 集成说明

**交互式终端 UI** (`interactive.rs`)：
- `handle_current_command()`：带键盘导航的交互式主菜单
- `handle_interactive_selection()`：实时配置浏览器和预览功能
- **数字键快速选择**：支持按数字键 1-9 直接选择对应配置项
- **智能分页系统**：配置超过 9 个时自动分页，支持 PageUp/PageDown 或 N/P 键翻页
- **快捷键支持**：R 键重置为官方配置，E 键退出
- 基于 Crossterm 的终端处理，支持降级到简单菜单
- 配置切换后自动启动 Claude CLI

**Shell 集成** (`completion.rs`, `shell_completion.rs`)：
- 配置别名的动态补全
- 支持 fish、zsh、bash、elvish、powershell 的 Shell 特定补全处理器
- 支持 eval 兼容输出的别名生成系统

### 关键数据流

1. **配置存储**：使用 JSON 序列化在 `~/.cc-switch/configurations.json` 中存储配置
2. **设置修改**：读取/写入 Claude 的 settings.json 来更新 `ANTHROPIC_AUTH_TOKEN` 和 `ANTHROPIC_BASE_URL`
3. **路径解析**：支持自定义 Claude 设置目录的绝对和相对路径

### CLI Usage Patterns

```bash
# Add configurations (multiple formats supported)
cc-switch add my-config sk-ant-xxx https://api.anthropic.com
cc-switch add my-config -t sk-ant-xxx -u https://api.anthropic.com
cc-switch add my-config -t sk-ant-xxx -u https://api.anthropic.com -m claude-3-5-sonnet-20241022
cc-switch add my-config -t sk-ant-xxx -u https://api.anthropic.com --small-fast-model claude-3-haiku-20240307
cc-switch add my-config -i  # Interactive mode
cc-switch add my-config --force  # Overwrite existing config

# Switch configurations
cc-switch use my-config
cc-switch use -a my-config
cc-switch use --alias my-config
cc-switch use  # Interactive mode

# Interactive current configuration menu
cc-switch current  # Shows current config + interactive menu

# Reset to default (remove API config)
cc-switch use cc

# List all configurations
cc-switch list

# Manage multiple configurations
cc-switch remove config1 config2 config3

# Set custom Claude settings directory
cc-switch set-default-dir /path/to/claude/config

# Shell integration
cc-switch completion fish  # Generate completion scripts
cc-switch alias fish       # Generate eval-compatible aliases
```

## Shell Completion Setup

### Fish Shell
```bash
# Generate completion script
cargo run -- completion fish > ~/.config/fish/completions/cc-switch.fish

# Restart fish or reload completions
source ~/.config/fish/config.fish
```

### Zsh Shell
```bash
# Create completions directory if it doesn't exist
mkdir -p ~/.zsh/completions

# Generate completion script
cargo run -- completion zsh > ~/.zsh/completions/_cc-switch

# Add to ~/.zshrc if not already present
echo 'fpath=(~/.zsh/completions $fpath)' >> ~/.zshrc

# Reload shell configuration
source ~/.zshrc

# Force rebuild completion cache
compinit
```

### Bash Shell
```bash
# Generate completion script
cargo run -- completion bash > ~/.bash_completion.d/cc-switch

# Add to ~/.bashrc if not already present
echo 'source ~/.bash_completion.d/cc-switch' >> ~/.bashrc

# Reload shell configuration
source ~/.bashrc
```

## Interactive Features

### Current Command Interactive Menu
The `cc-switch current` command provides a sophisticated interactive menu with:
- **Current Configuration Display**: Shows active API token and URL
- **Keyboard Navigation**: Arrow keys for menu navigation (with fallback to numbered menu)
- **数字键快速选择**: 按数字键 1-9 直接选择对应配置项，无需箭头键导航
- **智能分页**: 配置超过 9 个时自动分页显示，每页最多 9 个配置
- **页面导航**: PageUp/PageDown 或 N/P 键快速翻页
- **快捷操作**: R 键快速重置为官方配置，E 键直接退出
- **Real-time Selection**: Instant preview of configuration details during browsing
- **Automatic Claude Launch**: Seamlessly launches Claude CLI after configuration switches
- **Terminal Compatibility**: Crossterm-based terminal handling with graceful fallbacks

### Interactive Selection Mode
- **Visual Configuration Browser**: Browse all stored configurations with full details
- **Configuration Preview**: See token, URL, model settings before switching
- **Reset Option**: Quick reset to default Claude behavior
- **Smart Fallbacks**: Automatic fallback to simple menus when terminal capabilities are limited

### 键盘快捷键参考

#### 单页模式（≤9个配置）
- **↑↓**: 上下导航选择
- **1-9**: 数字键直接选择对应配置
- **R**: 重置为官方配置
- **E**: 退出程序
- **Enter**: 确认当前选择
- **Esc**: 取消操作

#### 分页模式（>9个配置）
- **↑↓**: 上下导航选择
- **1-9**: 数字键直接选择当前页对应配置
- **N/PageDown**: 下一页
- **P/PageUp**: 上一页
- **R**: 重置为官方配置（在任何页面都可用）
- **E**: 退出程序
- **Enter**: 确认当前选择

## Completion Features

The shell completion provides:
- **Command completion**: `cc-switch <TAB>` shows all subcommands
- **Subcommand completion**: `cc-switch completion <TAB>` shows available shells
- **Configuration alias completion**: `cc-switch use <TAB>` shows stored configuration names
- **Option completion**: `cc-switch -<TAB>` shows available options
- **Help completion**: Context-aware help for all commands and options
- **Dynamic alias loading**: Completion system dynamically loads available configuration names

## Pre-commit Hooks

The project includes pre-commit hooks that run automatically before each commit to ensure code quality:

### Required Checks (Run on every commit)
- **cargo check**: Verifies code compilation
- **cargo fmt --check**: Ensures code formatting compliance
- **cargo clippy -- -D warnings**: Runs linting with warnings as errors
- **cargo test**: Executes all tests
- **cargo audit**: Security vulnerability scanning
- **cargo doc --no-deps**: Validates documentation builds

### Setup Instructions
```bash
# One-time setup
./scripts/setup-pre-commit.sh

# Manual testing
pre-commit run --all-files

# Skip hooks (if needed)
git commit --no-verify
```

### Development Environment

- **Rust Version**: 1.88.0 or later
- **Rust Edition**: 2024 (using nightly-2024-12-01 toolchain in CI)
- **Cargo Version**: 1.88.0 or later
- **Dependencies**: anyhow (error handling), clap (CLI parsing with completion), clap_complete (shell completion), serde (JSON), dirs (directory paths), colored (terminal output), crossterm (terminal UI), tempfile (testing)
- **Pre-commit**: Python-based pre-commit framework (auto-installed)

## CI/CD Pipeline

### CI Workflow (.github/workflows/ci.yml)
- **Multi-platform testing**: Ubuntu, Windows, macOS
- **Cross-compilation**: Builds for x86_64 and aarch64 architectures
- **Code quality**: Formatting checks, clippy linting, security audit
- **Coverage**: Code coverage reporting with codecov

### Release Workflow (.github/workflows/release.yml)
- **Multi-architecture releases**: Linux, Windows, macOS (Intel and ARM)
- **Automated packaging**: Creates tar.gz artifacts for each target
- **GitHub releases**: Automated release creation with changelog

## File Storage Locations

- **App Config**: `~/.cc-switch/configurations.json`
- **Claude Settings**: `~/.claude/settings.json` (default) or custom directory
- **Path Resolution**: Supports both absolute paths and home-relative paths

## Important Implementation Details

### Major Architecture Changes
- **Modular Structure**: Codebase refactored from single file to multi-module architecture
- **Interactive Terminal UI**: Full terminal-based interactive menus with keyboard navigation
- **Enhanced Configuration**: Support for model and small_fast_model environment variables
- **Real-time Preview**: Interactive selection shows full configuration details before switching
- **Auto-launching**: Automatic Claude CLI execution after configuration switches

### Command Evolution
- **"switch" → "use"**: The main command changed from `switch` to `use` for clarity
- **Backward Compatibility**: The `switch` command is still available as an alias
- **Interactive Modes**: Both `use` and `current` commands support interactive selection
- **Enhanced Add Command**: Support for positional and flag-based arguments with interactive mode

### Error Handling
- Uses `anyhow` for comprehensive error handling with context
- All file operations include proper error context for debugging
- Graceful handling of missing files (creates defaults)

### Configuration Switching
- The `use cc` command removes API configuration entirely (resets to default)
- Preserves other settings in Claude's settings.json when modifying API config
- Validates configuration existence before switching

### Shell Integration
- Dynamic completion for configuration aliases with real-time loading
- Multi-shell support: fish, zsh, bash, elvish, powershell
- Alias generation system: `cs='cc-switch'` and `ccd='claude --dangerously-skip-permissions'`
- Hidden `--list-aliases` flag for programmatic access
- Eval-compatible alias output for immediate shell integration

### Cross-Platform Support
- Uses `dirs` crate for cross-platform directory resolution
- Handles file path differences between Windows, Linux, and macOS
- CI builds for multiple target architectures

## Testing Strategy

### Test Coverage
- **Unit Tests**: 43 tests covering all core functionality
- **Integration Tests**: Full workflow testing, error scenarios, edge cases
- **Error Handling Tests**: Comprehensive error condition testing including boundary cases
- **Interactive Feature Tests**: 数字键快速选择、分页逻辑、边界条件测试
- **Cross-Platform Tests**: Path resolution, file operations on different platforms

### Test Categories
1. **Configuration Management**: CRUD operations, validation, serialization
2. **Settings Management**: JSON handling, environment variable management
3. **CLI Parsing**: Command structure, argument validation, help generation
4. **Error Handling**: Invalid inputs, file operations, edge cases
5. **Integration**: End-to-end workflows, shell integration
6. **Interactive Features**: 
   - 分页计算和导航逻辑测试
   - 数字键映射和边界条件测试
   - 空配置列表和异常情况处理测试

## Key Dependencies and Their Roles

- **anyhow**: Context-rich error handling and propagation
- **clap**: Command-line argument parsing with auto-generated help and completion
- **clap_complete**: Shell completion script generation
- **serde**: JSON serialization/deserialization with derive macros
- **dirs**: Cross-platform directory resolution (home, config directories)
- **colored**: Terminal output formatting and colors
- **crossterm**: Cross-platform terminal manipulation and events (keyboard navigation, raw mode)
- **tempfile**: Temporary file management for testing

## Common Development Tasks

### Adding New Commands
1. Add variant to `Commands` enum in `src/cmd/cli.rs`
2. Implement command handler function in appropriate module (`src/cmd/main.rs` or dedicated module)
3. Add match arm in `run()` function in `src/cmd/main.rs`
4. Add completion logic if needed in `src/cmd/completion.rs`
5. Write comprehensive tests in appropriate test module
6. Update help text and documentation

### Modifying Configuration Structure
1. Update `Configuration` struct
2. Update serialization/deserialization logic if needed
3. Modify storage operations
4. Update tests to reflect changes
5. Test backward compatibility

### Adding New Shell Support
1. Add shell variant to `generate_completion()` function in `src/cmd/completion.rs`
2. Implement shell-specific completion logic in `src/cmd/shell_completion.rs`
3. Add to `generate_aliases()` function for alias support
4. Update help text in `src/cmd/cli.rs`
5. Test completion and alias functionality across platforms

## Important Notes for Future Development

1. **Backward Compatibility**: Maintain compatibility with existing configuration files
2. **Error Context**: Provide detailed error messages with context for debugging
3. **Cross-Platform**: Test on all supported platforms (Linux, macOS, Windows)
4. **Security**: Handle API tokens securely, avoid logging sensitive data
5. **Testing**: Maintain high test coverage (currently 100% with 57 tests)
6. **Documentation**: Keep README.md and CLAUDE.md synchronized with code changes