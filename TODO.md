# TODO

## Completed (current session)

### High Priority

- [x] **MCP `handle_withdraw` `request_id` still hardcoded to `"1"`** — `src/mcp/tools/funding.rs`: Fixed to use `chrono::Utc::now().timestamp_millis()`, aligning MCP handler with CLI fix.

- [x] **MCP `handle_sell_order` ignores `order_type` parameter** — `src/mcp/tools/trade.rs`: Now respects `order_type` parameter. If `order_type` is `"market"`, treats as market order; if `"limit"`, treats as limit order; otherwise falls back to `price.is_none()` behavior.

- [x] **Balance check precedes positivity validation** — `src/commands/trade.rs`: Reordered positivity checks (`<= 0.0`) to occur before balance sufficiency checks in both `place_buy_order` and `place_sell_order`.

- [x] **MCP `paper_buy`/`paper_sell` amount defaults to 0.0 instead of explicit error** — `src/mcp/tools/mod.rs`: Amount is now validated at the MCP dispatch layer with a clear error message if missing or non-positive.

### Medium Priority

- [x] **MCP withdrawal missing amount/address/currency validation** — `src/mcp/tools/funding.rs`: Added validation for `amount > 0 && is_finite()`, `address` non-empty, and `currency` non-empty in `handle_withdraw`.

- [x] **`paper_fill(all=true)` doesn't validate fill_price finiteness** — `src/commands/paper.rs`: Added `is_finite()` check in the batch-fill path before calling `execute_fill`, matching the single-order path.

- [x] **MCP `get_num` for `order_id` truncates non-integers silently** — `src/mcp/tools/mod.rs`: Added `fract() != 0.0` validation for `order_id` in `paper_cancel`, `cancel_order`, and `get_order` handlers.

- [x] **WebSocket `fetch_public_ws_token` signature exposes `reqwest::Client` internals** — `src/commands/websocket.rs`: Changed to accept `&IndodaxClient` and use `client.http_client()` internally.

### Low Priority / Polish

- [x] **MCP `paper_init` verbose IDR/BTC output via format_balance** — `src/mcp/tools/paper.rs` and `src/commands/paper.rs`: Made `format_balance` public and used it in MCP handler for consistent formatting with CLI.

- [x] **MCP `order_history`/`trade_history` `limit` cast from f64 to u32 truncates** — `src/mcp/tools/account.rs`: Added validation for whole-number and positivity before casting, with clear error messages.

- [x] **`cancel_all_orders` uses `value_to_string` for order_type from API** — `src/commands/trade.rs`: Changed to use `as_str()` with fallback to empty string instead of `value_to_string` which can produce multi-word strings for object values.

## New Issues (current review)

### High Priority

- [x] **CLI `funding withdraw` lacks input validation** — `src/commands/funding.rs:84-134`: Added validation for `currency`, `amount`, and `address` to provide clear error messages.

- [ ] **`paper_fill all=true` accumulates `_skipped` counter but never reports it** — `src/commands/paper.rs:519-520`: The `_skipped` variable is incremented when an order goes missing during batch fill but is never included in output, silently hiding partial failures.

### Medium Priority

- [ ] **`account info` joins all balances into a single line** — `src/commands/account.rs:92-99`: Balances are joined with `"  "` into one row instead of individual rows. With many currencies this becomes unreadable in table mode. Should show each balance as its own row like `balance` subcommand does.

- [ ] **`trans_history` shows only one transaction type at a time** — `src/commands/account.rs:303-306`: Uses `or_else` chaining on `get("withdraw")`, `get("deposit")`, and `get("transactions")`, so if multiple types are present in the response, only the first one checked is displayed.

- [ ] **`cancel_all_orders` doesn't validate pair filter scope** — `src/commands/trade.rs:275-339`: Cancel all with no `--pair` filter cancels ALL open orders across all pairs. No confirmation prompt or `--dry-run` flag exists to preview which orders would be cancelled.

### Low Priority / Polish

- [x] **`funding.rs:98` uses `format!` for timestamp when `.to_string()` suffices** — `src/commands/funding.rs:98`: Changed to `.to_string()` for consistency.

- [ ] **MCP tool validation errors all report `mcp_error` type** — `src/mcp/tools/mod.rs:136`: Validation errors in MCP handlers (missing params, non-positive amounts, etc.) all use `error_type: "mcp_error"` instead of more specific types that would help AI agents distinguish validation failures from system errors.

- [ ] **WebSocket `CommandOutput` discards all streamed data** — `src/commands/websocket.rs:202`: After disconnect, returns `{"status": "disconnected"}` regardless of the data received during the session. In JSON mode, data is printed directly to stdout, making it impossible for callers to distinguish streaming events from the final result.

## Known / Intentional

- **Buy hardcodes `idr` param name** — `trade.rs` always uses `"idr"` for buy amount. Indodax only supports IDR-quoted pairs for buys.
- **Paper market buy price unknown** — Balance sufficiency for market buys is checked at fill time, not at order placement. Intentional design.
- **JSON vs table output routing** — JSON to stdout, errors to stderr. Scripts parse JSON; humans see errors. Intentional.
- **`cancel_all_orders` partial failure** — Individual API cancels cannot be rolled back. Design limitation of the API.
- **Paper topup positive-only** — Negative topups rejected explicitly. Intentional.
- **MCP withdraw does not expose callback_url** — Deliberately excluded from the MCP `withdraw` tool for safety.
