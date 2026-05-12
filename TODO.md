# TODO

## Completed (all prior sessions)

- [x] `trade_count` inconsistent between CLI and WASM
- [x] Nonce collision under concurrent requests
- [x] `sign_v2` double-appends timestamp/recvWindow
- [x] Non-paper errors lose category/retryability through `anyhow` conversion
- [x] Rate limiter only refills when tokens reach zero
- [x] `place_sell_order` always includes price even for market orders
- [x] `fetch_market_prices` fetches sequentially, not concurrently
- [x] Funding callback manual mode is non-functional
- [x] MCP missing required parameters silently default to 0/""
- [x] Rate limiter sub-second busy-wait loop
- [x] JSON output mode silently drops addendum
- [x] `cancel_all_orders` discards per-order error details
- [x] Blocking `std::io::stdin()` in async context
- [x] `sign_v1` has unused `_use_timestamp` parameter
- [x] `resolve_public_ws_token` is a trivial dead-code wrapper
- [x] `report_error` uses `unwrap()` on JSON serialization
- [x] `auth test` creates unnecessary `IndodaxClient`
- [x] Inline URL instead of `PRIVATE_V1_URL` constant
- [x] WASM `fetch_public_ws_token` creates new `reqwest::Client` each call
- [x] MCP paper handlers swallow save errors
- [x] `cancel_all_orders` silently swallows individual failures
- [x] Success and error JSON have different envelope formats
- [x] `paper_status` JSON output differs between CLI and MCP
- [x] All balances formatted with 8 decimal places regardless of currency
- [x] `paper topup` accepts negative amounts
- [x] Double trailing newline in table output
- [x] OHLC help text doesn't mention 24h default for `--from`
- [x] `utility_execute` errors wrapped in `IndodaxError::Other`
- [x] `sign_v1` caller still passed unused `false` argument
- [x] `sell` command `price` required even for market orders
- [x] `json.rs` duplicate import
- [x] `paper_balance` / `paper_history` / `paper_status` empty addendums
- [x] MCP `sell_order` price made optional
- [x] MCP `handle_sell_order` only sends price for limit orders
- [x] MCP buy/sell balance pre-check added
- [x] `paper_fill(all=true)` checks price against side
- [x] `paper_fill` no longer panics on vanished order
- [x] Account order side detection is case-insensitive
- [x] WASM `execute_fill` buy side checks fee sufficiency
- [x] WASM `execute_fill` / `cancel_order` safe pair splitting
- [x] WASM default balances aligned with CLI
- [x] Paper buy/sell `price` `Option<f64>`
- [x] `priv_get`/`first_of` unified
- [x] Blocking `std::io::stdin()` in funding callback
- [x] `auth.rs` UNIX_EPOCH unwrap replaced
- [x] `errors.rs` duplicate category strings
- [x] `paper.rs` unnecessary clone in default()
- [x] `market.rs` `first_of()` inefficient string allocation
- [x] `config.rs` TOCTOU race in file permissions
- [x] `paper.rs:283` balance sort parse failures
- [x] Rate limiter lock optimization
- [x] Telemetry module removed
- [x] Buy/sell UX consistency
- [x] Cancel `order_type` help text
- [x] Buy `idr` short flag
- [x] Cancel short flags consistency
- [x] Funding `address` help text
- [x] OHLC default symbol
- [x] JSON success envelope
- [x] Funding `auto_ok` defaults to `false`
- [x] Verified: `serial_test` is used; `IndodaxClient` import needed

### Completed (this session)

#### MCP Improvements
- [x] **MCP `getInfo` consolidation** — Added `IndodaxMcp::get_account_info` helper to `mod.rs` and updated `account.rs` and `trade.rs` to use it, reducing redundant code and API interaction logic.
- [x] **MCP trade validation** — Added validation to `handle_buy_order` and `handle_sell_order` to ensure `idr`, `amount`, and `price` are positive and finite, bringing MCP tools to parity with CLI command robustness.

#### Refactoring
- [x] **MCP `tools.rs` split into sub-modules** — `src/mcp/tools/` directory with `mod.rs`, `market.rs`, `account.rs`, `trade.rs`, `funding.rs`, `paper.rs`, `auth.rs`. The 1489-line `tools.rs` is gone. `IndodaxMcp` struct and common helpers stay in `tools/mod.rs`; each group has its own tool definitions and handler impls. ServerHandler dispatch remains in `mod.rs`.
- [x] **CLI/MCP paper duplication reduced** — Added shared `paper_balance_value`, `paper_orders_value`, `paper_history_value`, `paper_status_value` helpers in `commands/paper.rs` that return `serde_json::Value`. MCP handlers now call these instead of duplicating formatting logic.
- [x] **`trade.rs` repeated `getInfo`+`HashMap::new()` consolidated** — Extracted `get_account_info()` helper in `trade.rs:5-8`, used by both `place_buy_order` and `place_sell_order`.
- [x] **`trade.rs` type mismatch fixed** — `get_account_info` return type alignment with `anyhow::Result` via `?` operator.

#### High Priority Fixes
- [x] **Nonce race condition** — `auth.rs:next_nonce()`: Changed from non-atomic load/store to `compare_exchange` loop with `Acquire`/`Release` ordering. Concurrent calls now safely serialize without collision.
- [x] **`private_get_v2` bypasses rate limiter & retry** — `client.rs:280-327`: Changed from `self.http.get().send()` to `self.send_with_retry(req)`. Now respects rate limiter token acquisition and retries on failure.
- [x] **`f64` for financial balance comparisons** — `trade.rs`, `paper.rs`: Added `BALANCE_EPSILON` constant (`1e-8`) for floating-point comparisons. All balance sufficiency checks use `balance + EPSILON < required` to avoid precision-edge rejection.
- [x] **Paper market buy check** — `paper.rs:324-331`: Market buy price is unknown at order placement, so only positive quote balance is required. The actual sufficiency check happens at fill time in `execute_fill`.
- [x] **Paper order `duration_since` unwrap** — `paper.rs:353-356`: Changed `.unwrap()` to `.unwrap_or_default()` to prevent panic on system clock rollback.
- [x] **HTTP client `.expect()`** — `client.rs:new()`: Changed return type to `Result<Self, IndodaxError>`. Propagates TLS/client build failures instead of panicking.
- [x] **HMAC key `.expect()`** — `auth.rs:hmac_sha512()`: Changed `Hmac::new_from_slice().expect()` to `.map_err()?`. Propagates key initialization errors through `IndodaxError`.
- [x] **Rate limiter refill race** — `client.rs:44-76`: Refill now uses `compare_exchange` loop instead of non-atomic `load()+store()`. Multiple concurrent `acquire()` calls no longer lose token increments.
- [x] **WebSocket output interleaves with JSON mode** — `websocket.rs`: Route all status events (connecting, connected, authenticated, etc.) to `stderr` in JSON mode. `stdout` is now a pure JSON event stream.
- [x] **Graceful WebSocket shutdown** — `websocket.rs`: Added Ctrl+C handler using `tokio::select!`. Connections now close cleanly with a WebSocket `Close` frame when interrupted.

#### Medium Priority Fixes
- [x] **Withdraw `request_id` hardcoded to `"1"`** — `funding.rs:98`: Changed from `"1".to_string()` to `chrono::Utc::now().timestamp_millis()` for unique per-request IDs.
- [x] **Silent error suppression** — `websocket.rs:114`: Added `tracing::warn!()` logging for JSON parse errors. No longer silently drops malformed messages.
- [x] **WebSocket ANSI escape codes unconditionally** — `websocket.rs:280,283`, `websocket.rs:315`: Made `\r\x1b[K` clear-line sequences conditional on `std::io::stdout().is_terminal()`. Terminal gets inline-updating display; piped output gets clean newline-separated lines.
- [x] **Cancel `order_type` not validated** — `trade.rs:35-38`: Added validation rejecting anything other than "buy" or "sell" (case-insensitive). Invalid values produce clear error before API call.
- [x] **`flatten_json_to_table` assumes uniform array schema** — `helpers.rs:23-35`: Now collects *all* unique keys from *all* array elements instead of deriving headers from the first element only. Heterogeneous schemas no longer lose columns.
- [x] **`f64` sort in paper_balance** — `paper.rs:284-289`: Added `.filter(|v| v.is_finite())` to exclude NaN/inf values from sort comparison. Falls back to `Ordering::Equal` for non-finite values.
- [x] **Missing zero/negative price validation** — `trade.rs`: Added validation rejecting `price <= 0.0` and `amount <= 0.0` / `idr <= 0.0` in both buy and sell paths.
- [x] **`paper topup` precision fix** — `commands/paper.rs`: Now uses `format_balance()` helper for addendum messages. BTC/crypto topups show 8 decimals instead of being truncated to 2.
- [x] **Funding callback server stdout cleanup** — `funding.rs`: All server status/interaction messages routed to `stderr`. Incoming callback bodies are emitted as structured JSON on `stdout` when in JSON mode.
- [x] **`paper_fill` NaN validation** — `paper.rs`: Added `f64::is_finite()` check on fill price before executing simulated trades.
- [x] **Config fallback warning** — `config.rs`: Added `eprintln!` warning and explicit `std::env::current_dir()` fallback when `dirs::config_dir()` is unavailable.

#### Low Priority Fixes
- [x] **Redundant empty history entry** — `utility.rs:123`: Removed `rl.add_history_entry("")` call that added empty string to readline history after every command.
- [x] **Order ID string sort fallback undefined** — `account.rs:200`: Changed `unwrap_or(0)` to fallback string comparison via `match` on `parse::<u64>().ok()`. Non-numeric order IDs now sort deterministically lexicographically.
- [x] **WebSocket auth `id` comparison fragile** — `websocket.rs:117-120,141`: Changed `== Some(&Value::Number(1.into()))` to `.and_then(|v| v.as_i64()) == Some(1)`. Handles both integer (`1`) and float (`1.0`) JSON representations.

## Known / Intentional

- **Balance check formatting** — `trade.rs` Buy shows `{:.2}`, Sell shows `{:.8}`. IDR precision is always 2 decimals; crypto precision depends on pair. Intentional.
- **JSON vs table output routing** — `main.rs:103-119`: JSON to stdout, table errors to stderr. Scripts parse JSON on stdout; humans see errors on stderr. Intentional.
- **Buy hardcodes `idr` param name** — `trade.rs:120` always uses `"idr"` for buy amount. Indodax only supports IDR-quoted pairs for buys; selling uses base currency dynamically.
- **Paper market buy price unknown** — `paper.rs:324-331`: Market buy price is unknown at placement time; balance sufficiency check happens at fill time in `execute_fill`. Only positive quote balance is required upfront.
- **`cancel_all_orders` partial failure** — `trade.rs:252-316`: Each cancel is individual via API. Failed orders are collected and reported; earlier successes cannot be rolled back. Design limitation of the API.

## New Findings (this review)

(All identified findings have been implemented in this session)
