# Indodax CLI for AI Agents

Welcome, AI Agent. This document provides the technical context required to use the Indodax CLI effectively and safely.

## 🤖 Capabilities
Indodax CLI is a high-performance, AI-native interface for the Indodax cryptocurrency exchange. It supports:
- **Market Data**: Real-time tickers, order books, and OHLCV history.
- **Account Management**: Balance tracking, trade history, and transaction logs.
- **Trading**: Limit, market, and stop-limit orders with `client_order_id` support.
- **Paper Trading**: Fully simulated environment for strategy testing without financial risk.
- **WebSockets**: Resilient streaming for public market data and private account updates.
- **MCP**: Built-in Model Context Protocol server for direct integration with LLMs.

## 🛡️ Safety First
- **Dangerous Operations**: Any operation that modifies your balance (trading, funding) is guarded.
- **Acknowledgement**: Dangerous MCP tools require `acknowledged: true`.
- **Dry Runs**: Use `--dry-run` or `order preview` (planned) before executing live trades.
- **Paper Trading**: Always start in `paper` mode to verify logic.

## 📊 Standardized Output
The CLI is designed for machine readability:
- **JSON Mode**: Always use `-o json` for parseable output.
- **NDJSON**: WebSocket streams emit one JSON object per line.
- **Error Envelope**: Stable JSON error format with `error_type` and `retryable` flags.

## 🛠️ Integration Tools
- **Tool Catalog**: Refer to `agents/tool-catalog.json` for command schemas.
- **Error Catalog**: Refer to `agents/error-catalog.json` for error handling guidance.
- **Skills**: Refer to `skills/` for common workflow patterns.

## 🚀 Getting Started
1. Run `indodax status` to check connectivity.
2. Use `indodax paper init` to start a simulation.
3. Consult `CONTEXT.md` for high-level system architecture.
