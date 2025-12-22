#!/usr/bin/env bash
# Test script to capture actual lag behavior

set -euo pipefail

echo "Building botticelli-chat..."
cargo build --release --bin botticelli-chat 2>&1 | tail -5

echo "Starting instrumented test..."
echo "Type some characters quickly, then press Ctrl+C"
echo ""

RUST_LOG=botticelli_tui=trace,trace cargo run --release --bin botticelli-chat 2>&1 | tee test_actual_lag.log
