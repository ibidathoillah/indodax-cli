## 🚀 Welcome to Indodax CLI v0.1.30

The unofficial, fast, and feature-rich command-line interface for **Indodax**, Indonesia's largest cryptocurrency exchange.

### 🆕 What's New in v0.1.30

- **🤖 MCP Server Overhaul**:
  - **New Tools**: Added support for price alerts, deposit addresses, and credential management directly via MCP.
  - **WebSocket Snapshots**: Added tools to fetch real-time snapshots of market data.
  - **Resources & Prompts**: Implemented MCP Resources (config, pairs, paper state) and Prompts (order creation, portfolio check) for better AI agent guidance.
  - **Security**: Refined dangerous operation guards for trade and funding tools.

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
