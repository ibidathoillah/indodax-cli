# TODO

Issues identified during comprehensive code/business/UI/UX review.

---

## Completed (this session)

### High Priority

- [x] **`paper_fill(all=true)` stops processing on first fill error** — Already fixed in PR #29: uses `match execute_fill(...)` collecting errors into a Vec rather than propagating with `?`.

- [x] **`alert_watch` detects triggers but never persists them to disk** — Already fixed in PR #29: sets `alert.status = AlertStatus::Triggered`, `alert.triggered_at`, and calls `save_alerts(&alerts)` at end of watch loop.

- [x] **u64→i64 timestamp overflow at remaining locations** — Added `.min(i64::MAX as u64)` guards at `alert.rs:258` (`from_timestamp_millis`) and `websocket.rs:266` (`from_timestamp`), completing the partial fix from PR #29.

- [x] **`ws_book` displays worst ask instead of best ask** — `websocket.rs:296`: Changed `asks.last()` to `asks.first()`. The best ask is the lowest price (first element in ascending order), not the highest.

- [x] **`paper_fill` single order ignores price condition matching** — `paper.rs:763-773`: Added the same `should_fill` check (`buy: fp <= order_price`, `sell: fp >= order_price`) that `--all` path uses. Single fills now return an error when `fill_price` doesn't match the order's side condition.

- [x] **MCP `handle_paper_trade` shows `amount: 0` for IDR-based buys** — `mcp/tools/paper.rs:181`: Now reads the actual order amount from `state.orders.last().amount` and includes `idr_spent` in the response for IDR-based purchases.

### Medium Priority

- [x] **Paper buy/sell balance error messages use inconsistent precision** — Already fixed in PR #29: replaced hardcoded `{:.2}`/`{:.8}` with `format_balance()` which dynamically selects precision by currency type.

- [x] **Equity snapshot `calculate_equity` misses pairs not denominated in IDR/BTC/USDT** — The code already handles `_idr`, `_btc`, `_usdt`, and `_eth` pairs. Added a warning at `account.rs:477` via `eprintln!` when a balance cannot be valued through any known pair.

- [x] **`alert_watch` auth/result detection differs from `ws_connect_and_listen`** — Already fixed in PR #29: both now use `id == 1 && result.is_some()` instead of `result.status == "ok"`.

- [x] **`save_alerts` doesn't set restrictive file permissions** — Already fixed in PR #29: uses `0o600` mode on Unix via `OpenOptionsExt::mode`.

- [x] **Duplicate WS token fetching logic** — Already fixed in PR #29: alert.rs now calls `helpers::fetch_public_ws_token(client)` instead of duplicating the fetch logic.

- [x] **WebSocket ping handler sends empty Pong payload** — `websocket.rs:170`: Changed from `Message::Pong(vec![])` to `Message::Pong(data)` to echo the received ping application data per RFC 6455.

- [x] **`save_equity_history` writes without restrictive file permissions** — `account.rs:412`: Now uses `0o600` mode on Unix, consistent with `config.rs`, `alert.rs`, and other sensitive file writes.

### Low Priority / Polish

- [x] **Paper `place_paper_order` sets `total_spent` only for limit buys** — Inherently correct design: market buys have no known price at order placement, so `total_spent` cannot be set. `place_paper_order_idr` always has a known IDR amount.

- [x] **`paper_balance_value` includes raw f64 in JSON** — Already fixed in PR #29: rounds values before serialization using the same precision logic as `format_balance`/`round_balance`.

- [x] **`cancel_all_paper_orders` only reports IDs** — Already fixed in PR #29: now returns `(usize, Vec<(u64, String)>)` including error messages, and `paper_cancel_all` displays them in both JSON and human-readable output.

---

## New Issues Found

### High Priority

- [x] **`cancel_all_orders` silently cancels without `--force` in non-interactive mode** — `src/commands/trade.rs:276-283`: Now requires explicit `--force` when stdin is not a terminal and no pair filter is provided.

- [x] **`countdown_cancel_all` duplicates V1 response parsing logic** — `src/client.rs:162-187`: Refactored V1 response parsing into a shared `handle_v1_response` helper, now reused by `private_post_v1` and `countdown_cancel_all`.

### Medium Priority


- [x] **OHLC `from`/`to` parameters accept seconds but users may pass milliseconds** — `src/commands/market.rs:44-48`: No validation to detect millisecond timestamps (`> 1e12`) and warn the user. A ms timestamp would produce data from 54,000 years in the future.

- [x] **`trans_history` merges maps with fragile type detection** — `src/commands/account.rs:320-355`: Merges `withdraw`/`deposit`/`transactions` maps with potential key collisions. Type detection uses `id.contains("withdraw")` which could break if API changes ID format.

- [x] **MCP `handle_sell_order` now errors if `price` is provided when `order_type` is `market`** — `src/mcp/tools/trade.rs:169-172`: Added validation to prevent contradictory parameters.

### Low Priority / Polish

- [x] **`format_ws_price` now uses epsilon for floating-point comparison** — `src/commands/websocket.rs:210`: Large whole numbers may have `fract() != 0.0` due to floating-point representation. Use `(f - f.round()).abs() < f64::EPSILON`.

- [x] **MCP `get_bool` refactored to `get_opt_bool` to distinguish between `false` and absent** — `src/mcp/tools/mod.rs:121-125`: Cannot distinguish between `false` and absent. Safety checks for dangerous operations exist separately but this limits API usability.

- [x] **`round_balance` and `format_balance` now support more stablecoins (DAI, BUSD, etc.) with 2-decimal precision** — `src/commands/paper.rs:530-541`: Only `idr`, `usdt`, `usdc` get 2-decimal rounding. Other fiat-pegged tokens (DAI, BUSD) get 8-decimal rounding.

- [x] **`ws_orders` now collects all events in history regardless of `order_id` presence** — `src/commands/websocket.rs:403-433`: When `result.data` lacks `order_id`, the event is printed but not collected in the `events` vector returned on disconnect.

- [x] **Account `show` now uses proportional masking for API keys for better security on short keys** — `src/commands/auth.rs:90-94`: Shows first 4 chars + `"****"`. For < 8 char keys this reveals >50% of the key.

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
