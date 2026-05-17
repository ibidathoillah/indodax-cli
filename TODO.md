## UPDATED 2026-05-17 (2nd Review) — All high/medium issues resolved.

### Completed (this session)
- [x] **Backward Compatibility** — Added hidden `market`, `account`, `trade`, `funding` commands to `lib.rs` for v0.1.x script compatibility.
- [x] **Paper State Migration** — Implemented one-time migration from `config.toml` to `paper_state.json` in `PaperState::load()`.
- [x] **Table Renderer Warnings** — Updated `table::render()` to display warnings from `CommandOutput.warnings`.
- [x] **MCP Tick Size Warnings** — Updated `handle_buy/sell_order` to include tick size warnings in the JSON response body.

