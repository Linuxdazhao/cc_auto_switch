#!/usr/bin/env fish

# Test script to verify cc-switch completion functionality

echo "=== Testing cc-switch completion functionality ==="
echo

# First, build the project
echo "Building cc-switch..."
cargo build --release
if test $status -ne 0
    echo "Failed to build cc-switch"
    exit 1
end
echo "Build successful!"
echo

# Set up test environment
echo "Setting up test environment..."
set CC_SWITCH "/Users/jingzhao/target-rust/release/cc-switch"

# Test 1: Basic alias listing
echo "=== Test 1: Basic alias listing ==="
$CC_SWITCH --list-aliases
echo

# Test 2: Completion generation
echo "=== Test 2: Fish completion generation ==="
$CC_SWITCH completion fish | head -20
echo

# Test 3: Alias generation
echo "=== Test 3: Fish alias generation ==="
$CC_SWITCH alias fish
echo

# Test 4: Verify current is prioritized
echo "=== Test 4: Verify 'current' is prioritized ==="
set aliases ($CC_SWITCH --list-aliases)
echo "Aliases in order:"
for i in (seq (count $aliases))
    echo "$i: $aliases[$i]"
end

# Check if current is second (after cc)
if test $aliases[2] = "current"
    echo "✅ SUCCESS: 'current' is correctly prioritized second"
else
    echo "❌ FAILURE: 'current' is not prioritized correctly"
    echo "Expected 'current' to be second, but got: $aliases[2]"
end
echo

echo "=== All tests completed ==="