#!/bin/bash
# Minimal Balance E2E Test Script for Indodax CLI
# This script performs safe, read-only and minimal-amount operations to verify CLI health.

set -e

echo "--- [E2E] Starting Minimal Risk Tests ---"

RUN_PUBLIC=true
RUN_PRIVATE=true
RUN_WS=true

if [[ "${1:-}" == "--public" ]]; then
    RUN_PRIVATE=false
    RUN_WS=false
elif [[ "${1:-}" == "--private" ]]; then
    RUN_PUBLIC=false
    RUN_WS=false
elif [[ "${1:-}" == "--ws" ]]; then
    RUN_PUBLIC=false
    RUN_PRIVATE=false
fi

INDODAX="${INDODAX_BIN:-./target/debug/indodax}"
if [ ! -x "$INDODAX" ]; then
    INDODAX="./target/release/indodax"
fi

if [ ! -x "$INDODAX" ]; then
    echo "Building indodax-cli ..."
    cargo build
    INDODAX="./target/debug/indodax"
fi

# Function to check that a command succeeds and returns output.
check_not_empty() {
    local cmd=$1
    local name=$2
    echo "Testing: $name..."
    local output
    output=$($INDODAX $cmd)
    
    if [ -n "$output" ]; then
        echo "✅ $name: OK"
    else
        echo "❌ $name: FAILED (Empty output)"
        echo "$output"
        exit 1
    fi
}

if $RUN_PUBLIC; then
    # 1. Public API Verification
    echo "[1/5] Checking Public Market Data..."
    check_not_empty "ticker btc_idr" "Market Ticker"
    check_not_empty "pairs" "Market Pairs"
    check_not_empty "trades btc_idr" "Market Trades"
    check_not_empty "ohlc --pair btc_idr" "Market OHLC"
    echo "✅ Public Market Data OK"
fi

if $RUN_PRIVATE; then
    if $INDODAX auth test >/dev/null 2>&1; then
        # 2. Authentication & V1 API Verification (Read-Only)
        echo "[2/5] Checking Private V1 API (Balance)..."
        check_not_empty "balance" "Account Balance"
        echo "✅ Private V1 (Auth) OK"

        # 3. API V2 Verification (Read-Only)
        echo "[3/5] Checking Private V2 API (Order History)..."
        # Note: History might be empty for new accounts, so we check for no error
        $INDODAX trades-history btc_idr --limit 1 > /dev/null
        echo "✅ Private V2 (History) OK"
    else
        echo "[2/5] Private API: SKIP (credentials unavailable or invalid)"
        echo "[3/5] Private V2 API: SKIP (credentials unavailable or invalid)"
    fi
fi

if $RUN_WS; then
    # 4. WebSocket Connectivity (Brief check)
    echo "[4/5] Checking WebSocket Connectivity (5s)..."
    ws_output=$(timeout 5s $INDODAX --output json ws ticker btc_idr 2>&1 || true)
    if echo "$ws_output" | grep -q "connecting"; then
        echo "✅ WebSocket: OK (connected)"
    else
        echo "❌ WebSocket: FAILED"
        echo "$ws_output"
        exit 1
    fi
fi

# 5. Minimal Trade Verification (Optional/Manual)
echo "[5/5] Minimal Trade Verification (Instructions):"
echo "   To verify execution, run: indodax order buy --pair btc_idr --idr 10000"
echo "   Then immediately cancel:  indodax --yes order cancel-all --pair btc_idr"

echo "--- [E2E] Minimal Tests Completed Successfully ---"
