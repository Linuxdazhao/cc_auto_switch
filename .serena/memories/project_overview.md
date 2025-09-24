# CC-Switch Project Overview

## Purpose
CC-Switch is a CLI tool for managing multiple Claude API configurations and automatically switching between them. It allows users to store different API configurations (aliases, tokens, URLs) and switch between them by modifying Claude's settings.json file through environment variables.

## Tech Stack
- **Language**: Rust (Edition 2024)
- **Binary Name**: cc-switch
- **Version**: 0.0.31
- **License**: MIT

## Key Dependencies
- anyhow: Error handling with context
- clap: CLI parsing with derive features and cargo integration
- clap_complete: Shell completion script generation
- serde/serde_json: JSON serialization for configuration
- dirs: Cross-platform directory resolution
- tempfile: Temporary file management for testing
- colored: Terminal output formatting
- crossterm: Cross-platform terminal manipulation

## Architecture
- **Entry Point**: src/main.rs (minimal - delegates to cmd module)
- **Core Logic**: src/cmd/ directory with modular structure
- **Config Storage**: ~/.cc-switch/configurations.json
- **Claude Integration**: ~/.claude/settings.json via environment variables

## Binary Distribution
- Releases optimized for size (LTO, strip debug, panic=abort)
- Cross-platform builds for Linux, macOS, Windows (x86_64 and aarch64)
- UPX compression support for smaller binaries