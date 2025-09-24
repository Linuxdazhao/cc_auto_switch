# NPM Installation Guide for cc-switch

## 概述

`cc-switch` 现在支持通过 NPM 安装！这为 JavaScript/Node.js 开发者提供了一种熟悉的安装方式。

## 安装方式

### 全局安装（推荐）

```bash
# 全局安装 cc-switch
npm install -g cc-switch

# 安装完成后直接使用
cc-switch --version
cc-switch --help
```

### 本地项目安装

```bash
# 在项目中安装
npm install cc-switch

# 使用 npx 运行
npx cc-switch --version
npx cc-switch use my-config
```

### 一次性运行（无需安装）

```bash
# 直接运行最新版本
npx cc-switch@latest --help
npx cc-switch@latest list
```

## 工作原理

NPM 包使用智能二进制分发机制：

1. **自动平台检测**：安装时自动检测您的操作系统和架构
2. **二进制下载**：从 GitHub Releases 下载对应的预编译二进制文件
3. **本地缓存**：二进制文件缓存在 `node_modules/cc-switch/bin/` 目录

### 支持的平台

| 操作系统 | 架构 | 目标平台 |
|---------|------|----------|
| macOS | x64 | x86_64-apple-darwin |
| macOS | arm64 | aarch64-apple-darwin |
| Linux | x64 | x86_64-unknown-linux-musl |
| Linux | arm64 | aarch64-unknown-linux-musl |
| Windows | x64 | x86_64-pc-windows-gnu |

## 使用示例

```bash
# 全局安装
npm install -g cc-switch

# 添加配置
cc-switch add my-config sk-ant-xxx https://api.anthropic.com

# 切换配置
cc-switch use my-config

# 列出所有配置
cc-switch list

# 交互式当前配置菜单
cc-switch current
```

## 与 Rust 版本的兼容性

NPM 版本与 Rust 版本完全兼容：
- 相同的命令行接口
- 相同的配置文件格式
- 相同的功能特性
- 可以与通过 `cargo install` 安装的版本共存

## 故障排除

### 安装失败

如果安装失败，通常是由于：

1. **网络连接问题**：确保可以访问 GitHub
2. **版本不匹配**：检查 GitHub Releases 是否存在对应版本
3. **平台不支持**：检查您的平台是否在支持列表中

```bash
# 检查网络连接
curl -I https://github.com/Linuxdazhao/cc_auto_switch/releases/latest

# 查看详细错误日志
npm install -g cc-switch --verbose
```

### 权限问题

```bash
# 如果遇到权限问题，使用 sudo (Linux/macOS)
sudo npm install -g cc-switch

# 或者配置 npm 全局目录
npm config set prefix ~/.npm-global
export PATH=~/.npm-global/bin:$PATH
```

### 二进制文件缺失

```bash
# 重新安装
npm uninstall -g cc-switch
npm install -g cc-switch

# 或者清除缓存后重新安装
npm cache clean --force
npm install -g cc-switch
```

## 开发者信息

### 发布到 NPM

```bash
# 发布新版本到 NPM
npm publish

# 发布测试版本
npm publish --tag beta
```

### 本地测试

```bash
# 创建本地包
npm pack

# 测试安装
npm install -g cc-switch-0.0.31.tgz
```

## 版本同步

NPM 包版本与 Rust crate 版本保持同步：
- 主版本号对应 Rust crate 的主版本
- 功能更新同时发布到 crates.io 和 npmjs.com
- GitHub Releases 包含所有平台的预编译二进制文件

## 反馈与支持

- **GitHub Issues**: https://github.com/Linuxdazhao/cc_auto_switch/issues
- **NPM Package**: https://www.npmjs.com/package/cc-switch
- **Rust Crate**: https://crates.io/crates/cc-switch