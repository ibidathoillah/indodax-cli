# TODO

Issues identified during comprehensive code/business/UI/UX review.

---

## Completed (this session)

- [x] **`helpers::format_balance` duplicated in `paper.rs`** — Consolidated `format_balance` and `is_fiat_or_stable` into `helpers.rs`. Added new stablecoins (USDE, GUSD, TUSD) and updated all call sites.
- [x] **WebSocket Reliability Overhaul (v0.1.13)** — Implemented Pings, automatic reconnection, and Private WS rewrite for real-time order/balance updates.
- [x] **Checklist: Build** — `cargo build` passes with Rust 1.95.0
- [x] **Checklist: Tests** — All 296 tests pass
- [x] **Checklist: Clippy** — 14 auto-fixable warnings resolved; remaining 24 are minor (mostly `result_large_err` in `IndodaxError` enum)
- [x] **MCP paper state TOCTOU race condition** — Added `paper_mutex: Arc<Mutex<()>>` to `IndodaxMcp`. All write handlers acquire it before load-modify-save. Save is now done while holding the config lock, eliminating the clone-and-save race.
- [x] **`alert_triggered()` panic on `unwrap()`** — Changed `alert.triggered_at.unwrap()` to `unwrap_or(0)` at both call sites.
- [x] **`paper_balance_value()` hardcoded stablecoin list** — Changed to use `is_fiat_or_stable()` instead of inline match.
- [x] **Real account `balance` excessive precision** — Added `helpers::format_balance()` and use it in `account.rs` for both `balance` and `info` commands.
- [x] **MCP `paper_fill` cannot use `--fetch`** — Added `fetch` parameter to tool definition and pass it to `handle_paper_fill`.
- [x] **`paper_status` fixed 8 decimal places** — Changed `{:.8}` to `{:.2}` for `total_fees_paid`.
- [x] **Account `info` raw balance values** — Use `helpers::format_balance()` for currency-aware display.
- [x] **Orderbook depth hardcoded to 20** — Added `--levels` parameter.
- [x] **MCP `cancel_all_orders` lacks warning** — Added scope warning in result and stderr when no pair filter specified.
- [x] **`send_with_retry` Retry-After header** — Added `Retry-After` header parsing and sleep on 429 responses.
- [x] **`refund_and_cancel` unused `refund` variable** — Moved `let refund` into the buy branch.
- [x] **`paper_watch` doesn't save state when no fills** — Moved `state.save(config)?` before the `if filled > 0` check.
- [x] **Test network dependency** — Changed `test_paper_check_fills_fetch_not_available` to `test_paper_check_fills_no_matching_prices` with inline prices instead of `fetch: true`.
- [x] **OHLC timestamp validation** — Added ms-to-seconds conversion and warning in both CLI and MCP handlers.

---

## New Issues Found

### Medium Priority

- [ ] **`alert.rs:210-218` fragile `unwrap()` chain** — `percent_down.unwrap()` is guarded by `condition_count` but could panic if if-else chain is refactored. Uses `ok_or_else` now but pattern is brittle throughout the function.
- [ ] **`websocket.rs:453` clippy `or_then_unwrap`** — `.or(Some(&val)).unwrap()` pattern should use `.or_else(|| ...).unwrap_or(&val)`.

### Low Priority / Polish

- [ ] **`client.rs:369` double-sleep on 429 with Retry-After** — After honoring `Retry-After`, the next loop iteration still applies exponential backoff. Should reset retry count when Retry-After is used.
- [ ] **`errors.rs` `result_large_err`** — `IndodaxError` is ~136 bytes. Box the `WebSocket` variant to reduce it. Known/Intentional from previous session.
- [ ] **Paper state stored in config.toml (mixing config with runtime data)** — `src/commands/paper.rs:89-93`: Paper balances/orders/trade count serialized into `IndodaxConfig.paper_balances` and saved alongside credentials in `config.toml`. Sharing config leaks paper data; paper state cannot be backed up independently.

## Known / Intentional

- **Buy hardcodes `idr` param name** — `trade.rs` always uses `"idr"` for buy amount. Indodax only supports IDR-quoted pairs for buys.
- **Paper market buy price unknown** — Balance sufficiency for market buys is checked at fill time, not at order placement. Intentional design.
- **JSON vs table output routing** — JSON to stdout, errors to stderr. Scripts parse JSON; humans see errors. Intentional.
- **`cancel_all_orders` partial failure** — Individual API cancels cannot be rolled back. Design limitation of the API.
- **Paper topup positive-only** — Negative topups rejected explicitly. Intentional.
- **MCP withdraw does not expose callback_url** — Deliberately excluded from the MCP `withdraw` tool for safety.
- **Duplicated balance checking in trade commands** — CLI and MCP both fetch `getInfo` and parse balance. Return types differ too much for a clean shared helper.
- **`paper_fill(all=true)` with explicit `fill_price` skips non-matching orders** — Acts as a price filter. Intentional behavior.
- **Rate limiter `as_millis()` minimal truncation guard** — Uses `.min(u128::from(u64::MAX))` before `as u64` cast. Sufficient for all practical purposes.
- **MCP paper response amount for non-IDR orders** — Uses `amount` parameter directly since `place_paper_order` knows the exact amount. Only IDR-based buys need to derive amount from state.
- **`IndodaxError` is large (~136 bytes)** — The `WebSocket` variant boxes `tokio_tungstenite::tungstenite::Error`. Consider boxing other large variants to reduce function return sizes.

## Known / Intentional

- **Buy hardcodes `idr` param name** — `trade.rs` always uses `"idr"` for buy amount. Indodax only supports IDR-quoted pairs for buys.
- **Paper market buy price unknown** — Balance sufficiency for market buys is checked at fill time, not at order placement. Intentional design.
- **JSON vs table output routing** — JSON to stdout, errors to stderr. Scripts parse JSON; humans see errors. Intentional.
- **`cancel_all_orders` partial failure** — Individual API cancels cannot be rolled back. Design limitation of the API.
- **Paper topup positive-only** — Negative topups rejected explicitly. Intentional.
- **MCP withdraw does not expose callback_url** — Deliberately excluded from the MCP `withdraw` tool for safety.
- **Duplicated balance checking in trade commands** — CLI and MCP both fetch `getInfo` and parse balance. Return types differ too much for a clean shared helper.
- **`paper_fill(all=true)` with explicit `fill_price` skips non-matching orders** — Acts as a price filter. Intentional behavior.
- **Rate limiter `as_millis()` minimal truncation guard** — Uses `.min(u128::from(u64::MAX))` before `as u64` cast. Sufficient for all practical purposes.
- **MCP paper response amount for non-IDR orders** — Uses `amount` parameter directly since `place_paper_order` knows the exact amount. Only IDR-based buys need to derive amount from state.
- **`IndodaxError` is large (~136 bytes)** — The `WebSocket` variant boxes `tokio_tungstenite::tungstenite::Error`. Consider boxing other large variants to reduce function return sizes.
