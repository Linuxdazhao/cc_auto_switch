#!/bin/bash

# Pre-commit setup script for cc_auto_switch
# This script installs pre-commit and sets up the hooks

set -e

echo "🔧 Setting up pre-commit hooks for cc_auto_switch..."

# Check if pre-commit is installed
if ! command -v pre-commit &> /dev/null; then
    echo "📦 Installing pre-commit..."
    # Install pre-commit using pip
    if command -v pip3 &> /dev/null; then
        pip3 install pre-commit
    elif command -v pip &> /dev/null; then
        pip install pre-commit
    else
        echo "❌ Error: pip is not installed. Please install pip first:"
        echo "   On macOS: brew install python"
        echo "   On Ubuntu: sudo apt-get install python3-pip"
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
    pre-commit run --all-files --show-diff-on-failure || true
    echo ""
    echo "💡 Usage:"
    echo "   - Run manually: pre-commit run --all-files"
    echo "   - Run on specific files: pre-commit run --files src/main.rs"
    echo "   - Skip hooks: git commit --no-verify"
    echo ""
    echo "🔄 To update hooks: pre-commit autoupdate"
else
    echo "❌ Error: pre-commit installation failed"
    exit 1
fi