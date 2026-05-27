# Stage 1: Build
FROM rust:1.95-slim-bookworm AS builder

WORKDIR /usr/src/app
COPY . .

# Install dependencies untuk build
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Build binary dengan fitur server & mcp
RUN cargo build --release --features "mcp server cli"

# Stage 2: Runtime
FROM debian:bookworm-slim

# Install runtime dependencies (OpenSSL)
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /usr/src/app/target/release/indodax-cli /usr/local/bin/indodax-cli

# Port default untuk HTTP Bridge
EXPOSE 8000

# Jalankan server MCP mode HTTP secara default
# Kita gunakan 0.0.0.0 agar bisa diakses dari luar container
ENTRYPOINT ["indodax-cli", "mcp", "--http", "--port", "8000", "--groups", "all", "--allow-dangerous"]
