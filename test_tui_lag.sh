#!/bin/bash
# Test script to capture TUI lag with full instrumentation

set -e

# Build first
echo "Building botticelli_tui..."
cargo build --package botticelli_tui --release 2>&1 | tail -5

# Run with full tracing
echo "Running TUI with full tracing..."
echo "Type some characters quickly and press Ctrl+C to exit"
echo "Logs will be in botticelli-tui-test.log"
echo ""

RUST_LOG=botticelli_tui=trace,trace cargo run --bin botticelli-chat --features="cli,tui" 2>&1 | tee botticelli-tui-test.log

echo ""
echo "Analyzing logs for timing issues..."
echo ""
echo "=== Event processing times ==="
grep "Event processing" botticelli-tui-test.log | tail -20

echo ""
echo "=== Handle key times ==="
grep "handle_key" botticelli-tui-test.log | tail -20

echo ""
echo "=== Render times ==="  
grep "render" botticelli-tui-test.log | grep -i "ms\|slow" | tail -20

echo ""
echo "=== Channel send times ==="
grep "Channel send" botticelli-tui-test.log | tail -20
