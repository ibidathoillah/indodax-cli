#!/bin/bash
# Minimal Balance E2E Test Script for Indodax CLI
# This script performs safe, read-only and minimal-amount operations to verify CLI health.

set -e

echo "--- [E2E] Starting Minimal Risk Tests ---"

# 1. Public API Verification
echo "[1/4] Checking Public API (Ticker)..."
./target/release/indodax market ticker --pair btc_idr > /dev/null
echo "✅ Public API OK"

# 2. Authentication & V1 API Verification (Read-Only)
echo "[2/4] Checking Private V1 API (Balance)..."
./target/release/indodax account balance > /dev/null
echo "✅ Private V1 (Auth) OK"

# 3. API V2 Verification (Read-Only)
echo "[3/4] Checking Private V2 API (Order History)..."
./target/release/indodax account order-history --symbol btc_idr --limit 10 > /dev/null
echo "✅ Private V2 (History) OK"

# 4. Minimal Trade Verification (Optional/Manual)
# Note: Indodax minimum buy/sell is 10,000 IDR. 
# We only suggest running this manually if balance is available.
echo "[4/4] Minimal Trade Verification (Instructions):"
echo "   To verify execution, run: indodax trade buy --pair btc_idr --idr 10000"
echo "   Then immediately cancel:  indodax trade cancel-all --pair btc_idr --force"

echo "--- [E2E] Minimal Tests Completed Successfully ---"
