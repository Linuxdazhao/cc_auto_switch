# Analysis for Task #24: Update UI hints and documentation

## Current Implementation
The interactive selection UI shows help text with old keybindings:
- Line 439: "↑↓导航，1-9快选，U-编辑，N/P翻页，R-官方，E-退出，Enter确认"
- Line 449: "↑↓导航，1-9快选，U-编辑，R-官方，E-退出，Enter确认，Esc取消"

## Required Changes
1. Update help text to show new keybindings:
   - `E` instead of `U` for editing
   - `Q` instead of `E` for exit
2. Update simple menu text to reflect new keybindings

## Implementation Location
File: `src/cmd/interactive.rs`
1. Line 439: Update help text for multi-page menu
2. Line 449: Update help text for single-page menu
3. Line 683: Update simple menu official option label
4. Line 705: Update simple menu exit option label
5. Line 709: Update simple menu navigation help
6. Line 760: Update simple single page menu official option
7. Line 778: Update simple single page menu exit option
8. Line 780: Update simple single page menu prompt

## Implementation Plan
1. Replace `U` with `E` for editing in all help texts
2. Replace `E` with `Q` for exit in all help texts
3. Update all related UI text to reflect new keybindings

## Test Considerations
- Verify all help text shows correct keybindings
- Ensure consistency across all UI elements
- Test both single-page and multi-page menu displays