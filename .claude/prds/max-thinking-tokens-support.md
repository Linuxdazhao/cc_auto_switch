---
name: max-thinking-tokens-support
description: Add MAX_THINKING_TOKENS environment variable support for configuring Claude's thinking token budget in API configurations
status: backlog
created: 2025-09-24T00:12:31Z
---

# PRD: max-thinking-tokens-support

## Executive Summary

Add support for the `MAX_THINKING_TOKENS` environment variable to the cc_auto_switch configuration management system. This feature allows users to configure Claude's thinking token budget per API configuration, enabling fine-tuned control over Claude's extended reasoning capabilities while managing token costs and response verbosity.

## Problem Statement

### Current State
- cc_auto_switch manages Claude API configurations (token, URL, model settings)
- Users cannot configure thinking token budgets per API configuration
- Users must manually export `MAX_THINKING_TOKENS` globally or per session
- No integrated way to switch thinking token settings with API configurations

### Pain Points
1. **Manual Environment Management**: Users must remember to set `MAX_THINKING_TOKENS` separately from API configuration switches
2. **Configuration Drift**: Thinking token settings are not tied to specific API configurations, leading to inconsistent behavior
3. **All-or-Nothing Behavior**: Current Claude Code implementation forces thinking mode on ALL requests when `MAX_THINKING_TOKENS` is set
4. **Cost Management**: No easy way to manage thinking token budgets across different API endpoints/accounts
5. **Workflow Interruption**: Users must manually manage environment variables outside the cc_auto_switch workflow

### Why Now?
- Claude Code has known issues with `MAX_THINKING_TOKENS` forcing thinking on all requests (#5257)
- Users need granular control over thinking behavior per API configuration
- Growing demand for cost optimization in AI workflows
- Upcoming Claude Code improvements may need configuration-aware thinking settings

## User Stories

### Primary Personas

**Persona 1: Multi-Environment Developer**
- Manages multiple Claude API accounts (development, staging, production)
- Needs different thinking token budgets per environment
- Wants cost control and predictable behavior

**Persona 2: Cost-Conscious Individual User**
- Uses Claude Code for various tasks with different complexity needs
- Wants thinking tokens for complex analysis but not simple queries
- Needs easy switching between "thinking" and "standard" modes

**Persona 3: Team Lead/DevOps Engineer**
- Manages team configurations across projects
- Needs standardized thinking token settings per project/client
- Requires easy distribution of optimized configurations

### User Journeys

**Journey 1: Adding Thinking Token Configuration**
```
User wants to add a new API config with thinking tokens
1. Run: cc-switch add my-config -t TOKEN -u URL --thinking-tokens 2048
2. System stores thinking token setting with the configuration
3. User switches: cc-switch use my-config
4. System sets both API config AND MAX_THINKING_TOKENS environment variable
5. Claude Code uses the specified thinking token budget
```

**Journey 2: Switching Between Thinking Modes**
```
User has multiple configs with different thinking settings
1. Current config: "standard" (no thinking tokens)
2. Switch to: cc-switch use complex-analysis (thinking-tokens: 4096)
3. System updates Claude settings AND sets MAX_THINKING_TOKENS=4096
4. User gets thinking mode for complex tasks
5. Switch back: cc-switch use standard
6. System removes MAX_THINKING_TOKENS (or sets to 0)
7. User gets normal Claude behavior
```

**Journey 3: Interactive Thinking Token Management**
```
User wants to modify thinking tokens for existing config
1. Run: cc-switch current (interactive menu)
2. Navigate to configuration
3. See thinking token setting in preview
4. Option to modify thinking tokens inline
5. System updates configuration and applies immediately
```

## Requirements

### Functional Requirements

**FR1: Configuration Storage**
- Store `thinking_tokens` field in Configuration struct
- Support values: null (no thinking), 0 (disabled), 1024-32000 (token budget)
- Serialize/deserialize with existing configuration JSON format
- Maintain backward compatibility with existing configurations

**FR2: CLI Interface Extensions**
- Add `--thinking-tokens` / `-T` flag to `add` command
- Support interactive thinking token configuration in add mode
- Display thinking token settings in `list` command output
- Show thinking token info in `current` command interactive menu

**FR3: Environment Variable Management**
- Set `MAX_THINKING_TOKENS` environment variable when switching configurations
- Unset or set to 0 when switching to configurations without thinking tokens
- Handle Claude settings.json environment variable updates
- Preserve other environment variables in settings.json

**FR4: Interactive Features**
- Show thinking token budget in configuration preview
- Allow modification of thinking tokens in interactive current menu
- Provide quick toggle between thinking/non-thinking modes
- Visual indicators for thinking-enabled configurations

**FR5: Validation and Defaults**
- Validate thinking token values (1024 minimum, 32000 maximum)
- Default to null (no thinking tokens) for new configurations
- Provide helpful error messages for invalid values
- Warn about cost implications for high token budgets

### Non-Functional Requirements

**NFR1: Performance**
- Environment variable updates should complete within 100ms
- No noticeable delay when switching configurations
- Efficient JSON serialization with new field

**NFR2: Compatibility**
- Maintain backward compatibility with existing configurations.json
- Work with existing Claude Code installations
- Support all platforms (Linux, macOS, Windows)

**NFR3: Reliability**
- Graceful handling of missing environment variable support
- Rollback capability if environment variable setting fails
- Consistent state between configuration and environment

**NFR4: Usability**
- Clear documentation of thinking token budget implications
- Helpful CLI help text and examples
- Intuitive interactive interfaces

## Success Criteria

### Measurable Outcomes

**Primary Metrics**
1. **Feature Adoption**: 40% of users add thinking token configurations within 3 months
2. **Workflow Efficiency**: 60% reduction in manual environment variable management
3. **Configuration Completeness**: 90% of new configurations include thinking token settings

**Secondary Metrics**
1. **Error Reduction**: 80% reduction in thinking token-related configuration errors
2. **User Satisfaction**: 4.5/5 rating for thinking token configuration usability
3. **Documentation Clarity**: <5% support requests about thinking token configuration

### Acceptance Criteria

**AC1: Basic Functionality**
- ✅ Add configuration with thinking tokens: `cc-switch add test -t TOKEN -u URL -T 2048`
- ✅ Switch to configuration sets MAX_THINKING_TOKENS environment variable
- ✅ List command shows thinking token settings
- ✅ Backward compatibility with existing configurations

**AC2: Interactive Features**
- ✅ Interactive add mode includes thinking token prompt
- ✅ Current command shows thinking token in configuration preview
- ✅ Visual indicators distinguish thinking-enabled configurations

**AC3: Edge Cases**
- ✅ Switching to configuration without thinking tokens unsets MAX_THINKING_TOKENS
- ✅ Invalid thinking token values show helpful error messages
- ✅ Configuration modification updates environment variables immediately

## Constraints & Assumptions

### Technical Constraints
- Must work with existing Claude settings.json format
- Limited by Claude Code's current MAX_THINKING_TOKENS implementation
- Environment variable changes require Claude CLI restart to take effect
- JSON schema must remain backward compatible

### Resource Constraints
- Implementation should reuse existing configuration management patterns
- Minimal impact on binary size and startup time
- No external dependencies beyond existing crates

### Timeline Constraints
- Feature should be ready for next minor version release
- Implementation complexity should be low-to-moderate
- Testing should cover all major use cases

### Assumptions
- Users understand thinking token cost implications
- Claude Code will eventually fix the "all requests become thinking" issue
- Environment variable approach is acceptable for configuration
- Most users will use thinking tokens occasionally, not always

## Out of Scope

### Explicitly NOT Building

**Advanced Thinking Token Features**
- Per-request thinking token configuration
- Dynamic thinking token adjustment based on query complexity
- Thinking token usage analytics or monitoring
- Integration with Claude Code's internal thinking heuristics

**Complex Configuration Management**
- Conditional thinking token rules
- Time-based thinking token schedules
- Project-specific thinking token profiles
- Team-wide thinking token policy enforcement

**Alternative Configuration Methods**
- Direct Claude settings.json editing interface
- Thinking token configuration via config files
- Integration with external configuration management systems

**Performance Optimization**
- Thinking token budget optimization algorithms
- Automatic thinking token tuning
- Cost prediction for thinking token usage

## Dependencies

### External Dependencies
- **Claude Code**: Must support MAX_THINKING_TOKENS environment variable
- **Claude API**: Thinking token feature availability and behavior
- **Operating System**: Environment variable support across platforms

### Internal Dependencies
- **Configuration Management**: Existing config.rs and config_storage.rs modules
- **CLI Parsing**: clap crate for new argument handling
- **Interactive UI**: crossterm integration for menu updates
- **Settings Management**: Claude settings.json modification logic

### Team Dependencies
- **Documentation Team**: Update user guides and examples
- **Testing Team**: Comprehensive testing across platforms
- **Release Management**: Coordinate with Claude Code updates

## Implementation Notes

### Technical Approach
1. **Extend Configuration Struct**: Add optional `thinking_tokens: Option<u32>` field
2. **CLI Integration**: Add thinking token flags to add command, update help text
3. **Environment Management**: Extend Claude settings management to handle MAX_THINKING_TOKENS
4. **Interactive Updates**: Enhance current command menu to show and edit thinking tokens
5. **Validation Layer**: Add thinking token value validation with clear error messages

### Migration Strategy
- Existing configurations.json files continue to work unchanged
- New thinking_tokens field defaults to null (no thinking tokens)
- Gradual rollout with feature flag if needed

### Testing Strategy
- Unit tests for configuration serialization/deserialization
- Integration tests for environment variable management
- Manual testing across platforms
- Performance testing for configuration switching speed