## 🚀 Welcome to Indodax CLI v0.2.0

This is a massive update that brings a "Kraken-Style" AI-native architecture and advanced trading polish to the Indodax CLI. The CLI is now fully optimized for use by both human traders and AI agents (like Claude, Gemini, and Cursor).

### 🆕 What's New in v0.2.0

#### 🤖 AI-Native Foundation
- **Agent Documentation**: Added `AGENTS.md`, `CONTEXT.md`, `CLAUDE.md`, and `llms.txt` to help LLMs understand the CLI architecture immediately.
- **Machine-Readable Catalogs**: Added `agents/tool-catalog.json` and `agents/error-catalog.json` defining command schemas and standardized error recovery.
- **Agent Skills**: Introduced 7 new workflow skills in the `skills/` directory (e.g., `safe-order-preview`, `portfolio-review`) to guide AI reasoning.
- **Plugin Ecosystem**: Added `.claude-plugin/` and `gemini-extension.json` for IDE integration.

#### 🛡️ Safety & Onboarding
- **Interactive Setup**: Use `indodax setup` to configure API keys, default output formats, trading pairs, and MCP profiles through a guided wizard.
- **System Doctor**: Use `indodax status` and `indodax auth doctor` to instantly diagnose network and permission issues.
- **API Secret Files**: Support for `--api-secret-file` to prevent secret exposure in process listings.
- **Signed Binaries**: Release binaries are now cryptographically signed with `minisign` to ensure authenticity.

#### 📈 Trading Polish & Advanced Market Data
- **Order Previews**: Added the `--validate` flag to buy and sell commands to dry-run trades and catch tick-size or balance errors *before* execution.
- **Batch Operations**: Use `indodax order cancel-batch` to efficiently clear multiple orders.
- **Order Editing**: Added `indodax order edit` which automatically handles cancel-and-replace workflows.
- **Portfolio Insights**: `indodax portfolio summary` and `indodax portfolio allocation` aggregate your holdings and calculate total IDR value using real-time prices.
- **Advanced Orderbooks**: Added `indodax market orderbook-grouped` to visualize liquidity clusters (walls), and `indodax market spreads` to measure the bid/ask gap.
- **Data Export**: Dump your transaction and trade histories to CSV with the new `indodax export` command group.
- **Private WebSocket**: Added `indodax ws balances` for streaming real-time account balance updates.

#### 🏗️ Command Restructuring
- **Kraken-Style Grouping**: Commands have been reorganized into intuitive groups (`market`, `account`, `order`, `funding`, `ws`, `auth`). Legacy top-level commands still work but are hidden to provide a cleaner `--help` menu.
- **TradeAPI-2 Upgrade**: Order cancellation now utilizes the modern TradeAPI-2 endpoint (`/api/v2/order/{id}`) for improved reliability.

#### ⚡ Reliability
- **Rate-Limiter**: Built-in client-side token bucket proactively sleeps to prevent `429 Too Many Requests` errors.
- **Microsecond Nonces**: Upgraded API nonce generation to microsecond precision, eliminating collision errors during high-frequency trading.
- **Smart Auto-Completion**: The interactive `indodax shell` now fetches pairs dynamically and supports multi-word completion.

## 🚀 Welcome to Indodax CLI v0.2.0

### 🆕 What's New in v0.2.0
- **🛠️ WebSocket TLS Test Fix**: Resolved a compilation error in `tests/websocket_tls.rs` where `url::Url` was incorrectly passed to `connect_async`.
- **🏗️ Feature Guarding Improvements**: Fixed several build issues when compiling with minimal features (e.g., without `mcp` or `server`). Properly guarded `axum` and `mcp` related code in `src/commands/funding.rs` and `src/main.rs`.
- **🧹 Warning Cleanup**: Suppressed unused variable warnings when certain features are disabled.

## 🚀 Welcome to Indodax CLI v0.2.0

### 🆕 What's New in v0.2.0
- **🧰 MCP Tool Definition Quality**: Added strict MCP input schemas, tool titles, and behavior annotations for read-only, destructive, idempotent, and open-world hints to improve Glama scoring and agent safety.
- **📚 Clearer Tool Guidance**: Expanded lower-scoring market, account, auth, paper trading, and WebSocket tool descriptions with authentication requirements, side effects, output shape, and recommended alternatives.
- **🛠️ Test Suite Fix**: Resolved a compilation error in `mcp_e2e` integration tests by making HTTP handler functions public.

## 🚀 Welcome to Indodax CLI v0.2.0

This release brings highly requested advanced trading features, affiliate support, and a significant boost to shell productivity.

### 🆕 What's New in v0.2.0

- **🛑 Stop-Limit Orders**: 
  - Full support for stop-limit orders in both CLI and MCP. 
  - Use `--stop-price` and `--price` (limit price) in `buy` and `sell` commands.

- **🆔 Advanced Order Tracking**:
  - Added support for `client_order_id` in trade commands for custom tracking.
  - New `get-order-by-client-id` command for detailed order lookups.

- **🤝 Affiliate & Funding**:
  - New commands: `account list-downline` and `account check-downline`.
  - CLI support for `funding deposit-address` (previously MCP only).

- **⌨️ Interactive Shell Power-up**:
  - Implemented **Auto-completion** for all commands and common trading pairs.
  - Tab through options to trade faster than ever.

- **📈 Portfolio Tracking in MCP**:
  - New tools: `equity_snap` and `equity_history` for AI-driven portfolio growth analysis.

### 🆕 What's New in v0.2.0

- **📡 Full WebSocket API Coverage**:
  - Implemented all public and private channels from the official Indodax documentation.
  - Added support for subscribing to **multiple pairs** simultaneously (e.g., `btc_idr,eth_idr`).
  - New `websocket subscribe` command for raw channel access.

- **🔄 Resilient Streaming with Recovery**:
  - Implemented **offset-based data recovery**; never miss an update after a brief disconnection.
  - Automatic reconnection with **exponential backoff** and application-level pings.
  - Robust handling of both legacy and v5.2.0 private WebSocket message formats.

- **📊 Rich Data & Enhanced Output**:
  - Added volume, sequence numbers, and high-precision timestamps to all WebSocket events.
  - Improved real-time order book display with support for multiple markets.
  - Detailed Private Order updates including fill information, fees, and tax assets.

### 🆕 What's New in v0.2.0

- **🌐 Production-Ready MCP HTTP Bridge**:
  - Integrated a high-performance HTTP server directly into the binary using **Axum**.
  - Supports **Isolated Multi-User Bridge** via custom headers (`x-api-key`, `x-api-secret`).
  - Added **X-Bridge-Auth** security layer to protect the server from unauthorized access.
  - Full tool coverage for HTTP transport (Market, Account, Trading, Paper, and Alerts).
  - Built-in **CORS support** for browser-based tool integrations.

- **🛡️ Security & Stability**:
  - Implemented secure credential isolation; API keys are never stored on the server.
  - Added **Automated CI/CD** via GitHub Actions for testing and Docker publishing.
  - Enhanced **E2E Testing** for HTTP bridge authentication and tool dispatching.
  - Replaced experimental library logic with a stable **Direct Tool Dispatcher**.

- **📦 Deployment & Integration**:
  - Added **Docker Compose** support for one-command deployment (Server + Cloudflare Tunnel).
  - Updated **OpenAPI Spec (v1.2)** for smarter ChatGPT Actions integration.
  - Revamped **MCP Documentation** in `src/mcp/README.md`.
  - Optimized Docker image with multi-stage build.

### 🆕 What's New in v0.2.0

- **🚀 Glama Deploy Fixes**:
  - Updated the Docker image to Debian Trixie and removed unnecessary OpenSSL runtime dependencies.
  - Limited hosted builds to the CLI, MCP, and callback-server features needed for deployment.
  - Added an `indodax` command alias alongside the `indodax-cli` binary for MCP clients.
  - Added `.dockerignore` to keep hosted Docker build contexts small and predictable.
  - Documented the exact Glama build spec and the transient `ECONNRESET` metadata-fetch failure mode.

... (sisanya tetap sama)
