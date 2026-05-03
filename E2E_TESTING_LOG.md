# E2E Testing Log & Bug Fix

**Date:** 2026-05-03

## Summary

This document logs the End-to-End (E2E) testing process for the `indodax-cli` tool, including the discovery, investigation, and resolution of a critical bug in the `trade` command.

---

## 1. Initial E2E Test Results

### ✅ Successful Commands
- `market server-time`: OK
- `market ticker`: OK
- `account balance`: OK
- `auth test`: OK
- `funding serve-callback`: OK (Tested locally)

### ❌ Failed Command: `trade buy`

- **Initial Issue:** The `indodax trade buy` command consistently failed with the error `Minimum order 10,000 IDR`, even for calculated order values greater than 10,000 IDR.
- **Example Failing Command:** `indodax trade buy --pair btc_idr --price 1355947000 --amount 0.00001106`

---

## 2. Bug Investigation & Fix

### Root Cause Analysis
- Research into the Indodax API V1 documentation revealed that the `trade` endpoint for `buy` orders supports a parameter named **`idr`**.
- This parameter allows placing an order based on the total Rupiah amount to be spent, which is more robust than calculating the base currency `amount` on the client-side and avoids floating-point precision/rounding issues.
- The original implementation, which used `--amount`, was causing the bug.

### Implementation of the Fix

1.  **Refactored `TradeCommand::Buy`:**
    - The `buy` command in `src/commands/trade.rs` was modified.
    - The `--amount` flag was removed.
    - A new, mandatory **`--idr`** flag was added to specify the total purchase amount in IDR.
    - The `--price` flag was made optional. If omitted, the command places a `market` order.

2.  **Updated `place_buy_order` function:**
    - The logic was rewritten to construct the API payload using the `idr` parameter instead of the base currency amount (`btc`).

---

## 3. Final E2E Test (Post-Fix)

- **Test Command:** `indodax trade buy --pair btc_idr --idr 15000` (Market Buy)
- **Result:** ✅ **SUCCESS**
- **Outcome:**
    - The order was successfully placed and executed.
    - **IDR Spent:** 14,967 (+ 30 fee)
    - **BTC Received:** 0.00001103
    - **Balance Update:** The user's IDR balance decreased and BTC balance increased correctly, confirming the trade was successful.

## Conclusion

The bug related to the `trade buy` command has been **resolved**. The CLI is now more robust by using the native `idr` parameter provided by the Indodax API. All other tested E2E scenarios passed successfully.
