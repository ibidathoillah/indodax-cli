FROM rust:1-slim-trixie AS builder

WORKDIR /app
COPY . .

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config build-essential ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN cargo build --release --features cli,mcp,server

FROM debian:trixie-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/indodax-cli /usr/local/bin/indodax-cli
RUN ln -s /usr/local/bin/indodax-cli /usr/local/bin/indodax

ENTRYPOINT ["indodax-cli"]
CMD ["mcp"]
