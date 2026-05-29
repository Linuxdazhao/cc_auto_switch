#!/bin/bash

# Pre-commit setup script for cc_auto_switch.
# Installs `prek` (Rust-based, drop-in compatible with .pre-commit-config.yaml)
# and wires up its hook into .git/hooks/pre-commit.

set -e

echo "🔧 Setting up prek hooks for cc_auto_switch..."

if ! command -v prek &> /dev/null; then
    echo "📦 Installing prek..."

    if command -v brew &> /dev/null; then
        echo "   Using Homebrew to install prek..."
        brew install prek
    elif command -v cargo &> /dev/null; then
        echo "   Using cargo to install prek..."
        cargo install prek
    else
        echo "❌ Error: No supported installer found. Install prek manually:"
        echo "   Option 1: brew install prek"
        echo "   Option 2: cargo install prek"
        echo "   See https://github.com/j178/prek for other options."
        exit 1
    fi
else
    echo "✅ prek is already installed ($(prek --version))"
fi

echo "🔗 Installing prek git hook..."
prek install --overwrite

echo ""
echo "📋 Configured hooks (from .pre-commit-config.yaml):"
echo "   • cargo-fmt: cargo fmt --all -- --check (matches CI)"
echo "   • cargo-clippy: Linting with warnings as errors"
echo "   • cargo-test: Run all tests"
echo "   • cargo-audit: Security vulnerability scan (auto-installs if needed)"
echo "   • cargo-doc: Documentation build check"
echo "   • cargo-build-release: Release build + compile verification"
echo ""
echo "🔄 Dry run on all files..."
prek run --all-files || echo "⚠️  Some hooks reported issues — fix before committing"
echo ""
echo "💡 Usage:"
echo "   • Run manually: prek run --all-files"
echo "   • Run a single hook: prek run cargo-fmt --all-files"
echo "   • Skip hooks for one commit: git commit --no-verify"
echo "   • Update hook versions: prek autoupdate"
echo ""
echo "🚀 Ready. CI-equivalent checks will run before every commit."
