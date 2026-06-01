## 🚀 Welcome to Indodax CLI v0.1.56

### 🆕 What's New in v0.1.56
- **🧰 MCP Tool Definition Quality**: Added strict MCP input schemas, tool titles, and behavior annotations for read-only, destructive, idempotent, and open-world hints to improve Glama scoring and agent safety.
- **📚 Clearer Tool Guidance**: Expanded lower-scoring market, account, auth, paper trading, and WebSocket tool descriptions with authentication requirements, side effects, output shape, and recommended alternatives.
- **🛠️ Test Suite Fix**: Resolved a compilation error in `mcp_e2e` integration tests by making HTTP handler functions public.

## 🚀 Welcome to Indodax CLI v0.1.56

This release brings highly requested advanced trading features, affiliate support, and a significant boost to shell productivity.

### 🆕 What's New in v0.1.56

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

### 🆕 What's New in v0.1.56

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

### 🆕 What's New in v0.1.56

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

### 🆕 What's New in v0.1.56

- **🚀 Glama Deploy Fixes**:
  - Updated the Docker image to Debian Trixie and removed unnecessary OpenSSL runtime dependencies.
  - Limited hosted builds to the CLI, MCP, and callback-server features needed for deployment.
  - Added an `indodax` command alias alongside the `indodax-cli` binary for MCP clients.
  - Added `.dockerignore` to keep hosted Docker build contexts small and predictable.
  - Documented the exact Glama build spec and the transient `ECONNRESET` metadata-fetch failure mode.

... (sisanya tetap sama)
