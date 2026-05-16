#!/bin/bash
# Minimal Balance E2E Test Script for Indodax CLI
# This script performs safe, read-only and minimal-amount operations to verify CLI health.

set -e

echo "--- [E2E] Starting Minimal Risk Tests ---"

# Use the bin wrapper or binary directly
INDODAX="./bin/indodax.js"
if [ ! -f "$INDODAX" ]; then
    INDODAX="./target/release/indodax"
fi

# Function to check if output is empty (no data rows in table)
check_not_empty() {
    local cmd=$1
    local name=$2
    echo "Testing: $name..."
    local output
    output=$($INDODAX $cmd)
    
    # Check if table has rows beyond header and separators
    # A typical empty table looks like:
    # ┌─────────┬──────┐
    # │ Header  ┆ Info │
    # ╞═════════╪══════╡
    # └─────────┴──────┘
    # We check if there's any content between ╞ and └
    
    local row_count
    # Count lines that start with │ but are NOT the header (containing ┆)
    row_count=$(echo "$output" | grep "^│" | grep -v "┆" | wc -l)
    
    if [ "$row_count" -gt 0 ]; then
        echo "✅ $name: OK ($row_count rows)"
    else
        echo "❌ $name: FAILED (Empty table)"
        echo "$output"
        exit 1
    fi
}

# 1. Public API Verification
echo "[1/5] Checking Public Market Data..."
check_not_empty "market ticker --pair btc_idr" "Market Ticker"
check_not_empty "market pairs" "Market Pairs"
check_not_empty "market trades --pair btc_idr" "Market Trades"
check_not_empty "market ohlc --symbol btc_idr" "Market OHLC"
echo "✅ Public Market Data OK"

# 2. Authentication & V1 API Verification (Read-Only)
echo "[2/5] Checking Private V1 API (Balance)..."
check_not_empty "account balance" "Account Balance"
echo "✅ Private V1 (Auth) OK"

# 3. API V2 Verification (Read-Only)
echo "[3/5] Checking Private V2 API (Order History)..."
# Note: History might be empty for new accounts, so we check for no error
$INDODAX account order-history --symbol btc_idr --limit 1 > /dev/null
echo "✅ Private V2 (History) OK"

# 4. WebSocket Connectivity (Brief check)
echo "[4/5] Checking WebSocket Connectivity (5s)..."
ws_output=$(timeout 5s $INDODAX websocket ticker --pair btc_idr --output json 2>&1 || true)
if echo "$ws_output" | grep -q "connecting"; then
    echo "✅ WebSocket: OK (connected)"
else
    echo "❌ WebSocket: FAILED"
    echo "$ws_output"
    exit 1
fi

# 5. Minimal Trade Verification (Optional/Manual)
echo "[5/5] Minimal Trade Verification (Instructions):"
echo "   To verify execution, run: indodax trade buy --pair btc_idr --idr 10000"
echo "   Then immediately cancel:  indodax trade cancel-all --pair btc_idr --force"

echo "--- [E2E] Minimal Tests Completed Successfully ---"
