# Indodax CLI (Unofficial)

An unofficial command-line interface for the [Indodax](https://indodax.com) cryptocurrency exchange.

Built with Rust, mirroring the [Kraken CLI](https://github.com/krakenfx/kraken-cli) architecture.

## Quick Start

### Install from source

```bash
cargo install --path .
```

### Or build locally

```bash
cargo build --release
./target/release/indodax --help
```

## Configuration

Configure your API credentials:

```bash
indodax auth set --api-key YOUR_API_KEY --api-secret YOUR_API_SECRET
```

Or use environment variables:

```bash
export INDODAX_API_KEY=your_api_key
export INDODAX_API_SECRET=your_api_secret
```

The config file is stored at `~/.config/indodax/config.toml` with `0600` permissions.

## Usage

```
indodax [OPTIONS] <COMMAND>

Options:
  -o, --output <OUTPUT>           Output format: table or json [default: table]
      --api-key <API_KEY>         API key (overrides config file and env var)
      --api-secret <API_SECRET>   API secret (overrides config file and env var)
  -v, --verbose                   Enable verbose output
  -h, --help                      Print help
  -V, --version                   Print version
```

## Commands

### Market Data (Public API)

| Command | Description |
|---------|-------------|
| `indodax market server-time` | Get server time |
| `indodax market pairs` | List available trading pairs |
| `indodax market ticker <pair>` | Get ticker for a pair |
| `indodax market ticker-all` | Get tickers for all pairs |
| `indodax market summaries` | Get 24h and 7d summaries |
| `indodax market orderbook <pair>` | Get order book |
| `indodax market trades <pair>` | Get recent trades |
| `indodax market ohlc` | Get OHLCV candle data |
| `indodax market price-increments` | Get tick sizes |

### Account (Private API)

| Command | Description |
|---------|-------------|
| `indodax account info` | Get account information |
| `indodax account balance` | Show balances |
| `indodax account open-orders` | List open orders |
| `indodax account order-history` | Get order history (v2 API) |
| `indodax account trade-history` | Get trade fill history (v2 API) |
| `indodax account trans-history` | Get deposit/withdrawal history |
| `indodax account get-order` | Get order details |

### Trading (Private API)

| Command | Description |
|---------|-------------|
| `indodax trade buy` | Place a buy order |
| `indodax trade sell` | Place a sell order |
| `indodax trade cancel` | Cancel an order by ID |
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

### Paper Trading (Simulated)

| Command | Description |
|---------|-------------|
| `indodax paper init` | Initialize paper trading |
| `indodax paper reset` | Reset paper trading state |
| `indodax paper balance` | Show paper balances |
| `indodax paper buy` | Simulated buy order |
| `indodax paper sell` | Simulated sell order |
| `indodax paper orders` | List paper orders |
| `indodax paper cancel` | Cancel a paper order |
| `indodax paper cancel-all` | Cancel all paper orders |
| `indodax paper history` | Show paper trade history |
| `indodax paper status` | Show paper trading status |

### Auth Management

| Command | Description |
|---------|-------------|
| `indodax auth set` | Set API credentials |
| `indodax auth show` | Show current config |
| `indodax auth test` | Test API credentials |
| `indodax auth reset` | Remove stored credentials |

### Utility

| Command | Description |
|---------|-------------|
| `indodax setup` | Interactive setup wizard |
| `indodax shell` | Start interactive REPL |

## Output Formats

Table mode (default):
```
indodax market ticker btc_idr
```

JSON mode:
```
indodax -o json market ticker btc_idr
```

## Paper Trading

Paper trading provides a simulated trading environment with virtual balances:

```bash
# Initialize with default balances (100M IDR, 1 BTC)
indodax paper init

# Place simulated orders
indodax paper buy --pair btc_idr --price 500000000 --amount 0.1
indodax paper sell --pair btc_idr --price 600000000 --amount 0.05

# Check balances
indodax paper balance

# View status
indodax paper status
```

## Authentication

Indodax uses HMAC-SHA512 signing for API authentication. Credentials are resolved in this order:

1. CLI flags (`--api-key`, `--api-secret`)
2. Environment variables (`INDODAX_API_KEY`, `INDODAX_API_SECRET`)
3. Config file (`~/.config/indodax/config.toml`)

### Callback URL

For withdrawals, Indodax requires a Callback URL to validate the request. You can set this in your config:

```bash
indodax auth set --callback-url https://indodax.tep2.in/
```

Then run the server to handle incoming validation requests:

```bash
indodax funding serve-callback --port 8081
```

## License

MIT
