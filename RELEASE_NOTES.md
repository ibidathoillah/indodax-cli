## 🚀 Welcome to Indodax CLI v0.1.45

The unofficial, fast, and feature-rich command-line interface for **Indodax**, Indonesia's largest cryptocurrency exchange.

### 🆕 What's New in v0.1.45

- **🚀 Glama Deploy Fixes**:
  - Updated the Docker image to Debian Trixie and removed unnecessary OpenSSL runtime dependencies.
  - Limited hosted builds to the CLI, MCP, and callback-server features needed for deployment.
  - Added an `indodax` command alias alongside the `indodax-cli` binary for MCP clients.
  - Added `.dockerignore` to keep hosted Docker build contexts small and predictable.
  - Documented the exact Glama build spec and the transient `ECONNRESET` metadata-fetch failure mode.

- **🛠️ Dependency & API Fixes**:
  - **rmcp 1.7.0 Alignment**: Adapted MCP server code to the latest `rmcp` API changes (model renames and constructor updates). Fixed non-exhaustive struct creation errors.
  - **rustyline 12 Support**: Fixed `DefaultEditor` import issues in the interactive shell.
  - **WASM Compatibility**: Improved target-specific builds for shell command parsing.
  - **Feature Gating**: Refined MCP feature dependencies in `Cargo.toml`.

- **📊 Expanded Market Data**:
  - Added support for several new public endpoints: `webdata`, `chatroom`, `pairs_v2`, `search`, `terminal`, `onramp`, and `news`.
  - **TradingView History V2**: Enhanced OHLCV data fetching with better interval handling.

- **🚀 Glama & Cloud Ready**:
  - **Optimized Dockerfile**: Added default `mcp` command for seamless one-click deployment as an MCP server.
  - **License Compliance**: Added official MIT LICENSE file for registry compliance.
  - **Environment Documentation**: Added `.env.example` for easier configuration in hosted environments.

- **🔧 Platform & CI/CD**:
  - Improved WASM support for web-based integrations.
  - Added Android release build support for Termux users.
  - Streamlined NPM publishing for unscoped packages.

### ✨ Core Features

- **🤖 AI Agent Integration (MCP)**: Built-in Model Context Protocol server. Seamlessly connect your Indodax account to **Claude Desktop, ChatGPT, Cursor, or Gemini CLI**.
- **🔥 Real-Time WebSocket Streams**: Live data for tickers, trades, order books, and private order updates.
- **📊 Comprehensive Market Data**: Access OHLCV, order books, tickers, and price increments without an API key.
- **💰 Full Account Management**: Check balances, track open orders, and view trade/transaction history (V2 API support).
- **🛠️ Powerful Trading**: Execute buy/sell orders (including Market IDR orders) and manage a deadman switch countdown.
- **🧪 Paper Trading**: Risk-free simulated environment with virtual balances to test your strategies.
- **🔐 Secure & Flexible**: HMAC-SHA512 signing, 0600 config permissions, and support for ENV vars/CLI flags.

### 📦 Installation

**From NPM:**
```bash
npm install -g indodax-cli
```

**From Source (requires Rust):**
```bash
git clone https://github.com/ibidathoillah/indodax-cli.git
cd indodax-cli
cargo install --path .
```

### 🤖 MCP Integration
Add this to your `claude_desktop_config.json`:
```json
{
  "mcpServers": {
    "indodax": {
      "command": "indodax",
      "args": ["mcp", "-s", "all"]
    }
  }
}
```

---
*Disclaimer: This is an unofficial tool. Trading cryptocurrency involves significant risk. Use at your own risk.*
