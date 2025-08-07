#!/bin/bash

# Version increment script for cc-switch
# This script automatically increments the version in Cargo.toml when code changes are made

set -e

CARGO_TOML="Cargo.toml"

# Check if Cargo.toml exists
if [ ! -f "$CARGO_TOML" ]; then
    echo "Error: $CARGO_TOML not found"
    exit 1
fi

# Extract current version
current_version=$(grep -m1 '^version = ' "$CARGO_TOML" | sed 's/version = "\(.*\)"/\1/')
if [ -z "$current_version" ]; then
    echo "Error: Could not find version in $CARGO_TOML"
    exit 1
fi

echo "Current version: $current_version"

# Parse version components (assume semantic versioning x.y.z)
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

echo "✅ Version updated to $new_version in $CARGO_TOML"

# Stage the updated Cargo.toml for commit
git add "$CARGO_TOML"

echo "📦 $CARGO_TOML staged for commit"