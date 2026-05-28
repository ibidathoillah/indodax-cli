# Indodax CLI Agent Guide

This repository exposes Indodax market, account, trading, funding, paper trading, auth, alert, and WebSocket workflows through a CLI and an MCP server.

## Setup

Build the full local tool:

```sh
cargo build --features cli,mcp,server
```

Configure credentials for private endpoints:

```sh
indodax auth set --api-key "$INDODAX_API_KEY" --api-secret "$INDODAX_API_SECRET"
```

Public market commands do not require credentials. Private account, trade, funding, and private WebSocket commands do.

## MCP

Start stdio MCP:

```sh
indodax mcp --groups market,account,paper,auth
```

Enable dangerous operations explicitly:

```sh
indodax mcp --groups all --allow-dangerous
```

Trade and funding tools require an `acknowledged: true` argument unless the server is started with `--allow-dangerous`.

## Catalogs

Agent-facing catalogs live in:

- `agents/tool-catalog.json`
- `agents/error-catalog.json`

Use the tool catalog to discover parameters and safety levels before calling MCP tools. Use the error catalog to decide whether retrying is appropriate.

## Safety

Never call dangerous tools speculatively. Confirm intent, validate balances and parameters, and prefer read-only tools first:

- `ticker`, `orderbook`, `balance`, `open_orders`
- `withdraw_fee` before `withdraw`
- `ws_token` or `generate_ws_token` before private WebSocket connections

## Common Examples

```sh
indodax ticker btc_idr
indodax account info
indodax account trans-history --start 2026-05-01 --end 2026-05-28
indodax order buy --pair btc_idr --idr 100000 --price 1000000000 --client-order-id bot_001 --time-in-force GTC
indodax ws generate-token
```
