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

#### Refactoring
- [x] **MCP `tools.rs` split into sub-modules** — `src/mcp/tools/` directory with `mod.rs`, `market.rs`, `account.rs`, `trade.rs`, `funding.rs`, `paper.rs`, `auth.rs`. The 1489-line `tools.rs` is gone. `IndodaxMcp` struct and common helpers stay in `tools/mod.rs`; each group has its own tool definitions and handler impls. ServerHandler dispatch remains in `mod.rs`.
- [x] **CLI/MCP paper duplication reduced** — Added shared `paper_balance_value`, `paper_orders_value`, `paper_history_value`, `paper_status_value` helpers in `commands/paper.rs` that return `serde_json::Value`. MCP handlers now call these instead of duplicating formatting logic.
- [x] **`trade.rs` repeated `getInfo`+`HashMap::new()` consolidated** — Extracted `get_account_info()` helper in `trade.rs:5-8`, used by both `place_buy_order` and `place_sell_order`.
- [x] **`trade.rs` type mismatch fixed** — `get_account_info` return type alignment with `anyhow::Result` via `?` operator.

#### From comprehensive code/business/UI-UX review
- [x] **MCP `_required` param unused** — `str_param`/`num_param` accept `_required` but never use it. Consider removing or implementing required field enforcement in JSON schema.

## Known / Intentional

- **Balance check formatting** — `trade.rs` Buy shows `{:.2}`, Sell shows `{:.8}`. IDR precision is always 2 decimals; crypto precision depends on pair. Intentional.
- **JSON vs table output routing** — `main.rs:103-119`: JSON to stdout, table errors to stderr. Scripts parse JSON on stdout; humans see errors on stderr. Intentional.
- **Buy hardcodes `idr` param name** — `trade.rs:120` always uses `"idr"` for buy amount. Indodax only supports IDR-quoted pairs for buys; selling uses base currency dynamically.

## New Findings (from review)

### High Priority

- [ ] **Nonce race condition** — `auth.rs:32-41` `next_nonce()` uses non-atomic load/store with `Ordering::Relaxed`. Concurrent calls get same nonce, causing API signature failures. Fix: use `fetch_add` or `compare_exchange`.
- [ ] **`private_get_v2` bypasses rate limiter & retry** — `client.rs:280-327` uses direct `http.get().send()` instead of `send_with_retry()`. No rate limiting or retry on failures for V2 private endpoints.
- [ ] **`f64` for financial balance comparisons** — `trade.rs:106,161`, `paper.rs:323-349` use `f64` with `<`/`<=`. Floating-point precision errors could cause incorrect order placement/rejection. Consider `rust_decimal` or checked operations.
- [ ] **Paper market buy check inadequate** — `paper.rs:324-331` only checks `quote_balance > 0`, not if balance is sufficient for the market buy amount.
- [ ] **Paper order `duration_since` unwrap** — `paper.rs:356-359` uses `.unwrap()` on `duration_since(UNIX_EPOCH)` instead of `.unwrap_or_default()`. Panics on clock rollback.
- [ ] **HTTP client `.expect()`** — `client.rs:103-109` uses `.expect("Failed to create HTTP client")` which panics if TLS init fails. Should propagate error.
- [ ] **HMAC key `.expect()`** — `auth.rs:62-67` uses `.expect()` on `Hmac::new_from_slice()`. Should propagate error for invalid key sizes.
- [ ] **Rate limiter refill race** — `client.rs:44-76` token refill logic is non-atomic. Concurrent `acquire()` calls can lose token increments.

### Medium Priority

- [ ] **Withdraw `request_id` hardcoded to `"1"`** — `funding.rs:98` always uses `"1"` for `request_id` when `to_username=true`. Should be unique/incrementing.
- [ ] **Trade commands assume IDR quote** — `trade.rs:118-120` buy always uses `"idr"` param; sell uses `base_currency` from pair split. Non-IDR pairs (e.g. `eth_btc`) may fail.
- [ ] **Silent error suppression** — `websocket.rs:114` JSON parse errors use `Err(_) => continue` with no logging. Several other locations silently swallow errors.
- [ ] **WebSocket output not JSON-mode aware** — `websocket.rs:83-96,124-128` prints status events to stdout even in JSON mode, breaking downstream parsers.
- [ ] **WebSocket ANSI escape codes unconditionally** — `websocket.rs:280,283` uses `\r\x1b[K` regardless of output format/terminal.
- [ ] **Cancel `order_type` not validated** — `trade.rs:35-38` `--order-type` accepts any string; invalid values sent to API produce confusing errors.
- [ ] **Config fallback to CWD** — `config.rs:63-72` falls back to `PathBuf::from(".")` when `dirs::config_dir()` fails. Could pick up configs from untrusted directories.
- [ ] **`flatten_json_to_table` assumes uniform array schema** — `helpers.rs:23-35` takes headers from first array element; heterogeneous schemas produce empty columns.
- [ ] **`cancel_all_orders` partial failure** — `trade.rs:252-316` cancels serially; if a later cancel fails, earlier successes leave partial state.
- [ ] **`f64` sort in paper_balance** — `paper.rs:284-289` uses `partial_cmp` without NaN handling. Should clamp/filter NaN values.
- [ ] **Missing zero/negative price validation** — `trade.rs` no validation for `price=Some(0.0)` or `Some(-100.0)` in trade commands.
- [ ] **MCP `str_param`/`num_param` `_required` unused** — These accept `_required` but ignore it. No schema-level enforcement of required fields.

### Low Priority

- [ ] **Redundant empty history entry** — `utility.rs:123` adds `""` to readline history after every command.
- [ ] **Sequential cancellation could be parallel** — `trade.rs:cancel_all_orders` uses sequential loop instead of `join_all`.
- [ ] **Order ID string sort fallback undefined** — `account.rs:200` sort fallback on parse failure is undefined behavior.
- [ ] **WebSocket auth `id` comparison fragile** — `websocket.rs:117-120` compares `Value::Number(1.into())` which may not match all JSON number representations.
