#!/bin/bash

# Manual / emergency publish script for cc-switch.
#
# NORMAL PATH: pushing a v* tag triggers .github/workflows/publish.yml which
# runs `cargo publish` from CI. Do NOT call this script from release.sh — it
# will race the workflow and one side will fail with "already exists on
# crates.io index".
#
# Use this script only when CI is unavailable or the workflow failed and you
# need to publish from your machine.

set -e

echo "🚀 Publishing cc-switch to crates.io (manual / emergency path)..."

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