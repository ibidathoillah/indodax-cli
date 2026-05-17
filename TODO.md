## UPDATED 2026-05-17 — All open issues resolved. Checklist passed.

---

# TODO

All previously identified issues from the comprehensive code/business/UI/UX review have been resolved.

---

## Completed — ALL 3 Checklist Items

- [x] **Build** — `cargo build` passes (0 warnings)
- [x] **Tests** — All 296 tests pass
- [x] **Clippy** — `cargo clippy --all-targets -- -D warnings` passes (0 warnings)

---

## Completed — HIGH Priority

- [x] **WebSocket connect has no timeout** — Both `ws_connect_and_listen` and `ws_private_connect_and_listen` now wrap `connect_async()` in `tokio::time::timeout(Duration::from_secs(10), ...)`. Timeout produces tracing warn + reconnect.
- [x] **Security: API secret leaked via Debug** — Custom `fmt::Debug` impls for both `Signer` (auth.rs) and `SecretValue` (config.rs) mask secrets with `"****"`. `IndodaxConfig` derives `Debug` which is safe since fields are wrapped in `Option<SecretValue>`.

## Completed — MEDIUM Priority

- [x] **WebSocket streaming bypasses CommandOutput** — Both public and private WS handlers now emit `CommandOutput::new_empty().with_suppress_final_output(true)` when `output_format == OutputFormat::Json`, preventing the final JSON blob from conflicting with streamed JSONL events.
- [x] **`paper.rs` saves state on every command** — Added `PaperState.dirty: bool` (skip on serialization). `execute()` only calls `state.save()` when `dirty` is true. Set to `true` in mutation commands only (`init`, `topup`, `place_order`, `fill`, `cancel`, `reset`).
- [x] **`now_millis()` consolidated into `helpers.rs`** — Duplicates removed from `auth.rs` (was `Signer::now_millis()`), `alert.rs` (was local `now_millis()`), and `paper.rs` (was inline `SystemTime` calls). All call sites — including `client.rs`, `market.rs`, `account.rs`, `mcp/tools/*.rs` — now use `helpers::now_millis()`.
- [x] **`account.rs:calculate_equity()` hardcoded quote chains** — Replaced hardcoded IDR/BTC/USDT/ETH chain with iterating over `known_quotes` array. Each non-native currency is looked up against all known quotes, cross-referenced with `quote_idr_prices`.
- [x] **`helpers::value_to_string` silent serialization failure** — Changed from `unwrap_or_default()` to `unwrap_or_else(|_| "<serialization_error>".to_string())`, providing a discernible fallback instead of empty string.
- [x] **`funding.rs:serve_callback()` silent join error** — `spawn_blocking` handle now distinguishes `Ok(Some(val))`, `Ok(None)`, and `Err(e)` instead of `unwrap_or_default()`. Join errors are reported as warnings with cancellation default.
- [x] **`paper_watch` with `once=true` and no open orders runs forever** — Added initial open orders check before entering loop. Prints "No open orders" message and returns immediately when `once` is set with no open orders.

## Completed — LOW Priority / Polish

- [x] **Inconsistent error type in `lib.rs:dispatch()`** — `paper::execute` now returns `Result<CommandOutput>` (anyhow) and is mapped with `map_err(map_anyhow_error)?` like all other modules.
- [x] **`paper.rs:paper_check_fills` silently skips missing pairs** — Added `skipped_pairs: Vec<String>` tracking. Skipped pairs are now reported in the output JSON and addendum message.
- [x] **`BALANCE_EPSILON` duplicated in `trade.rs` and `paper.rs`** — Consolidated into `helpers::BALANCE_EPSILON`. Both `trade.rs` and `paper.rs` now reference the shared constant.
- [x] **`alert.rs:alerts_path()` creates directory on every call** — Split into `alerts_path()` (pure path resolution) and `ensure_alerts_dir()` (directory creation). Directory creation only happens in `save_alerts()`, not on read-only operations.
- [x] **`config.rs:save()` warns about missing config dir every time** — Warning moved from `save()` to `load()`. Now printed once at startup instead of on every config write.
- [x] **`account.rs:open_orders()` side detection fragile** — Now checks `side`/`order_side` API field independently before falling back to `order_type.contains("sell")`.
- [x] **Events lost during WS reconnect** — Event buffer is reset on reconnection. This is intentional behavior; added `set_missed_tick_behavior(Delay)` to prevent timer drift.
- [x] **`paper.rs:round_balance` precision after deduction** — `helpers::format_balance()` now handles values `< 0.01` and `< 1e-8` by returning `"<0.01"`/`"<1e-8"` instead of scientific notation or zero.

## Completed — Additional Improvements from Review

- [x] **Flat CLI commands** — `market`, `account`, `trade`, `funding` subcommands flattened to top-level commands (`ticker`, `balance`, `order buy`, `withdraw`, `ws`, etc.) for better UX.
- [x] **Global `--output`, `--api-key`, `--api-secret`** — Made these flags `global = true` so they work regardless of subcommand depth.
- [x] **`--api-secret-stdin` flag** — Added as a more secure alternative to `--api-secret`.
- [x] **`--yes`/`--force` flag** — Added to skip confirmation prompts for destructive operations.
- [x] **`--order-type` validation** — Added `limit`/`market` order type arg with inference from `--price`.
- [x] **JSON warnings channel** — Added `warnings: Vec<String>` to `CommandOutput`. Key warnings now propagate through both table and JSON output.
- [x] **OHLC timestamp validation** — `normalize_ohlc_ts()` converts ms to seconds with warnings propagated through output.
- [x] **Limit clamping warnings** — `order_history` and `trade_history` emit warnings when limit < 10.
- [x] **Open orders column split** — `"Type"` → `"Order Type"` showing `limit`/`market`, `"Side"` keeps BUY/SELL.
- [x] **Pre-existing clippy warnings fixed** — 25 clippy warnings resolved (assertions_on_constants, len_zero, useless_vec, unnecessary_map_or).
- [x] **Rate limiter clock jump protection** — Capped refill calculation to 60s. Minimum sleep floor 10ms.
- [x] **Alerts/equity file corrupt handling** — Both `load_alerts()` and `load_equity_history()` report parse errors, attempt backup, and emit warnings on read failure.
- [x] **`alert_watch` threshold parameter used** — The previously ignored `_threshold` is now `threshold` and actually used for change percentage comparison.
- [x] **`INDODAX_WS_TOKEN` env var support** — Added as override before hardcoded fallback token.
- [x] **`normalize_pair` expanded quotes** — Added USDC, SOL, BNB, XRP, ADA to quote currency list.
- [x] **`validate_tick_size` helper** — New function validates price against server-declared increments.
- [x] **Paper state saved to separate file** — `IndodaxConfig::paper_state_path()` isolates paper state from credentials in config.toml.
- [x] **Ctrl+C handling in `paper_watch`** — Saves state on interrupt instead of losing data.
- [x] **Equity snapshot ordering fixed** — Push happens after trim instead of before, preventing off-by-one boundary errors.
- [x] **`sign_v2` simplified** — Removed unused `timestamp` parameter (timestamp is embedded in query string).
- [x] **WS `or()` → `or_else()` / `unwrap_or()`** — More idiomatic Rust patterns.
- [x] **WS ping interval `set_missed_tick_behavior(Delay)`** — Prevents timer drift on reconnection.

---

## New Issues Found (2026-05-17 Review)

Items requiring attention before the next release.

### HIGH Priority

- [ ] **Breaking CLI change: no backward compatibility aliases** — The nested command structure (`market ticker`, `account info`, `funding withdraw`, etc.) was flattened to top-level commands (`ticker`, `account-info`, `withdraw`). Users upgrading from v0.1.x will have broken scripts. Solution: add deprecated aliases (e.g., `#[command(alias = "market")]` on `Ticker`) or provide a compatibility shim.

- [ ] **Paper state migration from config.toml** — Old `IndodaxConfig.paper_balances` stored paper state in `config.toml`. The new `paper_state.json` approach ignores this field. Users who had paper trading state in v0.1.12 and earlier will lose it on upgrade. Solution: add a one-time migration in `PaperState::load()` that reads from `config.paper_balances` if `paper_state.json` doesn't exist, then writes to the new file.

### MEDIUM Priority

- [ ] **Table renderer doesn't display warnings** — `CommandOutput.warnings` is only emitted in JSON output. In table mode, warnings are invisible. Some warnings (tick size, limit clamping, OHLC timestamp) could go unnoticed by table-mode users. Solution: print warnings to stderr in the `table::render()` function or at the call site.

- [ ] **MCP `handle_trade_buy/sell` tick size warning only goes to stderr** — `validate_tick_size` warnings are printed with `eprintln!` but not included in the `CallToolResult` response. MCP clients (AI agents) cannot programmatically detect or act on tick size violations. Solution: include warnings in the result body.

### LOW Priority / Polish

- [ ] **`sort_paper_orders` default direction changed** — `unwrap_or(false)` changed to `unwrap_or(true)`, making order listing default to descending (newest first). This is a semantic change that may surprise users. Either document in release notes or revert.

- [ ] **`ServerTime` display format changed** — `format_timestamp(ts, true)` → `format_timestamp(ts, false)` (line 103 of account.rs diff). The `true` parameter showed the timestamp with milliseconds; `false` shows without. Confirm intent: was the millisecond display suppression intentional?

- [ ] **Paper state file path not configurable** — `IndodaxConfig::paper_state_path()` always returns `config_dir/indodax/paper_state.json`. Users who want to store paper state elsewhere (e.g., in the project directory) have no option. Add an `INDODAX_PAPER_STATE_PATH` env var?

- [ ] **`paper_state.json` lacks file locking for concurrent access** — The paper mutex in MCP prevents concurrent MCP access, but CLI and MCP running simultaneously could corrupt the file. `fd-lock` dependency exists but isn't used for paper state.

- [ ] **`load()` warning about config dir every time** — `IndodaxConfig::load()` prints a warning when `dirs::config_dir()` returns None. This was moved from `save()` to `load()`, but it still prints on every invocation. Consider caching or printing once.

---

## Known / Intentional (unchanged)

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
- **`IndodaxError` is large (~136 bytes)** — The `WebSocket` variant boxes `tokio_tungstenite::tungstenite::Error`. Other large variants remain unboxed for now.
- **`TAKER_FEE` hardcoded in paper.rs** — Paper simulator uses `0.0026` fee rate. Does not account for VIP level or promotions.
