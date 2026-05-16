# TODO

Issues identified during comprehensive code/business/UI/UX review.

---

## Completed (this session)

- [x] All 22 issues from previous TODO.md — verified fixed in source code
- [x] **Checklist: Build** — `cargo build` passes with Rust 1.95.0
- [x] **Checklist: Tests** — All 300 tests pass
- [x] **Checklist: Clippy** — 14 auto-fixable warnings resolved; remaining 22 are minor (mostly `result_large_err` in `IndodaxError` enum)

---

## New Issues Found

### High Priority

- [ ] **MCP paper state TOCTOU race condition** — `src/mcp/tools/paper.rs:109-117,148-200,208-222,224-240,254-280,282-307`: All MCP paper trade handlers load-modify-save with the `Mutex<IndodaxConfig>` lock released between load and save. Two concurrent requests can interleave, causing silent paper state corruption. The entire read-modify-write cycle must be atomic under the lock.

- [ ] **`alert_triggered()` can panic on `unwrap()` of corrupted alerts file** — `src/commands/alert.rs:493`: `alert.triggered_at.unwrap()` panics if a user manually edits `alerts.json` and sets `"triggered_at": null` while `"status": "triggered"`. Same issue at line 448. Fix: use `unwrap_or(0)` or `match` with fallback.

### Medium Priority

- [ ] **`paper_balance_value()` uses hardcoded stablecoin list, inconsistent with `is_fiat_or_stable()`** — `src/commands/paper.rs:1153-1167`: Only `"idr" | "usdt" | "usdc"` get 2-decimal rounding, while `is_fiat_or_stable()` (line 314) correctly includes `dai`, `busd`, `pax`, `usde`, `gusd`, `tusd`. MCP `paper_balance` formats DAI/BUSD with 8 decimals instead of 2.

- [ ] **Real account `balance` shows raw f64 values with excessive precision** — `src/commands/account.rs:141`: Uses `amount.to_string()` for all currencies. IDR balances show `100000.12345678` instead of `100000.12`. Paper trading has `format_balance()` but real account commands don't.

- [ ] **MCP `paper_fill` cannot use `--fetch` (live price fetching)** — `src/mcp/tools/paper.rs:271`: Hardcodes `fetch: false` when calling `paper_fill()`. Tool definition in `mod.rs:418-423` doesn't expose a `fetch` parameter. MCP users cannot auto-fetch live prices while CLI users can via `paper fill --fetch`.

- [ ] **`paper_status` displays `total_fees_paid` with fixed 8 decimal places** — `src/commands/paper.rs:978`: Uses `{:.8}` regardless of currency. IDR-denominated fees show e.g. `2600.00000000`. Should use `format_balance()`.

- [ ] **Account `info` shows raw balance values without currency-aware formatting** — `src/commands/account.rs:109-116`: Uses `helpers::value_to_string(v)` which calls `serde_json::Value::to_string()` — excessive precision for IDR/large balances.

### Low Priority / Polish

- [ ] **Orderbook depth hardcoded to 20 levels with no user control** — `src/commands/market.rs:180,187`: Both buy/sell sides use `.take(20)`. No `--levels` or `--depth` parameter exposed to user.

- [ ] **MCP `cancel_all_orders` lacks warning when no pair filter specified** — `src/mcp/tools/trade.rs:222-238`: CLI version shows interactive confirmation + requires `--force` in non-interactive mode. MCP silently proceeds with no explicit messaging about global scope.

- [ ] **Paper state stored in config.toml (mixing config with runtime data)** — `src/commands/paper.rs:89-93`: Paper balances/orders/trade count serialized into `IndodaxConfig.paper_balances` and saved alongside credentials in `config.toml`. Sharing config leaks paper data; paper state cannot be backed up independently.

- [ ] **`send_with_retry` doesn't honor `Retry-After` header on 429 responses** — `src/client.rs:337-397`: Uses hardcoded exponential backoff starting at 500ms. Ignores any `Retry-After` header, potentially exacerbating rate-limiting.

- [ ] **`refund_and_cancel` computes unused `refund` variable for sell orders** — `src/commands/paper.rs:1276`: `let refund = order.price * order.remaining;` computed for all sides but only used for buys. Dead code for sell cancels — minor clarity issue.

- [ ] **`paper_watch` doesn't save state when no fills occur** — `src/commands/paper.rs:1002-1031`: State loaded fresh each iteration, only saved if `filled > 0`. If MCP tools modify paper state between watch iterations, those changes are discarded on next iteration.

- [ ] **`test_paper_check_fills_fetch_not_available` makes real network calls** — `src/commands/paper.rs:1811-1821`: Creates `IndodaxClient::new(None)` and calls with `fetch: true`, triggering real HTTP GET requests. Tests should use mocked clients if run in offline environments.

- [ ] **No upper-bound validation for OHLC `from`/`to` timestamps** — `src/commands/market.rs:227-236`, `src/mcp/tools/market.rs:154-160`: Millisecond warning exists but no rejection. API returns empty data for out-of-range timestamps without meaningful error.

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
