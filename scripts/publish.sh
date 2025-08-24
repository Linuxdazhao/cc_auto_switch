#!/bin/bash

# Publish script for cc-switch
# This script publishes the package to crates.io after version increment

set -e

echo "🚀 Publishing cc-switch to crates.io..."

# Check if Cargo.toml exists
if [ ! -f "Cargo.toml" ]; then
    echo "Error: Cargo.toml not found"
    exit 1
fi

# Extract current version
current_version=$(grep -m1 '^version = ' "Cargo.toml" | sed 's/version = "\(.*\)"/\1/')
echo "Publishing version $current_version..."

# Run cargo publish
if cargo publish --registry crates-io; then
    echo "✅ Successfully published cc-switch v$current_version to crates.io"
else
    echo "❌ Failed to publish to crates.io"
    exit 1
fi