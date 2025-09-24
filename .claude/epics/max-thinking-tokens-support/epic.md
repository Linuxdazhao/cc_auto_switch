---
name: max-thinking-tokens-support
status: backlog
created: 2025-09-24T00:17:55Z
progress: 0%
prd: .claude/prds/max-thinking-tokens-support.md
github: https://github.com/Linuxdazhao/cc_auto_switch/issues/12
---

# Epic: max-thinking-tokens-support

## Overview

Implement MAX_THINKING_TOKENS environment variable support by extending the existing configuration management system. The implementation follows established patterns for optional model configuration, adding thinking token budget control to API configurations with minimal code changes and full backward compatibility.

## Architecture Decisions

**Configuration Storage**: Extend Configuration struct with `Option<u32>` field using existing serialization patterns
- Leverages `#[serde(skip_serializing_if = "Option::is_none")]` for backward compatibility
- Follows exact pattern established by `model` and `small_fast_model` fields

**Environment Variable Management**: Reuse existing ClaudeSettings environment handling
- Extends `remove_anthropic_env()` and `switch_to_config()` methods
- Maps `ANTHROPIC_MAX_THINKING_TOKENS` following established naming conventions
- Handles u32 to String conversion for environment variables

**CLI Integration**: Follow established optional parameter patterns from clap
- Add `--max-thinking-tokens` flag using existing model parameter structure
- Integrate with AddCommandParams for interactive mode support

**No New Dependencies**: Reuse existing crates (serde, clap, dirs) and patterns
- No external dependencies required
- Minimal binary size impact
- Leverages proven serialization and CLI patterns

## Technical Approach

### Core Data Model
**Configuration Extension** (`src/cmd/types.rs`)
- Add `max_thinking_tokens: Option<u32>` field to Configuration struct
- Use established `#[serde(skip_serializing_if = "Option::is_none")]` pattern
- Extend AddCommandParams with matching field

### CLI Interface
**Command Line Integration** (`src/cmd/cli.rs`)
- Add `--max-thinking-tokens` argument following existing model parameter pattern
- Include validation and help text consistent with existing style
- Support both positional and flag-based configuration

### Environment Management
**Settings Integration** (`src/cmd/config.rs`)
- Extend ClaudeSettings environment variable handling
- Add `ANTHROPIC_MAX_THINKING_TOKENS` to removal and setting logic
- Handle numeric to string conversion for environment variables

### Interactive Features
**Menu Enhancement** (`src/cmd/interactive.rs`)
- Display thinking token settings in configuration previews
- Show visual indicators for thinking-enabled configurations
- Maintain existing keyboard navigation patterns

## Implementation Strategy

**Phase 1: Core Extension (Single Sprint)**
- Extend Configuration struct and serialization
- Add CLI argument parsing
- Implement environment variable management
- Update display logic for interactive menus

**Risk Mitigation**
- Follow established patterns exactly to minimize regression risk
- Maintain full backward compatibility with existing configurations
- Add comprehensive validation for thinking token ranges (1024-32000)

**Testing Approach**
- Unit tests for configuration serialization/deserialization
- CLI argument parsing tests
- Environment variable integration tests
- Interactive menu functionality tests

## Task Breakdown Preview

High-level task categories that will be created:
- [ ] **Core Data Model**: Extend Configuration struct and serialization patterns
- [ ] **CLI Integration**: Add command line argument parsing and validation
- [ ] **Environment Management**: Implement MAX_THINKING_TOKENS environment variable handling
- [ ] **Interactive UI**: Update menu displays and configuration previews
- [ ] **Input Validation**: Add thinking token range validation and error handling
- [ ] **Testing**: Comprehensive test coverage for all new functionality
- [ ] **Documentation**: Update help text, examples, and CLAUDE.md

## Dependencies

**External Dependencies**
- Claude Code MAX_THINKING_TOKENS environment variable support (existing)
- Operating system environment variable capabilities (existing)

**Internal Dependencies**
- Existing configuration management in config.rs and config_storage.rs
- ClaudeSettings environment variable handling
- CLI argument parsing with clap crate
- Interactive UI framework with crossterm

**No Blocking Dependencies**: All required functionality exists in current codebase

## Success Criteria (Technical)

**Performance Benchmarks**
- Configuration switching with thinking tokens: <100ms (same as current)
- Environment variable updates: <50ms additional overhead
- JSON serialization: No measurable performance impact

**Quality Gates**
- 100% backward compatibility with existing configurations.json files
- All existing tests continue to pass
- New functionality covered by comprehensive test suite
- Memory usage increase: <1KB for new optional field

**Acceptance Criteria**
- CLI command: `cc-switch add test -t TOKEN -u URL --max-thinking-tokens 2048` works
- Configuration switching sets ANTHROPIC_MAX_THINKING_TOKENS environment variable
- Interactive menu shows thinking token settings in configuration preview
- Configurations without thinking tokens don't set environment variable

## Estimated Effort

**Overall Timeline**: 1-2 development days
- Configuration extension: 2-3 hours
- CLI integration: 2-3 hours
- Environment management: 1-2 hours
- Interactive UI updates: 1-2 hours
- Testing and validation: 2-4 hours

**Resource Requirements**: Single developer with Rust experience
**Critical Path**: Configuration struct extension → CLI integration → Environment management → Interactive UI

**Low Complexity**: Follows established patterns exactly, minimal new logic required

## Tasks Created

- [ ] #13 - Extend Configuration Data Model for Thinking Tokens (parallel: true)
- [ ] #14 - Add CLI Argument Support for Thinking Tokens (parallel: true)
- [ ] #15 - Implement Environment Variable Management (parallel: true)
- [ ] #16 - Update Interactive UI for Thinking Tokens (parallel: false)
- [ ] #17 - Add Input Validation and Error Handling (parallel: true)
- [ ] #18 - Update Documentation and Help Text (parallel: true)
- [ ] #19 - Implement Comprehensive Test Coverage (parallel: false)

Total tasks: 7
Parallel tasks: 5
Sequential tasks: 2
Estimated total effort: 12-17 hours
