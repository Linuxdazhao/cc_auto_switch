#!/bin/bash

# Pre-commit setup script for cc_auto_switch
# This script installs pre-commit and sets up the hooks

set -e

echo "🔧 Setting up pre-commit hooks for cc_auto_switch..."

# Check if pre-commit is installed
if ! command -v pre-commit &> /dev/null; then
    echo "📦 Installing pre-commit..."
    
    # Try different installation methods based on what's available
    if command -v pipx &> /dev/null; then
        echo "   Using pipx to install pre-commit..."
        pipx install pre-commit
    elif command -v brew &> /dev/null; then
        echo "   Using Homebrew to install pre-commit..."
        brew install pre-commit
    elif command -v pip3 &> /dev/null; then
        echo "   Using pip3 with --user flag..."
        pip3 install --user pre-commit
    elif command -v pip &> /dev/null; then
        echo "   Using pip with --user flag..."
        pip install --user pre-commit
    else
        echo "❌ Error: No package manager found. Please install pre-commit manually:"
        echo "   Option 1: brew install pre-commit"
        echo "   Option 2: pipx install pre-commit"
        echo "   Option 3: pip3 install --user pre-commit"
        exit 1
    fi
else
    echo "✅ pre-commit is already installed"
fi

# Install the pre-commit hooks
echo "🔗 Installing pre-commit hooks..."
pre-commit install

# Verify installation
echo "🔍 Verifying installation..."
if pre-commit --version &> /dev/null; then
    echo "✅ Pre-commit setup completed successfully!"
    echo ""
    echo "📋 Available hooks:"
    echo "   • cargo-fmt: Code formatting check" 
    echo "   • cargo-clippy: Linting with warnings as errors"
    echo "   • cargo-test: Run all tests"
    echo "   • cargo-audit: Security vulnerability scan (auto-installs if needed)"
    echo "   • cargo-doc: Documentation build check"
    echo "   • cargo-build-release: Release build + compile verification (removes artifacts after)"
    echo ""
    echo "🔄 Testing hooks (dry run)..."
    pre-commit run --all-files --show-diff-on-failure || echo "⚠️  Some hooks may need fixes before committing"
    echo ""
    echo "💡 Usage:"
    echo "   • Run manually: pre-commit run --all-files"
    echo "   • Run on specific files: pre-commit run --files src/main.rs"
    echo "   • Skip hooks: git commit --no-verify"
    echo "   • Update hooks: pre-commit autoupdate"
    echo ""
    echo "🚀 Ready to go! All CI checks will now run locally before commits."
else
    echo "❌ Error: pre-commit installation failed"
    exit 1
fi