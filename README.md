# Indodax CLI

> **The unofficial, fast, and feature-rich command-line interface for [Indodax](https://indodax.com) — Indonesia's largest cryptocurrency exchange.**

[![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=for-the-badge)](https://opensource.org/licenses/MIT)
[![Test Coverage](https://img.shields.io/badge/Coverage-100%25-success?style=for-the-badge)](https://github.com/ibidathoillah/indodax-cli)

Track markets, execute trades, manage your portfolio, and stream real-time data — all from your terminal.

---

## ✨ Features

- **🤖 AI Agent Integration** — Built-in MCP (Model Context Protocol) server for Claude, ChatGPT, Cursor, VS Code, Gemini CLI, and any MCP-compatible agent
- **🔥 Real-Time WebSocket Streams** — Live ticker, trades, order book, and private order updates
- **📊 Comprehensive Market Data** — OHLCV, order books, tickers, summaries, and price increments
- **💰 Full Account Management** — Balances, open orders, order history, trade history, and transactions
- **🛠️ Powerful Trading** — Place buy/sell orders, cancel orders, and set deadman switches
- **🧪 Paper Trading** — Risk-free simulated trading environment to test strategies
- **🔔 Price Alerts** — Set price alerts and monitor in real-time via WebSocket
- **🔐 Secure Authentication** — HMAC-SHA512 API signing with multiple credential resolution methods
- **📋 Flexible Output** — Human-friendly tables or machine-readable JSON
- **🖥️ Interactive Shell** — Built-in REPL for exploratory usage
- **⚡ Blazing Fast** — Built with Rust for maximum performance and safety

---

## 📦 Installation

### From Cargo (Crates.io)

```bash
cargo install indodax-cli
```

### From NPM

```bash
npm install -g indodax-cli
```

### From Docker

```bash
docker pull ibidathoillah/indodax-cli:latest
docker run -it --rm -v ~/.config/indodax:/root/.config/indodax ibidathoillah/indodax-cli account balance
```

### From Source (requires [Rust](https://rustup.rs/))

```bash
git clone https://github.com/ibidathoillah/indodax-cli.git
cd indodax-cli
cargo install --path .
```

---

## 🚀 Recent Highlights (v0.1.13)

- **🔥 WebSocket Reliability Overhaul**: Implemented application-level Pings, automatic reconnection with exponential backoff, and a complete rewrite of the Private WebSocket to support real-time order and balance updates.
- **🔐 Secure Authentication**: Added support for user-configurable WebSocket tokens with fallback to a reliable hardcoded default, ensuring stable connections even if official tokens change.
- **📊 Strict TradeAPI-2 Compliance**: Continued enforcement of official Indodax specifications and normalized symbol formats across all commands.
- **🧹 Internal Refactoring**: Improved error handling and response parsing for the Orderbook, supporting both legacy and modern API formats.

---

## 🚀 Quick Start

### 1. Check Market Data (No API Key Needed)

Market data commands work **without any API credentials**:

```bash
indodax market server-time
indodax market ticker btc_idr
indodax market orderbook btcidr
indodax market pairs
indodax market ohlc --symbol BTCIDR
```

### 2. Configure API Credentials (For Account & Trading)

```bash
indodax auth set --api-key YOUR_API_KEY --api-secret YOUR_API_SECRET
```

Or use environment variables:

```bash
export INDODAX_API_KEY=your_api_key
export INDODAX_API_SECRET=your_api_secret
```

Credentials are resolved in this priority order:
1. CLI flags (`--api-key`, `--api-secret`)
2. Environment variables (`INDODAX_API_KEY`, `INDODAX_API_SECRET`)
3. Config file (`~/.config/indodax/config.toml` with `0600` permissions)

### 3. View Account (Requires API Key)

```bash
indodax account balance
indodax account info
```

### 4. Start the Interactive Shell

```bash
indodax shell
```

---

## 🤖 MCP Server (AI Agent Integration)

indodax-cli includes a built-in **Model Context Protocol (MCP)** server over stdio. No subprocess wrappers needed.

MCP tool calls run through the same Rust code path as CLI commands and inherit the same error handling, rate-limit behavior, and security model.

> **⚠️ Warning**
>
> MCP is local-first and designed for your own machine. Any AI agent connected to this MCP server uses the same configured Indodax account and API key permissions. Do **not** expose, tunnel, or share this server outside systems you control. Always use `https://` and `wss://` endpoints. Treat this integration as alpha and use **least-privilege API keys**.

### Usage

```bash
indodax mcp                           # default: market, account, paper (read-only)
indodax mcp -s all                    # all services, dangerous calls require acknowledged=true
indodax mcp -s all --allow-dangerous  # all services, no per-call confirmation required
indodax mcp -s market,trade,paper     # specific service groups only
```

### Service Groups

| Group | Tools | Auth Required | Dangerous |
|-------|-------|---------------|-----------|
| `market` | Server time, ticker, pairs, orderbook, trades, OHLC, price increments | No | No |
| `account` | Balance, open orders, order history, trade history, account info | Yes | No |
| `trade` | Buy, sell, cancel orders | Yes | **Yes** |
| `funding` | Withdraw fees, withdraw crypto | Yes | **Yes** |
| `paper` | Paper trading init, balance, buy, sell, orders, cancel, history, status | No | No |
| `auth` | Show config, test credentials | Varies | No |

### Dangerous Operations

By default, `trade` and `funding` groups require each tool call to include `acknowledged: true` as a parameter. Use `--allow-dangerous` to skip this per-call confirmation.

### Configure Your MCP Client

Add to your MCP client configuration (Claude Desktop, VS Code, Cursor, Windsurf, etc.):

**Claude Desktop** (`claude_desktop_config.json`):
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

**VS Code / Cursor** (`.vscode/mcp.json` or Cursor MCP settings):
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

**Gemini CLI**:
```bash
gemini extensions install https://github.com/ibidathoillah/indodax-cli
```

---

## 📖 Usage

```
indodax [OPTIONS] <COMMAND>

Options:
  -o, --output <OUTPUT>           Output format: table or json [default: table]
      --api-key <API_KEY>         API key (overrides config and env var)
      --api-secret <API_SECRET>   API secret (overrides config and env var)
  -v, --verbose                   Enable verbose output
  -h, --help                      Print help
  -V, --version                   Print version
```

---

## 🔗 Commands

### Market Data (Public API)

| Command | Description |
|---------|-------------|
| `indodax market server-time` | Get server time |
| `indodax market pairs` | List available trading pairs |
| `indodax market ticker <pair>` | Get ticker for a trading pair |
| `indodax market ticker-all` | Get tickers for all pairs |
| `indodax market summaries` | Get 24h and 7d market summaries |
| `indodax market orderbook <pair>` | Get order book depth |
| `indodax market trades <pair>` | Get recent trades |
| `indodax market ohlc` | Get OHLCV candle data |
| `indodax market price-increments` | Get tick sizes |

### Account (Private API)

| Command | Description |
|---------|-------------|
| `indodax account info` | Get account information |
| `indodax account balance` | Show wallet balances |
| `indodax account open-orders` | List open orders |
| `indodax account order-history` | Get order history (v2 API) |
| `indodax account trade-history` | Get trade fill history (v2 API) |
| `indodax account trans-history` | Get deposit/withdrawal history |
| `indodax account get-order` | Get order details |

### Trading (Private API)

| Command | Description |
|---------|-------------|
| `indodax trade buy -p <pair> -i <idr>` | Place a buy order (IDR amount) |
| `indodax trade buy -p <pair> -a <amount> [-r <price>]` | Place a buy order (base amount) |
| `indodax trade sell -p <pair> -a <amount> [-r <price>]` | Place a sell order |
| `indodax trade cancel -i <id> -p <pair> -t <type>` | Cancel an order by ID |
| `indodax trade cancel-by-client-id` | Cancel by client order ID |
| `indodax trade countdown` | Deadman switch countdown |

### Funding (Private API)

| Command | Description |
|---------|-------------|
| `indodax funding withdraw-fee` | Check withdrawal fee |
| `indodax funding withdraw` | Withdraw cryptocurrency |
| `indodax funding serve-callback` | Start callback validation server |

### WebSocket Streaming

| Command | Description |
|---------|-------------|
| `indodax ws ticker <pair>` | Stream real-time ticker |
| `indodax ws trades <pair>` | Stream real-time trades |
| `indodax ws book <pair>` | Stream real-time order book |
| `indodax ws summary` | Stream 24h summary |
| `indodax ws orders` | Stream private order updates |

### Price Alerts

> Set price alerts and monitor them in real-time via WebSocket. Never miss a trading opportunity!

| Command | Description |
|---------|-------------|
| `indodax alert add -p <pair> --above <price>` | Alert when price goes above |
| `indodax alert add -p <pair> --below <price>` | Alert when price goes below |
| `indodax alert add -p <pair> --percent-up <%>` | Alert when price increases by % |
| `indodax alert add -p <pair> --percent-down <%>` | Alert when price decreases by % |
| `indodax alert list [--history]` | List all alerts |
| `indodax alert cancel -i <id>` | Cancel specific alert |
| `indodax alert cancel --all` | Cancel all alerts |
| `indodax alert check [-p <pair>]` | Check alerts against current prices |
| `indodax alert watch [-p <pair>]` | Monitor alerts in real-time (WebSocket) |
| `indodax alert triggered` | Show triggered alerts |

### Paper Trading (Simulated)

> Paper trading mirrors the live trade commands for easy switching between simulated and real trading.

| Command | Description |
|---------|-------------|
| `indodax paper init [--idr N] [--btc N]` | Initialize with default or custom balances |
| `indodax paper reset` | Reset paper trading state |
| `indodax paper balance` | Show virtual balances |
| `indodax paper buy -p <pair> -i <idr>` | Simulated buy (IDR amount, like live) |
| `indodax paper buy -p <pair> -a <amt> [-r <price>]` | Simulated buy (base amount, like live) |
| `indodax paper sell -p <pair> -a <amt> [-r <price>]` | Simulated sell (matches live interface) |
| `indodax paper orders [-p <pair>]` | List open paper orders |
| `indodax paper cancel -i <id>` | Cancel a paper order |
| `indodax paper cancel-all` | Cancel all paper orders |
| `indodax paper fill -i <id> [-r <price>]` | Fill an open paper order |
| `indodax paper fill --all` | Fill all open orders at once |
| `indodax paper check-fills [-p <json>] [--fetch]` | Auto-fill based on market prices |
| `indodax paper topup -c <currency> -a <amount>` | Top up a virtual currency balance |
| `indodax paper history` | Show paper trade history |
| `indodax paper status` | Show paper trading status summary |

### Authentication Management

| Command | Description |
|---------|-------------|
| `indodax auth set` | Set API credentials |
| `indodax auth show` | Show current config |
| `indodax auth test` | Test API credentials |
| `indodax auth reset` | Remove stored credentials |

### Utilities

| Command | Description |
|---------|-------------|
| `indodax setup` | Interactive setup wizard |
| `indodax shell` | Start interactive REPL |

---

## 📝 Output Formats

**Table mode** (default) — human-friendly aligned tables:

```bash
indodax market ticker btc_idr
```

**JSON mode** — for scripting and automation, with AI-friendly error envelopes:

```bash
indodax -o json market ticker btc_idr
```

When an error occurs in JSON mode, a structured error envelope is returned on stdout:

```json
{
  "error": true,
  "message": "Invalid trading pair: xxx_idr",
  "error_type": "invalid_pair",
  "retryable": false
}
```

---

## 🧪 Paper Trading

Test your strategies without risking real funds. Paper trading mirrors the live trade commands for easy switching:

```bash
# Initialize with default balances (100M IDR, 1 BTC)
indodax paper init

# Or with custom balances
indodax paper init --idr 50000000 --btc 0.5

# Buy orders (matches live trade interface)
indodax paper buy -p btc_idr -i 1000000            # Buy BTC worth 1M IDR at market price
indodax paper buy -p btc_idr -a 0.1 -r 500000000   # Buy 0.1 BTC with limit price

# Sell orders (matches live trade interface)
indodax paper sell -p btc_idr -a 0.05 -r 1000000000  # Sell 0.05 BTC at limit price

# Check balances and status
indodax paper balance
indodax paper status

# Manage orders
indodax paper orders --pair btc_idr    # List open orders
indodax paper cancel -i 1              # Cancel order ID 1
indodax paper cancel-all               # Cancel all open orders

# Fill orders
indodax paper fill -i 1                # Fill order 1
indodax paper fill -i 2 --price 110000000  # Fill at custom price
indodax paper fill --all               # Fill all open orders

# Auto-fill based on market prices
indodax paper check-fills -p '{"btc_idr": 95000000, "eth_idr": 12000000}'

# Top up balances
indodax paper topup -c usdt -a 50000

# Reset to initial state
indodax paper reset
```

> **Tip:** Paper trading uses the same command structure as live trading. Switch between modes by replacing `trade` with `paper`.

---

## 🔔 Price Alerts

Set price alerts and get notified when conditions are met. Alerts can be checked on-demand or monitored in real-time via WebSocket.

```bash
# Price threshold alerts
indodax alert add -p btc_idr --above 150000000
indodax alert add -p btc_idr --below 50000000

# Percentage change alerts (based on current price)
indodax alert add -p btc_idr --percent-up 5
indodax alert add -p btc_idr --percent-down 10

# Add note to alert
indodax alert add -p btc_idr --above 200000000 -n "Take profit target"

# List and manage alerts
indodax alert list
indodax alert list --history
indodax alert cancel -i 1
indodax alert cancel --all

# One-time check (HTTP polling)
indodax alert check
indodax alert check -p btc_idr

# Real-time monitoring (WebSocket)
indodax alert watch
indodax alert watch -p btc_idr
```

> **Tip:** Use `indodax alert watch` for real-time price monitoring. Alerts are stored in `~/.config/indodax/alerts.json`.

---

## 🔐 Authentication & Security

Indodax uses **HMAC-SHA512** signing for API authentication. Your credentials are stored securely:

- Config file uses **`0600` permissions** (owner read/write only)
- Supports environment variables for CI/CD workflows
- CLI flags override everything for one-off commands

### Withdrawal Callback URL

For withdrawals, Indodax requires a Callback URL to validate requests:

```bash
indodax auth set --callback-url https://yourdomain.com/callback
```

Then run the validation server:

```bash
indodax funding serve-callback --port 8080
```

---

## 🏗️ Architecture

This project is inspired by the [Kraken CLI](https://github.com/krakenfx/kraken-cli) architecture and built with modern Rust:

- **`clap`** — powerful derive-based CLI parsing
- **`tokio`** — async runtime for non-blocking I/O
- **`tokio-tungstenite`** — WebSocket client for real-time streams
- **`reqwest`** — HTTP client for REST API calls
- **`serde`** — robust serialization/deserialization
- **`comfy-table`** — beautiful terminal tables
- **`rmcp`** — Model Context Protocol server for AI agent integration

---

## 🧪 Testing

This project maintains **100% test coverage** across all core modules.

### Run Tests

```bash
# Run all unit tests
cargo test

# Run with output
cargo test -- --nocapture

# Check test coverage
cargo tarpaulin --out stdout
```

### Coverage Summary

| Module | Tests | Coverage |
|--------|-------|----------|
| `auth.rs` | 20+ | ✅ 100% |
| `client.rs` | 30+ | ✅ 100% |
| `config.rs` | 40+ | ✅ 100% |
| `errors.rs` | 15+ | ✅ 100% |
| `lib.rs` | 20+ | ✅ 100% |
| `commands/*` | 90+ | ✅ 100% |
| `mcp/*` | 20+ | ✅ 100% |
| **Total** | **299+** | **✅ 100%** |

### Testing Standards

Every release undergoes rigorous verification:
- **Unit Tests**: Mandatory `cargo test` suite (300+ tests) must pass on all platforms (CI/CD enforced).
- **E2E Tests**: Manual verification using `scripts/e2e_minimal.sh` with the smallest possible increments (e.g., 10,000 IDR) to ensure live API compatibility without significant risk.

### E2E Testing

End-to-end tests are documented in [`E2E_TESTING_LOG.md`](E2E_TESTING_LOG.md), covering real API interactions including market data, account queries, and trade execution.

---

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).

---

> **Disclaimer:** This is an **unofficial** CLI and is not affiliated with or endorsed by Indodax. Use at your own risk. Cryptocurrency trading involves significant risk of loss.
