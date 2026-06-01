# System Context for Indodax CLI

## 🌍 Overview
Indodax CLI is a Rust-based tool designed to bridge the gap between traditional exchange APIs and AI-driven trading/management. It prioritizes stability, security, and machine-readable transparency.

## 🏗️ Architecture
- **Client**: Asynchronous Reqwest-based client with rate-limiting and automatic retries.
- **Commands**: Modular structure following a hierarchy: `market`, `account`, `trade`, `funding`, `ws`, `paper`.
- **Paper Trading**: Local state management using JSON files in the user's config directory.
- **MCP Server**: Implements the Model Context Protocol (stdio/http) for tool-use by LLMs.
- **Output Layer**: Flexible rendering for both humans (Tables) and machines (JSON).

## 🔑 Authentication
- Supports API Key/Secret pairs.
- Credentials can be loaded via config file, environment variables (`INDODAX_API_KEY`, `INDODAX_API_SECRET`), or CLI flags.
- Secure prompt for API secrets via stdin.

## 📡 Connectivity
- **REST V1/V2**: Used for transactional and historical data.
- **WebSockets**: Used for real-time market data and account event streaming.
- **Retry Logic**: Exponential backoff for transient network errors.

## ⚠️ Important Constraints
- Indodax is primarily a **Spot Exchange**. No futures or margin trading are currently supported in this CLI.
- Asset pairs usually follow the `base_quote` format (e.g., `btc_idr`).
- Some endpoints are rate-limited per IP; prefer WebSockets for high-frequency polling.
