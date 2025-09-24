# Analysis for Task #21: Implement j/k key navigation in interactive selection

## Current Implementation
The interactive selection currently uses arrow keys for navigation:
- `KeyCode::Up` to move selection up
- `KeyCode::Down` to move selection down

## Required Changes
1. Add support for `j` key to move selection down (same as Down arrow)
2. Add support for `k` key to move selection up (same as Up arrow)

## Implementation Location
File: `src/cmd/interactive.rs`
Function: `handle_full_interactive_menu`
Location: Around lines 565-571 where arrow key handling is implemented

## Implementation Plan
1. In the key event match block, add cases for `KeyCode::Char('j')` and `KeyCode::Char('k')`
2. Map `j` to the same behavior as `Down` arrow
3. Map `k` to the same behavior as `Up` arrow
4. Handle both lowercase and uppercase variants for consistency

## Test Considerations
- Ensure `j`/`k` keys work the same as arrow keys
- Verify edge cases (pressing `j` at bottom, `k` at top)
- Test that existing arrow key functionality is not affected