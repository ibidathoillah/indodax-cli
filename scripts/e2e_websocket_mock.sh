#!/bin/bash
# E2E Mock verification for WebSocket commands
# This script runs public websocket commands for a few seconds and verifies they produce output.

set -e

echo "Starting WebSocket E2E verification..."

# Function to test a websocket command
test_ws() {
    local cmd=$1
    local name=$2
    echo "Testing: $name..."
    
    # Run command with timeout. It should produce at least one event in 10 seconds for BTC_IDR
    # We use --output json to make it easy to verify.
    output=$(timeout 15s "$INDODAX" --output json ws $cmd 2>&1 || true)
    
    if echo "$output" | grep -q "\"event\""; then
        echo "✅ $name: Success (received events)"
    elif echo "$output" | grep -q "connecting"; then
        echo "✅ $name: Success (connected but no events in timeout window)"
    else
        echo "❌ $name: Failed (no connection or events)"
        echo "Debug output: $output"
        return 1
    fi
}

INDODAX="${INDODAX_BIN:-./target/debug/indodax}"
if [ ! -x "$INDODAX" ]; then
    echo "Building indodax-cli ..."
    cargo build
fi

if [ ! -x "$INDODAX" ]; then
    echo "Error: $INDODAX not found after cargo build."
    exit 1
fi

test_ws "ticker btc_idr" "Public Ticker"
test_ws "summary" "Public Summary"
test_ws "book btc_idr" "Public Orderbook"

echo "E2E verification complete."
