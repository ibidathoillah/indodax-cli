# TODO

## Completed (current session)

### High Priority

- [x] **MCP `handle_withdraw` `request_id` still hardcoded to `"1"`** — `src/mcp/tools/funding.rs`: Fixed to use `chrono::Utc::now().timestamp_millis()`, aligning MCP handler with CLI fix.

- [x] **MCP `handle_sell_order` ignores `order_type` parameter** — `src/mcp/tools/trade.rs`: Now respects `order_type` parameter. If `order_type` is `"market"`, treats as market order; if `"limit"`, treats as limit order (requires price); otherwise returns validation error.

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

## Completed (this review session)

### High Priority

- [x] **Paper `place_paper_order_idr` market buy with no price gives free crypto** — `src/commands/paper.rs:441-458`: Rejected market buys via `--idr` unless a limit price is specified.

- [x] **Paper `place_paper_order_idr` division by zero on insufficient balance path** — `src/commands/paper.rs:451-454`: Eliminated by rejecting market buys via `--idr` without a limit price.

### Medium Priority

- [x] **`countdown_cancel_all` bypasses rate limiter and retry logic** — `src/client.rs`: Now uses `send_with_retry()` which calls `rate_limiter.acquire()` and retries on failure.

- [x] **`generate_ws_token` bypasses rate limiter and retry logic** — `src/client.rs`: Changed to use `send_with_retry()` for rate limiting and retry protection.

- [x] **Rate limiter `acquire` has racy refill logic** — `src/client.rs`: Simplified to single mutex-protected state, eliminating the race between atomic load and mutex lock.

### Low Priority / Polish

- [x] **`cancel_all_orders` fetches all orders then cancels one-by-one with no progress indicator** — `src/commands/trade.rs`: Added `indicatif` progress bar showing cancellation progress.

- [x] **`serve_callback` stdin read silently swallows errors** — `src/commands/funding.rs`: Added `eprintln!` warning on both stdin I/O errors and spawn_blocking failures.

- [x] **Paper P&L on migrated state shows inflated profits** — `src/commands/paper.rs`: When loading state with missing `initial_balances`, snapshots current balances and warns user.

- [x] **MCP `handle_paper_init` duplicates default balance constants** — `src/mcp/tools/paper.rs`: Now imports and uses `DEFAULT_BALANCE_IDR`/`DEFAULT_BALANCE_BTC` from `commands/paper.rs`.

- [x] **WebSocket ticker timestamp overflow from u64 to i64** — `src/commands/websocket.rs:230-234`: Added saturating conversion via `ts.min(i64::MAX as u64) as i64`.

## Completed (this review session, continued)

### High Priority

- [x] **MCP `sell_order` `order_type` semantics mismatch** — `src/mcp/tools/trade.rs:149-165`: Changed `_` catch-all fallback from `price.is_none()` (silent market order guess) to explicit validation error. Only `"limit"` and `"market"` are now accepted.

### Medium Priority

- [x] **Default rate limit 10 RPS → 5 RPS** — `src/client.rs:45`: Changed from 10 to 5. `max(1)` clamp was already present.

### Low Priority / Polish

- [x] **Config directory warning fires for public-only commands** — `src/config.rs:72-88`: Removed warning from `config_path()`/`config_dir()`. Warning is now emitted only in `save()` when a write is actually needed.

- [x] **MCP `buy_order`/`sell_order` unreachable in call_tool dispatch** — `src/mcp/tools/mod.rs`: Had no dispatch entries for `"buy_order"` or `"sell_order"`, making MCP tools visible but unusable. Added dispatch entries with safety `acknowledged` checks and pair normalization.

## New Issues (current review)

### High Priority

### Medium Priority

- [ ] **MCP `cancel_all_orders` tool missing** — No MCP tool equivalent for the CLI's `cancel_all_orders` exists. MCP clients can only cancel individual orders via `cancel_order`. Fix: add tool definition in `trade_tools()` and handler calling existing `cancel_all_orders` logic.

### Low Priority / Polish

- [ ] **MCP `cancel_order` lacks `order_type` validation** — `src/mcp/tools/trade.rs:186`: `handle_cancel_order` passes `order_type` to API without validating it's `"buy"` or `"sell"`. An invalid value gets a raw API error instead of a clear validation error.

- [ ] **`normalize_pair` base-currency stripping for unconventional pairs** — `src/commands/helpers.rs:78-84`: `strip_suffix`-based logic would misinterpret a hypothetical pair like `usdtbtc` as `usdt_btc` (correct) but `idrbtc` as `idr_btc` (unlikely but no guard). No real pairs affected.

- [ ] **Paper trading uses f64 for IDR balances** — `src/commands/paper.rs`: IDR amounts up to billions (e.g., 100M+) use f64, which loses integer precision above 2^53 (~9 quadrillion for smallest fractional). For typical IDR balances this is unlikely to cause visible issues, but could manifest as tiny rounding errors in fee calculations.

## Known / Intentional

- **Buy hardcodes `idr` param name** — `trade.rs` always uses `"idr"` for buy amount. Indodax only supports IDR-quoted pairs for buys.
- **Paper market buy price unknown** — Balance sufficiency for market buys is checked at fill time, not at order placement. Intentional design.
- **JSON vs table output routing** — JSON to stdout, errors to stderr. Scripts parse JSON; humans see errors. Intentional.
- **`cancel_all_orders` partial failure** — Individual API cancels cannot be rolled back. Design limitation of the API.
- **Paper topup positive-only** — Negative topups rejected explicitly. Intentional.
- **MCP withdraw does not expose callback_url** — Deliberately excluded from the MCP `withdraw` tool for safety.
