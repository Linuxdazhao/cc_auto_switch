# Code Style and Conventions

## Rust Code Style
- **Edition**: 2024 (latest)
- **Formatting**: Standard rustfmt (enforced via pre-commit)
- **Linting**: Clippy with warnings as errors (-D warnings)
- **Error Handling**: anyhow::Result for error propagation with context

## Module Structure
- **Entry Point**: Minimal main.rs that delegates to cmd module
- **Core Logic**: Organized in src/cmd/ with clear separation of concerns:
  - cli.rs: Command-line interface definitions
  - config.rs: Configuration management
  - config_storage.rs: Persistence layer
  - types.rs: Core data structures
  - interactive.rs: Terminal UI and user interaction
  - completion.rs: Shell completion logic
  - utils.rs: Utility functions

## Testing Conventions
- **Test Organization**: 
  - Unit tests in dedicated test files (tests.rs, main_tests.rs, etc.)
  - Integration tests in integration_tests.rs
  - Error handling tests in error_handling_tests.rs
- **Test Coverage**: Comprehensive testing (currently 100% with 57 tests)
- **Test Data**: Use tempfile for temporary file operations

## Naming Conventions
- **Binary**: cc-switch (kebab-case)
- **Crate**: cc-switch 
- **Functions**: snake_case
- **Types**: PascalCase
- **Constants**: SCREAMING_SNAKE_CASE

## Documentation
- **README**: Bilingual (Chinese primary, English secondary)
- **Code Comments**: Minimal, focus on why not what
- **API Documentation**: cargo doc support with --no-deps

## Configuration Management
- **Storage**: JSON files in ~/.cc-switch/
- **Environment Integration**: Via Claude's settings.json
- **Cross-Platform**: Use dirs crate for path resolution

## Security Practices
- **Token Handling**: Secure storage, no logging of sensitive data
- **File Permissions**: Appropriate for configuration files
- **Audit**: Regular cargo audit runs via pre-commit hooks