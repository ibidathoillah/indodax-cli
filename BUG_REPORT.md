# Bug Report: `trade buy` fails with "Minimum order 10,000 IDR"

**Date:** 2026-05-03

## Description

The `indodax trade buy` command consistently fails with the error message "Minimum order 10,000 IDR", even when the calculated total order value exceeds this minimum threshold. This suggests an issue with either payload formatting, floating-point precision handling, or an undocumented API constraint.

## E2E Test Details

- **Balance:** ~68,129 IDR
- **Pair:** `btc_idr`
- **BTC Price:** ~1,355,947,000 IDR

### Attempts

Multiple attempts were made with increasing amounts, all of which failed with the same error:

1.  **~11,000 IDR Order:**
    -   `indodax trade buy --pair btc_idr --price 1355947000 --amount 0.00000811`
    -   Result: `Error: Minimum order 10,000 IDR`

2.  **~12,000 IDR Order:**
    -   `indodax trade buy --pair btc_idr --price 1355947000 --amount 0.00000885`
    -   Result: `Error: Minimum order 10,000 IDR`

3.  **~15,000 IDR Order:**
    -   `indodax trade buy --pair btc_idr --price 1355947000 --amount 0.00001106`
    -   Result: `Error: Minimum order 10,000 IDR`

## Possible Causes

1.  **Floating-Point Precision:** The Indodax API might be truncating the `amount` value before multiplying it by the `price`, causing the calculated total to fall below 10,000.
2.  **Parameter Encoding:** There might be a subtle issue in how the `amount` is encoded in the `application/x-www-form-urlencoded` payload, especially concerning decimal separators.
3.  **Missing Parameter:** The API might require a different parameter for placing orders by total IDR value (e.g., an equivalent of `cost` or `total`), which is not yet implemented in the CLI. The current implementation only supports ordering by `amount` (in BTC).
4.  **Undocumented API Constraint:** Indodax may have a minimum *amount* of BTC per order, in addition to the minimum total IDR value.

## Recommended Next Steps

1.  **Add Logging:** Implement verbose logging in `src/client.rs` to print the exact `payload` string being sent to the Indodax `/tapi` endpoint.
2.  **Test with `amount_idr`:** Research the Indodax API V2 documentation for an endpoint that allows placing orders by total IDR cost, as this would be more robust and avoid floating-point calculation issues.
3.  **Check BTC Amount Constraint:** Verify if there is a minimum BTC amount for orders on the `btc_idr` pair.
