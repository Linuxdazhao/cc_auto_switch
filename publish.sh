#!/bin/bash

# Script to publish cc-switch to GitHub Packages
# Usage: ./publish.sh [version]

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if help is requested
if [[ "$1" == "--help" ]] || [[ "$1" == "-h" ]]; then
    echo "Usage: ./publish.sh [version]"
    echo "Publish cc-switch to GitHub Packages"
    echo ""
    echo "Arguments:"
    echo "  version    Optional version to publish (default: current version in Cargo.toml)"
    echo "  --help,-h  Show this help message"
    exit 0
fi

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    print_error "Cargo is not installed. Please install Rust first."
    exit 1
fi

# Get current version from Cargo.toml
CURRENT_VERSION=$(grep "^version = " Cargo.toml | cut -d '"' -f 2)
print_info "Current version: $CURRENT_VERSION"

# Check if version argument is provided
if [ $# -eq 0 ]; then
    VERSION=$CURRENT_VERSION
    print_info "Using current version: $VERSION"
else
    VERSION=$1
    print_info "Using provided version: $VERSION"
    
    # Update version in Cargo.toml
    sed -i.bak "s/^version = \".*\"/version = \"$VERSION\"/" Cargo.toml
    print_info "Updated version in Cargo.toml to $VERSION"
fi

# Run cargo publish dry-run first
print_info "Running cargo publish --dry-run..."
cargo publish --dry-run

# Ask for confirmation
echo
print_warn "Ready to publish version $VERSION to GitHub Packages"
read -p "Do you want to continue? (y/N): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    print_info "Publish cancelled."
    # Cleanup backup file if it exists
    if [ -f Cargo.toml.bak ]; then
        rm Cargo.toml.bak
    fi
    exit 0
fi

# Publish to GitHub Packages
print_info "Publishing to GitHub Packages..."
cargo publish

print_info "Successfully published version $VERSION to GitHub Packages!"
print_info "Package is now available at: https://github.com/jingzhao/cc_auto_switch/packages"

# Cleanup backup file if it exists
if [ -f Cargo.toml.bak ]; then
    rm Cargo.toml.bak
fi