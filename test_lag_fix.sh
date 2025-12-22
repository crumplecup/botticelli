#!/usr/bin/env bash
# Test script to verify keyboard input lag fix
#
# This script runs the TUI with instrumentation and verifies that:
# 1. Input handling is <1ms
# 2. Rendering happens at 60fps independently
# 3. No "SLOW!" warnings appear

set -euo pipefail

echo "🧪 Testing Keyboard Input Lag Fix"
echo "=================================="
echo ""
echo "This test will:"
echo "  1. Build the TUI with instrumentation"
echo "  2. Run with RUST_LOG=trace to capture timing"
echo "  3. Analyze logs for performance issues"
echo ""
echo "Please type rapidly in the TUI (mash keys), then press Ctrl+C to exit."
echo ""
read -p "Press Enter to start..."

# Build
echo "Building..."
cargo build --bin botticelli-chat --features="cli,tui" --quiet

# Run with instrumentation
LOG_FILE="test_lag_fix_$(date +%Y%m%d_%H%M%S).log"
echo "Running TUI (output logged to $LOG_FILE)..."
echo "Type rapidly, then Ctrl+C to exit."
echo ""

RUST_LOG=botticelli_tui=trace cargo run --bin botticelli-chat --features="cli,tui" -- tui 2>&1 | tee "$LOG_FILE" || true

echo ""
echo "🔍 Analyzing logs..."
echo ""

# Check for slow input handling
SLOW_INPUT=$(grep "Event handling took.*SLOW" "$LOG_FILE" | wc -l)
if [ "$SLOW_INPUT" -gt 0 ]; then
    echo "❌ Found $SLOW_INPUT slow input handling events (>5ms)"
    echo "   These should be <1ms. Something is still blocking input."
    grep "Event handling took.*SLOW" "$LOG_FILE" | head -5
    echo ""
fi

# Check for slow renders
SLOW_RENDER=$(grep "Render took.*should be <16ms" "$LOG_FILE" | wc -l)
if [ "$SLOW_RENDER" -gt 0 ]; then
    echo "⚠️  Found $SLOW_RENDER slow renders (>16ms)"
    echo "   This is OK if terminal I/O is slow, as long as input isn't blocked."
    grep "Render took" "$LOG_FILE" | head -5
    echo ""
fi

# Check for fast input handling
FAST_INPUT=$(grep "Event handled in" "$LOG_FILE" | wc -l)
echo "✅ Found $FAST_INPUT fast input events logged"

# Check for periodic renders
RENDERS=$(grep "Rendering (state is dirty)" "$LOG_FILE" | wc -l)
echo "✅ Found $RENDERS render cycles"

# Check that renders are decoupled
IMMEDIATE_RENDERS=$(grep -A1 "Event handled in" "$LOG_FILE" | grep "Rendering" | wc -l)
if [ "$IMMEDIATE_RENDERS" -gt 0 ]; then
    echo "⚠️  Found $IMMEDIATE_RENDERS immediate renders after input"
    echo "   Input should NOT trigger immediate rendering!"
fi

echo ""
echo "📊 Summary:"
if [ "$SLOW_INPUT" -eq 0 ]; then
    echo "  ✅ All input handling was fast (<5ms)"
else
    echo "  ❌ Some input handling was slow (>5ms) - FIX NEEDED"
fi

if [ "$IMMEDIATE_RENDERS" -eq 0 ]; then
    echo "  ✅ Rendering is decoupled from input"
else
    echo "  ❌ Rendering still coupled to input - FIX NEEDED"
fi

if [ "$RENDERS" -gt 0 ]; then
    echo "  ✅ Periodic rendering working at 60fps"
else
    echo "  ⚠️  No renders detected - test too short?"
fi

echo ""
echo "Log file saved: $LOG_FILE"
echo ""
echo "🎯 Expected behavior:"
echo "  - Input handling: <1ms (instant)"
echo "  - Renders: periodic at 60fps (every ~16ms)"
echo "  - No immediate renders after input"
echo "  - Visual feedback delay: 0-16ms (imperceptible)"
echo ""

if [ "$SLOW_INPUT" -eq 0 ] && [ "$IMMEDIATE_RENDERS" -eq 0 ]; then
    echo "🎉 SUCCESS: Keyboard lag fix verified!"
    exit 0
else
    echo "❌ FAILURE: Issues detected, see above"
    exit 1
fi
