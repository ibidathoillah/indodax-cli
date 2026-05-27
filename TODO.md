## UPDATED 2026-05-17 (2nd Review) — All high/medium issues resolved.

### Completed (this session)
- [x] **Backward Compatibility** — Added hidden `market`, `account`, `trade`, `funding` commands to `lib.rs` for v0.1.x script compatibility.
- [x] **Paper State Migration** — Implemented one-time migration from `config.toml` to `paper_state.json` in `PaperState::load()`.
- [x] **Table Renderer Warnings** — Updated `table::render()` to display warnings from `CommandOutput.warnings`.
- [x] **MCP Tick Size Warnings** — Updated `handle_buy/sell_order` to include tick size warnings in the JSON response body.

### High Priority (Planned)
- [ ] **Private WebSocket** — Full implementation of `ws orders` (token generation + live streaming) [#19].
- [ ] **Stop-Limit Orders** — Add `stoplimit` support to trade commands and MCP [#21].
- [ ] **Advanced Trading Params** — Add `client_order_id` and `time_in_force` (GTC/MOC) support [#18, #8].
- [ ] **Order Lookup** — Implement `getOrderByClientOrderId` command and MCP tool [#17, #8].
- [ ] **Reliability Upgrades** — Client-side rate-limit tracking and high-precision nonce generation [#9].

### Medium Priority
- [ ] **Affiliate Commands** — `listDownline` and `checkDownline` support [#20, #8].
- [ ] **Auto-completion** — Pair and command completion for the interactive shell [#16].
- [ ] **Shell Improvements** — REPL-specific enhancements and stability fixes.
- [ ] **Visual Documentation** — Add demo GIFs/SVGs to README [#13].

### Low Priority
- [ ] **Voucher Commands** — `createVoucher` support (partner-only) [#22, #8].
- [ ] **Localization** — Support for Indonesian language outputs [#15].
- [ ] **Homebrew Tap** — Create a brew formula for easier installation [#12].
- [ ] **Community Outreach** — Promotion on relevant platforms [#14].

