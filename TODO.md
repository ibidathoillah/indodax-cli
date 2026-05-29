## UPDATED 2026-05-29 (3rd Review) — Full WebSocket, Advanced Trading, and Affiliate features implemented.

### Completed (this session)
- [x] **Stop-Limit Orders** — Added `stop_price` support to CLI (`buy`/`sell`) and MCP tools.
- [x] **Advanced Trading Params** — Added `client_order_id` support for order tracking in CLI and MCP.
- [x] **Order Lookup** — Implemented `get-order-by-client-id` command and MCP tool.
- [x] **Affiliate Commands** — Added `list-downline` and `check-downline` commands to CLI and MCP.
- [x] **Equity Snap in MCP** — Added `equity_snap` and `equity_history` tools to MCP for AI-driven portfolio tracking.
- [x] **Deposit Address in CLI** — Added `indodax funding deposit-address` command to CLI.
- [x] **Auto-completion** — Implemented command and pair completion for the interactive shell.
- [x] **Private WebSocket** — Full implementation of `ws orders` (token generation + live streaming) [#19].
- [x] **Public WebSocket Recovery** — Implemented offset-based recovery for market data streams.
- [x] **Multi-pair WebSocket** — Support for comma-separated pairs in ticker, trades, and book commands.
- [x] **Backward Compatibility** — Added hidden `market`, `account`, `trade`, `funding` commands to `lib.rs` for v0.1.x script compatibility.

### High Priority (Planned)
- [ ] **Reliability Upgrades** — Client-side rate-limit tracking and high-precision nonce generation [#9].
- [ ] **Extended TradeAPI-2** — Map more endpoints to the new V2 REST API where available.
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

