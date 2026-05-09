## 🚀 Welcome to Indodax CLI v0.1.2

The unofficial, fast, and feature-rich command-line interface for **Indodax**, Indonesia's largest cryptocurrency exchange.

### 🆕 What's New in v0.1.2

- **🐛 Bug Fixes**:
  - Fixed compilation errors in `paper` trading tests.
  - Resolved several Clippy linting warnings for better code quality and performance.
  - Fixed a critical bug in the `trade buy` command where orders were incorrectly rejected due to minimum amount constraints (switched to native `idr` parameter).
- **⚡ Performance & Quality**:
  - Optimized various data transformation pipelines in `account` and `market` commands.
  - Improved code structure and safety in internal helper functions.

### ✨ Highlights from v0.1.1

- **🤖 AI Agent Integration (MCP)**: Built-in Model Context Protocol server. Seamlessly connect your Indodax account to **Claude Desktop, ChatGPT, Cursor, or Gemini CLI**.
- **🔥 Real-Time WebSocket Streams**: Live data for tickers, trades, order books, and private order updates.
- **📊 Comprehensive Market Data**: Access OHLCV, order books, tickers, and price increments without an API key.
- **💰 Full Account Management**: Check balances, track open orders, and view trade/transaction history (V2 API support).
- **🛠️ Powerful Trading**: Execute buy/sell orders (including Market IDR orders) and manage a deadman switch countdown.
- **🧪 Paper Trading**: Risk-free simulated environment with virtual balances to test your strategies.
- **🔐 Secure & Flexible**: HMAC-SHA512 signing, 0600 config permissions, and support for ENV vars/CLI flags.

### 📦 Installation

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
