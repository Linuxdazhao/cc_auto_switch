# Homebrew 安装指南

cc-switch 现在支持通过 Homebrew 安装！

## 安装

```bash
# 添加 tap
brew tap Linuxdazhao/cc-switch

# 安装 cc-switch
brew install cc-switch
```

## 支持的平台

- **macOS Intel** (x86_64)
- **macOS Apple Silicon** (ARM64/M1/M2)
- **Linux x86_64**
- **Linux ARM64**

## 更新

```bash
# 更新到最新版本
brew update && brew upgrade cc-switch
```

## 卸载

```bash
# 卸载 cc-switch
brew uninstall cc-switch

# 移除 tap（可选）
brew untap Linuxdazhao/cc-switch
```

## 自动化

每次发布新版本时，Homebrew formula 会自动更新：

1. 当创建新的 git tag 时，GitHub Actions 构建多平台二进制包
2. Release 发布后，另一个 GitHub Action 自动更新 Homebrew tap
3. 用户可以通过 `brew update && brew upgrade cc-switch` 获取最新版本

## Tap 仓库

Homebrew tap 仓库：https://github.com/Linuxdazhao/homebrew-cc-switch

## 其他安装方式

- **从源码**: `cargo install cc-switch`
- **从 crates.io**: https://crates.io/crates/cc-switch  
- **二进制包**: https://github.com/Linuxdazhao/cc_auto_switch/releases

## 使用指南

安装后，参考主 README 获取详细使用说明：https://github.com/Linuxdazhao/cc_auto_switch