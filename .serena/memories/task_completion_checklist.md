# Task Completion Checklist

## Before Committing Code
1. **Format Code**: `cargo fmt` or `make fmt`
2. **Check Compilation**: `cargo check` or `make check`
3. **Lint Code**: `cargo clippy -- -D warnings` or `make clippy-strict`
4. **Run Tests**: `cargo test` or `make test`
5. **Security Audit**: `cargo audit` or `make audit`
6. **Documentation**: `cargo doc --no-deps` or included in quality checks

## Automated Quality Gates
- **Pre-commit Hooks**: Automatically run all quality checks
- **CI Pipeline**: Multi-platform testing and building
- **Cross-Platform**: Test on Linux, macOS, Windows

## Release Workflow
1. **Code Changes**: Implement and test
2. **Version Management**: `./scripts/increment-version.sh` (if needed)
3. **Quality Check**: `make quality` (all checks)
4. **Release**: `./scripts/release.sh` (handles version + commit + publish)

## Manual Quality Check
```bash
# Quick checks (recommended during development)
make quick-check

# Full quality suite (before commits)
make quality

# Pre-commit hook simulation
make run-hooks
```

## Testing Requirements
- **Unit Tests**: For all core functionality
- **Integration Tests**: End-to-end workflows
- **Error Handling**: Boundary conditions and error scenarios
- **Cross-Platform**: Path resolution and file operations

## Documentation Updates
- **README.md**: Keep synchronized with code changes
- **CLAUDE.md**: Update development instructions
- **Help Text**: Update CLI help for new features

## Performance Considerations
- **Binary Size**: Monitor with `make sizes`
- **Release Optimization**: LTO, strip debug symbols
- **Cross-Platform**: Build and test all target platforms