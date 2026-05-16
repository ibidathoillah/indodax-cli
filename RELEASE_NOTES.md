## 🚀 Welcome to Indodax CLI v0.1.11

The unofficial, fast, and feature-rich command-line interface for **Indodax**, Indonesia's largest cryptocurrency exchange.

### 🆕 What's New in v0.1.11

- **🔥 WebSocket Reliability Overhaul**:
  - **Connection Stability**: Added application-level Ping (method 7) every 30 seconds and automatic reconnection with exponential backoff for all WebSocket streams.
  - **Private WebSocket (PWS)**: Fully rewritten to support the correct `connect` (authenticate) and `subscribe` flow. Now streams both **Order Updates** and **Balance Updates** in real-time.
  - **Robust Parsing**: Enhanced public WebSocket handlers to support multiple Indodax API response formats (array-based and object-based), ensuring compatibility with all trading pairs.
  - **Flexible Auth**: Introduced `ws_token` configuration. Prioritizes dynamic fetching, followed by user-defined tokens in `config.toml`, with a reliable hardcoded default fallback.
- **🐛 Bug Fixes**:
  - **Market Pairs**: Fixed an issue where `indodax market pairs` returned an empty table due to an unhandled API response format (JSON array).
  - **Market Trades**: Resolved empty output by correcting the trading pair format (no underscore) required by the trades API.
  - **Market OHLC**: Refactored parser to support the modern array-of-objects response format, ensuring historical candle data is correctly displayed.
- **🛡️ Security & Configuration**:
  - Added `ws_token` field to `IndodaxConfig` for custom WebSocket authentication tokens.
  - Improved credential resolution priority to ensure consistency across CLI, ENV, and Config file.
- **🧪 Enhanced Testing**:
  - Added comprehensive unit tests for WebSocket message parsing and event handling.
  - Introduced `scripts/e2e_websocket_mock.sh` for automated connectivity verification.

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
