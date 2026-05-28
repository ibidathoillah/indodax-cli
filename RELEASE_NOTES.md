## 🚀 Welcome to Indodax CLI v0.1.46

The unofficial, fast, and feature-rich command-line interface for **Indodax**, Indonesia's largest cryptocurrency exchange.

### 🆕 What's New in v0.1.46

- **🌐 Production-Ready MCP HTTP Bridge**:
  - Integrated a high-performance HTTP server directly into the binary using **Axum**.
  - Supports **Isolated Multi-User Bridge** via custom headers (`x-api-key`, `x-api-secret`).
  - Added **X-Bridge-Auth** security layer to protect the server from unauthorized access.
  - Full tool coverage for HTTP transport (Market, Account, Trading, Paper, and Alerts).
  - Built-in **CORS support** for browser-based tool integrations.

- **🛡️ Security & Stability**:
  - Implemented secure credential isolation; API keys are never stored on the server.
  - Added **Automated CI/CD** via GitHub Actions for testing and Docker publishing.
  - Enhanced **E2E Testing** for HTTP bridge authentication and tool dispatching.
  - Replaced experimental library logic with a stable **Direct Tool Dispatcher**.

- **📦 Deployment & Integration**:
  - Added **Docker Compose** support for one-command deployment (Server + Cloudflare Tunnel).
  - Updated **OpenAPI Spec (v1.2)** for smarter ChatGPT Actions integration.
  - Revamped **MCP Documentation** in `src/mcp/README.md`.
  - Optimized Docker image with multi-stage build.

### 🆕 What's New in v0.1.46

- **🚀 Glama Deploy Fixes**:
  - Updated the Docker image to Debian Trixie and removed unnecessary OpenSSL runtime dependencies.
  - Limited hosted builds to the CLI, MCP, and callback-server features needed for deployment.
  - Added an `indodax` command alias alongside the `indodax-cli` binary for MCP clients.
  - Added `.dockerignore` to keep hosted Docker build contexts small and predictable.
  - Documented the exact Glama build spec and the transient `ECONNRESET` metadata-fetch failure mode.

... (sisanya tetap sama)
