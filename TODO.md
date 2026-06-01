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
- [x] **AI-Native Foundation (Kraken-Style Phase 1)**
  - [x] Add `AGENTS.md`, `CONTEXT.md`, `CLAUDE.md`, and `llms.txt`.
  - [x] Create `agents/tool-catalog.json` and `agents/error-catalog.json`.
  - [x] Implement Workflow Skills in `skills/*/SKILL.md` (Safe Order Preview, Portfolio Review, etc.).
  - [x] Enriched JSON error envelope (category, retryable, suggestion, docs_url).
- [x] **Safety & Onboarding (Kraken-Style Phase 2)**
  - [x] Implement `indodax setup` interactive wizard.
  - [x] Implement `indodax status` and `indodax auth doctor`.
  - [x] Add `indodax auth reset`.
  - [x] Support `--api-secret-file`.
  - [x] Signed release verification (checksums + minisign).
  - [x] Add `DISCLAIMER.md` and `CONTRIBUTING.md`.
- [x] **Trading Polish (Kraken-Style Phase 3)**
  - [x] Order preview/validate (`indodax order preview buy --dry-run`).
  - [x] Portfolio summary and allocation (`indodax portfolio summary`).
  - [x] Orderbook grouping (`indodax market orderbook-grouped`).
  - [x] Spreads calculation (`indodax market spreads`).
  - [x] Transactions/Trades export (`indodax export transactions --format csv`).
  - [x] WebSocket private balances (`indodax ws balances`).
  - [x] Order cancel-batch and order edit (cancel+replace).
- [x] **Kraken-Style Command Structure (Aliases)**
  - [x] `indodax market <subcommand>` (time, pairs, ticker, tickers, summaries, orderbook, trades, ohlc, history).
  - [x] `indodax account <subcommand>` (info, balance, transactions, trades-history, ledger).
  - [x] `indodax funding <subcommand>` (deposit-address, withdraw-fee, withdraw, withdraw-callback).
  - [x] `indodax order <subcommand>` (preview, validate, open, closed, get, history, cancel-batch, edit).
- [x] **Reliability Upgrades** — Client-side rate-limit tracking and high-precision nonce generation [#9].
- [x] **Extended TradeAPI-2** — Map more endpoints to the new V2 REST API where available.
- [x] **Plugin Ecosystem** — Add `.claude-plugin/`, `.codex-plugin/`, `.cursor-plugin/`, and `gemini-extension.json`.
### Medium Priority
- [x] **Affiliate Commands** — `listDownline` and `checkDownline` support [#20, #8].
- [x] **Auto-completion** — Pair and command completion for the interactive shell [#16].
- [x] **Shell Improvements** — REPL-specific enhancements and stability fixes.
- [x] **Visual Documentation** — Add demo GIFs/SVGs to README [#13].

### Low Priority
- [ ] **Voucher Commands** — `createVoucher` support (partner-only) [#22, #8].
- [ ] **Localization** — Support for Indonesian language outputs [#15].
- [ ] **Homebrew Tap** — Create a brew formula for easier installation [#12].
- [ ] **Community Outreach** — Promotion on relevant platforms [#14].

