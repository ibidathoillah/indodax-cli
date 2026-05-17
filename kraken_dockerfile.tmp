# Build stage
FROM rust:1-slim-bullseye AS builder

WORKDIR /app
COPY . .

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
RUN cargo build --release

# Final stage
FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y ca-certificates libssl1.1 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/kraken /usr/local/bin/kraken

ENTRYPOINT ["kraken"]
