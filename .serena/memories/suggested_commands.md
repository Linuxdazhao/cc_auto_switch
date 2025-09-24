# Suggested Commands for CC-Switch Development

## Build and Run
```bash
# Development build and run
cargo run
make dev

# Release build
cargo build --release
make build

# Install locally
cargo install --path .
make install
```

## Testing
```bash
# Run all tests
cargo test
make test

# Run tests with output
cargo test -- --nocapture
```

## Code Quality
```bash
# Format code
cargo fmt
make fmt

# Check compilation
cargo check
make check

# Lint with clippy
cargo clippy
make lint

# Strict linting (warnings as errors)
cargo clippy -- -D warnings
make clippy-strict

# Security audit
cargo audit
make audit

# All quality checks
make quality
```

## Pre-commit Hooks
```bash
# Setup (one-time)
./scripts/setup-pre-commit.sh
make install-hooks

# Run manually
pre-commit run --all-files
make run-hooks
```

## Cross-Platform Building
```bash
# Build for all platforms
make build-all

# Build specific platforms
make build-linux
make build-macos
make build-windows

# Package for distribution
make package-all
make release
```

## Publishing
```bash
# Dry run publish
make publish-dry-run

# Publish to crates.io
make publish
./scripts/publish.sh

# Full release workflow
./scripts/release.sh
```

## Development Utilities
```bash
# Update dependencies
cargo update
make update

# Show binary sizes
make sizes
make sizes-all

# Clean build artifacts
cargo clean
make clean
```