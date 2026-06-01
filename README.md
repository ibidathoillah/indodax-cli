# Indodax CLI & MCP Server

[![indodax-cli MCP server](https://glama.ai/mcp/servers/ibidathoillah/indodax-cli/badges/card.svg)](https://glama.ai/mcp/servers/ibidathoillah/indodax-cli)

Professional command-line interface and **Model Context Protocol (MCP)** server for the Indodax cryptocurrency exchange.

## 🤖 AI Agent & Mobile Support (MCP)
This project is now a fully-featured MCP server, allowing AI agents (like ChatGPT, Claude, and Glama) to interact with Indodax.

- **Desktop**: Use with Claude Desktop for a local trading experience.
- **Mobile**: Use via ChatGPT Actions or Glama with our built-in **Isolated HTTP Bridge**.
- **Features**: Market data, private balances, trading, and risk-free paper trading.

👉 **[Read the MCP Full Feature Guide & Setup](./src/mcp/README.md)**

---

Unofficial Rust CLI for Indodax.
 Use it to inspect markets, manage account data, place spot orders, stream live WebSocket events, run paper trading and price alerts, and expose the same command surface to agents through MCP.

[![Rust](https://img.shields.io/badge/Rust-2021-000000?logo=rust)](https://www.rust-lang.org/)
[![CLI](https://img.shields.io/badge/interface-terminal-2f855a)](#quick-start)
[![WebSocket](https://img.shields.io/badge/websocket-live-2563eb)](#websocket-streaming)
[![MCP](https://img.shields.io/badge/MCP-ready-7c3aed)](#mcp-server)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

## Highlights

- Public market data: server time, pairs, ticker, all tickers, summaries, order book, trades, OHLC, and price increments.
- Private account data: account info, balances, transactions, and trade history.
- Spot trading: buy, sell, cancel, cancel by client order ID, cancel all, and deadman countdown.
- Funding: withdrawal fee lookup, crypto withdrawal, and withdrawal callback validation server.
- Real-time streams: ticker, trades, order book, summary, and private order updates.
- Paper trading: risk-free simulated trading with balances, orders, fills, history, and status.
- Price alerts: threshold and percentage alerts with one-shot checks or live WebSocket monitoring.
- Automation-friendly output: human tables by default, JSON envelopes with `-o json`.
- Credential resolution: CLI flags, environment variables, or `~/.config/indodax/config.toml`.
- Agent support: MCP server mode with guarded dangerous operations.

## Recent Highlights (v0.1.56)

- **MCP server overhaul**: Alert tools, deposit address, auth configuration, WebSocket snapshot tools, Resources, and Prompts support.
- WebSocket reliability overhaul: application-level pings, automatic reconnection with exponential backoff, and private WebSocket support for real-time order and balance updates.
- Secure WebSocket authentication: configurable WebSocket tokens with fallback to a stable default.
- TradeAPI-2 compliance: normalized symbol formats and stricter request handling across commands.
- Response parsing improvements: order book handling supports both legacy and modern API shapes.

## Installation

Install from source:

```bash
git clone https://github.com/ibidathoillah/indodax-cli.git
cd indodax-cli
cargo install --path .
```

Install from crates.io:

```bash
cargo install indodax-cli
```

Install from npm:

```bash
npm install -g indodax-cli
```

On Android/Termux, `npm install -g indodax-cli` downloads the Android release binary when available. If the release asset is missing, it falls back to a local `cargo build`, so `cargo` still needs to be installed in Termux for that fallback path.

Run with Docker:

```bash
docker run --rm ibidathoillah/indodax-cli --help
docker run --rm -v ~/.config/indodax:/root/.config/indodax ibidathoillah/indodax-cli balance
```

Run from the checkout:

```bash
cargo build
./target/debug/indodax --help
```

## Quick Start

Market data does not require credentials:

```bash
indodax server-time
indodax ticker btc/idr
indodax orderbook btc/idr --count 10
indodax pairs
indodax ohlc --pair btc/idr
indodax -o json ticker btc/idr
```

Configure private API credentials:

```bash
indodax auth set --api-key YOUR_API_KEY --api-secret YOUR_API_SECRET
indodax auth test
indodax auth show
```

Or use environment variables:

```bash
export INDODAX_API_KEY=your_api_key
export INDODAX_API_SECRET=your_api_secret
```

Credential priority:

1. `--api-key` and `--api-secret`
2. `INDODAX_API_KEY` and `INDODAX_API_SECRET`
3. `~/.config/indodax/config.toml`

## Command Reference

Global options:

```text
indodax [OPTIONS] <COMMAND>

Options:
  -o, --output <table|json>      Output format [default: table]
      --api-key <API_KEY>        API key override
      --api-secret <API_SECRET>  API secret override
      --api-secret-stdin         Read API secret from stdin
  -v, --verbose                  Enable verbose logs
      --yes, --force             Skip confirmation prompts
```

### Market

```bash
indodax server-time
indodax pairs
indodax ticker btc/idr
indodax ticker-all
indodax summaries
indodax orderbook btc/idr --count 10
indodax trades btc/idr
indodax ohlc --pair btc/idr --interval 60
indodax history btc/idr --timeframe 15 --from 1779158061 --to 1779454161
indodax webdata btc/idr
indodax chat-history
indodax pairs-v2
indodax search-v2
indodax terminal-trade usdt/idr
indodax terminal-market usdt/idr
indodax terminal-categories
indodax onramp-config usdt/idr
indodax news btc
indodax price-increments
```

### Account

```bash
indodax account-info
indodax balance
indodax transactions
indodax trades-history btc/idr --limit 5
```

### Trading

```bash
indodax order buy --pair btc/idr --idr 100000 --price 1000000000
indodax order buy --pair btc/idr --idr 100000 --order-type market
indodax order sell --pair btc/idr --amount 0.001 --price 1000000000
indodax order cancel --order-id 123456 --pair btc/idr --order-type buy
indodax order cancel-by-client-id --client-order-id CLIENT_ID
indodax --yes order cancel-all --pair btc/idr
indodax order countdown --pair btc/idr --countdown-time 60000
```

### Funding

```bash
indodax withdrawal fee --asset btc
indodax withdraw --asset btc --volume 0.001 --address bc1... --network BTC
indodax withdrawal serve-callback --port 8080
```

For withdrawals, Indodax may require a callback URL. Configure it with:

```bash
indodax auth set --callback-url https://yourdomain.com/callback
```

### WebSocket Streaming

Public streams:

```bash
indodax ws ticker btc/idr
indodax ws trades btc/idr
indodax ws book btc/idr
indodax ws summary
```

Private stream:

```bash
indodax ws orders
```

### Price Alerts

```bash
indodax alert add -p btc/idr --above 150000000
indodax alert add -p btc/idr --below 50000000
indodax alert add -p btc/idr --percent-up 5
indodax alert add -p btc/idr --percent-down 10
indodax alert list
indodax alert list --history
indodax alert cancel -i 1
indodax alert cancel --all
indodax alert check
indodax alert watch -p btc/idr
indodax alert triggered
```

Alerts are stored in `~/.config/indodax/alerts.json`.

### Paper Trading

```bash
indodax paper init
indodax paper init --idr 50000000 --btc 0.5
indodax paper balance
indodax paper buy -p btc/idr -i 1000000
indodax paper buy -p btc/idr -a 0.1 -r 500000000
indodax paper sell -p btc/idr -a 0.05 -r 1000000000
indodax paper orders --pair btc/idr
indodax paper cancel -i 1
indodax paper cancel-all
indodax paper fill -i 1
indodax paper fill -i 2 --price 110000000
indodax paper fill --all
indodax paper check-fills -p '{"btc/idr": 95000000, "eth/idr": 12000000}'
indodax paper topup -c usdt -a 50000
indodax paper history
indodax paper status
indodax paper reset
```

Paper trading mirrors the live order interface for safer strategy testing.

### Interactive Shell

```bash
indodax shell
```

### MCP Server

```bash
indodax mcp
indodax mcp -s all
indodax mcp -s all --allow-dangerous
indodax mcp -s market,trade,paper
```

Service groups:

| Group | Tools | Auth Required | Dangerous |
|-------|-------|---------------|-----------|
| `market` | Server time, ticker, pairs, orderbook, trades, OHLC, price increments, WS snapshots | No | No |
| `account` | Balance, trade history, transactions, account info, open orders, order history, get order | Yes | No |
| `trade` | Buy, sell, cancel, cancel all orders | Yes | Yes |
| `funding` | Withdraw fees, withdraw crypto, deposit address | Yes | Yes |
| `paper` | Paper trading init, balance, buy, sell, orders, cancel, fill, history, status | No | No |
| `auth` | Show config, test credentials, set credentials | Varies | No |
| `alert` | Add, list, cancel, check price alerts | No | No |

By default, `trade` and `funding` MCP tools require `acknowledged: true`. Use `--allow-dangerous` only for controlled local automation.

#### MCP Resources

The server exposes read-only resources that MCP clients can inspect:

| Resource URI | Description |
|---|---|
| `config://current` | Current API configuration status |
| `pairs://list` | All available trading pairs from the API |
| `paper://state` | Current paper trading state (balances, orders count) |

#### MCP Prompts

Pre-built prompt templates for common workflows:

| Prompt | Arguments | Description |
|---|---|---|
| `create_order` | `side`, `pair`, `price`, `amount`, `idr` | Generate buy/sell order with safety checks |
| `check_portfolio` | (none) | Account balance and open orders overview |
| `analyze_market` | `pair` | Market analysis with ticker, order book, and trades |

### Glama Deployment

This server is optimized for deployment on [Glama](https://glama.ai/mcp).

- **Manifest**: `glama.json` defines the required environment variables for the build spec.
- **Docker**: The included `Dockerfile` builds only the CLI, MCP, and callback-server features, installs the `indodax-cli` binary, and also exposes an `indodax` alias for clients that use the shorter command name.
- **Environment Variables**: Configure `INDODAX_API_KEY` and `INDODAX_API_SECRET` in the Glama admin panel.

Use this Glama build spec:

```json
{
  "baseImage": "debian:trixie-slim",
  "buildSteps": [
    "apt-get update && apt-get install -y --no-install-recommends ca-certificates curl git pkg-config build-essential && rm -rf /var/lib/apt/lists/*",
    "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y",
    ". $HOME/.cargo/env && cargo build --release --features cli,mcp,server",
    "cp target/release/indodax-cli /usr/local/bin/indodax-cli && ln -s /usr/local/bin/indodax-cli /usr/local/bin/indodax"
  ],
  "cmdArguments": [
    "mcp-proxy",
    "--",
    "indodax-cli",
    "mcp"
  ],
  "nodeVersion": "26",
  "pythonVersion": "3.14"
}
```

If Glama fails while loading Docker metadata, such as `ECONNRESET` during `load metadata for docker.io/library/debian:trixie-slim`, retry the build. That failure happens before this project is cloned or compiled.

Example configuration for manual setup:

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

## Output Formats

Table mode is the default:

```bash
indodax ticker btc/idr
```

JSON mode is intended for scripting and automation:

```bash
indodax -o json ticker btc/idr
```

Error responses in JSON mode use structured envelopes:

```json
{
  "error": true,
  "message": "Invalid trading pair: xxx_idr",
  "error_type": "invalid_pair",
  "retryable": false
}
```

## E2E Testing

The repository includes live API smoke tests:

```bash
./scripts/e2e_minimal.sh --public
./scripts/e2e_minimal.sh --private
./scripts/e2e_minimal.sh --ws
./scripts/e2e_websocket_mock.sh
```

Environment knobs:

```bash
INDODAX_BIN=./target/debug/indodax
```

Latest local verification:

```text
cargo test: 296 passed
./scripts/e2e_minimal.sh --public: passed
./scripts/e2e_minimal.sh --private: skipped private checks (credentials unavailable)
./scripts/e2e_minimal.sh --ws: passed
```

## API Coverage

- Public REST: Indodax market endpoints
- Private REST: Indodax TradeAPI and TradeAPI-2 endpoints
- Public WebSocket: `wss://ws3.indodax.com/ws/`
- Private WebSocket: `wss://pws.indodax.com/ws/`

## Architecture

This project is inspired by the Kraken CLI architecture and built with modern Rust:

- `clap` for derive-based CLI parsing
- `tokio` for async runtime
- `tokio-tungstenite` for WebSocket streams
- `reqwest` for REST API calls
- `serde` for serialization and deserialization
- `comfy-table` for terminal tables
- `rmcp` for Model Context Protocol support

## Testing Standards

Every release should pass:

- Unit tests with `cargo test`
- Public E2E smoke tests
- WebSocket smoke tests
- Private read-only tests when credentials are available

Coverage is maintained across `auth`, `client`, `config`, `errors`, `commands`, `mcp`, and `output` modules.

## Security

- Credentials are stored with `0600` permissions when using `indodax auth set`.
- HMAC-SHA512 signing is used for private API authentication.
- Prefer read-only API keys for account inspection and WebSocket monitoring.
- Use least-privilege exchange API keys for MCP and automation.
- Never commit real API keys, secrets, callback tokens, or listen keys.

## Development

```bash
cargo fmt
cargo test
cargo build
```

## Contributing

Contributions are welcome:

1. Fork the repository.
2. Create a feature branch.
3. Run tests and relevant E2E smoke checks.
4. Open a pull request.

## Related Projects

If you use multiple exchanges, check out these related CLI tools built with the same architecture:

- [indodax-cli](https://github.com/ibidathoillah/indodax-cli) - CLI for Indodax
- [bittime-cli](https://github.com/ibidathoillah/bittime-cli) - CLI for Bittime
- [binance-cli](https://github.com/ibidathoillah/binance-cli) - CLI for Binance Spot
- [tokocrypto-cli](https://github.com/ibidathoillah/tokocrypto-cli) - CLI for Tokocrypto
- [kraken-cli](https://github.com/ibidathoillah/kraken-cli) - CLI for Kraken (Spot, Margin, Futures)

## License

MIT

## Disclaimer

This project is unofficial and is not affiliated with or endorsed by Indodax. Cryptocurrency trading is risky; review commands carefully before using write-capable API keys.
