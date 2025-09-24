---
name: support-vim-bindkey-in-edit-config
status: completed
created: 2025-09-24T08:49:32Z
progress: 100%
prd: .claude/prds/support-vim-bindkey-in-edit-config.md
github: https://github.com/Linuxdazhao/cc_auto_switch/issues/20
---

# Epic: support-vim-bindkey-in-edit-config

## Overview
Implement vim-style keybindings (j/k for navigation, e for edit, q for exit) in the alias selection interface to improve productivity for users familiar with vim navigation patterns. This enhancement will integrate with the existing Crossterm-based terminal UI framework.

## Architecture Decisions
- Leverage existing Crossterm library for keyboard event handling
- Maintain backward compatibility with current navigation methods
- Implement keybinding logic within the existing interactive selection module
- Use event-driven approach for key handling to ensure <50ms response time
- Follow existing code patterns and error handling conventions

## Technical Approach
### Frontend Components
- Modify interactive selection interface in `src/cmd/interactive.rs`
- Extend keyboard event handling to recognize j, k, e, q keys
- Maintain existing arrow key and Enter key functionality
- Update UI hints to reflect new keybindings

### Backend Services
- No backend services required - this is purely a CLI interface enhancement
- Update key event processing logic in the interactive module
- Ensure proper mapping of e key (previously 'u') for edit functionality
- Ensure proper mapping of q key (previously 'e') for exit functionality

### Infrastructure
- No infrastructure changes required
- Leverage existing terminal UI framework (Crossterm)
- Maintain cross-platform compatibility (Linux, macOS, Windows)
- No additional dependencies needed

## Implementation Strategy
### Phase 1: Key Event Handling
- Add j/k key support for navigation in alias selection
- Implement e key for edit functionality (replacing 'u')
- Implement q key for exit functionality (replacing 'e')
- Maintain all existing keyboard navigation options

### Phase 2: Integration and Testing
- Integrate with existing interactive selection logic
- Ensure backward compatibility with current navigation
- Test cross-platform functionality
- Validate performance requirements (<50ms response time)

### Risk Mitigation
- Thorough testing of keyboard event handling
- Maintain all existing functionality as fallback
- Comprehensive error handling for edge cases
- Cross-platform testing before release

## Task Breakdown Preview
- [ ] Implement j/k key navigation in interactive selection
- [ ] Remap e key for edit functionality
- [ ] Remap q key for exit functionality
- [ ] Update UI hints and documentation
- [ ] Cross-platform testing and validation

## Dependencies
- Internal: Crossterm library for terminal handling (already integrated)
- External: None
- Prerequisite work: Understanding of existing interactive selection implementation in `src/cmd/interactive.rs`

## Success Criteria (Technical)
- Keyboard events respond within 50ms
- 100% backward compatibility maintained
- Functionality works across Linux, macOS, and Windows
- All existing unit and integration tests pass
- No performance degradation in alias selection interface

## Estimated Effort
- Overall timeline: 1-2 days
- Resource requirements: 1 developer
- Critical path: Key event handling implementation and integration

## Tasks Created
- [ ] #21 - Implement j/k key navigation in interactive selection (parallel: true)
- [ ] #22 - Remap e key for edit functionality (parallel: true)
- [ ] #23 - Remap q key for exit functionality (parallel: true)
- [ ] #24 - Update UI hints and documentation (parallel: false)
- [ ] #25 - Cross-platform testing and validation (parallel: false)

Total tasks: 5
Parallel tasks: 3
Sequential tasks: 2
Estimated total effort: 10-14 hours
