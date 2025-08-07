#!/bin/bash

# Complete workflow script for cc-switch (bypassing pre-commit hooks)
# This script handles version increment, commit, and publishing

set -e

echo "🚀 Starting cc-switch release workflow..."

# Check if there are changes to commit
if [ -z "$(git status --porcelain)" ]; then
    echo "ℹ️  No changes to commit"
    exit 0
fi

# Stage all changes
git add .

# Get current version
CARGO_TOML="Cargo.toml"
current_version=$(grep -m1 '^version = ' "$CARGO_TOML" | sed 's/version = "\(.*\)"/\1/')
echo "Current version: $current_version"

# Parse version components
if [[ $current_version =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)$ ]]; then
    major=${BASH_REMATCH[1]}
    minor=${BASH_REMATCH[2]}
    patch=${BASH_REMATCH[3]}
    
    # Increment patch version
    new_patch=$((patch + 1))
    new_version="$major.$minor.$new_patch"
else
    echo "Error: Version format not supported. Expected x.y.z format"
    exit 1
fi

echo "New version: $new_version"

# Update version in Cargo.toml
sed -i.tmp "s/^version = \"$current_version\"/version = \"$new_version\"/" "$CARGO_TOML"
rm -f "$CARGO_TOML.tmp"

echo "✅ Version updated to $new_version"

# Stage the version update
git add "$CARGO_TOML"

# Use default commit message
commit_msg="Release v$new_version"

# Commit with version info (bypassing pre-commit hooks)
git commit --no-verify -m "$commit_msg

📦 Version: $new_version"

echo "✅ Committed changes"

# Run tests
echo "🧪 Running tests..."
cargo test

if [ $? -eq 0 ]; then
    echo "✅ Tests passed"
    
    # Publish to crates.io
    echo "🚀 Publishing to crates.io..."
    ./scripts/publish.sh
else
    echo "❌ Tests failed, skipping publish"
    exit 1
fi

echo "🎉 Release workflow completed successfully!"