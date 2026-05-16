# TODO

Issues identified during comprehensive code/business/UI/UX review.

---

## High Priority

- [ ] **`paper_fill(all=true)` stops processing on first fill error** — `src/commands/paper.rs:711`: Uses `?` to propagate `execute_fill` errors immediately inside the loop, leaving remaining orders unchecked. Should collect errors and fill as many as possible (like `paper_check_fills` does at lines 945-948).

- [ ] **`alert_watch` detects triggers but never persists them to disk** — `src/commands/alert.rs:613-646`: When an alert triggers during `watch`, the event is printed to stderr but `alert.status` is never updated and `save_alerts` is never called. The alert remains `Active` in storage. Should save triggered state so `alert triggered` and subsequent `alert list` reflect it. Contrast with `alert_check` (line 399-406) which properly saves.

## Medium Priority

- [ ] **Paper buy/sell balance error messages use inconsistent precision** — `src/commands/paper.rs:360`: `format!("Need {:.2}, have {:.2}")` for quote, but line 371 uses `{:.8}` for base. Should use consistent precision based on currency type (fiat vs crypto).

- [ ] **u64→i64 timestamp casts can silently overflow** — Multiple locations cast `u64` timestamps to `i64` for `DateTime::from_timestamp` (e.g. `alert.rs:243`, `websocket.rs:239`, `account.rs:622`). If timestamp exceeds `i64::MAX` (~292 billion years), it wraps negative. Practically impossible but a `min(i64::MAX as u64)` guard would be defensive, similar to `client.rs:65`.

- [ ] **Equity snapshot `calculate_equity` misses pairs not denominated in IDR/BTC/USDT** — `src/commands/account.rs:462-472`: Only checks `_idr`, `_btc`, `_usdt` pairs. A currency that only trades against e.g. `eth` would be valued at 0. Should attempt to denominate through intermediate pairs or warn about missing rates.

## Low Priority / Polish

- [ ] **Paper `place_paper_order` sets `total_spent` only for limit buys; `place_paper_order_idr` always sets it** — Inconsistent: regular buys (`paper.rs:397`) only set `total_spent` for limit buys, but IDR-based buys (`paper.rs:492`) always set it.

- [ ] **`alert_watch` subscribes to `chart:tick-{pair}` but auth/result detection differs from `ws_connect_and_listen`** — The auth check in `alert.rs:575` looks for `result.status == "ok"`, while the WS module checks for `id == 1` + `result` presence. Should use the same approach for consistency.

- [ ] **`save_alerts` doesn't set restrictive file permissions** — `src/commands/alert.rs:144`: Uses `fs::write` without `0o600` mode. Alerts don't contain secrets but should still use the same pattern as `config.rs:101-112` for consistency.

- [ ] **Duplicate WS token fetching logic** — `src/commands/alert.rs:544-552` and `src/commands/websocket.rs:16-26` both fetch public WS tokens with the same approach. Consider sharing a helper.

- [ ] **`paper_balance_value` includes `balances` map directly in JSON — raw f64 values may have floating-point artifacts** — `src/commands/paper.rs:979`: Exposes `state.balances` directly. After rounding, values should be clean, but direct f64 exposure is fragile for downstream consumers.

- [ ] **`cancel_all_paper_orders` returns `Vec<(u64, String)>` for failures but `paper_cancel_all` only reports IDs** — `src/commands/paper.rs:658-659`: The CLI handler extracts only the IDs from failures and discards error messages. Should include error reasons for better diagnostics.

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
