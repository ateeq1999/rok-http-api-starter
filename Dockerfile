FROM rust:1.88-slim AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

ENV CARGO_HTTP_TIMEOUT=300
ENV CARGO_NET_RETRY=3

COPY Cargo.toml Cargo.lock ./

RUN mkdir -p src && echo 'fn main() {}' > src/main.rs

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build --release 2>/dev/null || true

COPY src/ src/

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    touch src/main.rs \
    && cargo build --release

FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 1001 appuser
WORKDIR /app
COPY --from=builder /app/target/release/rok-api-start ./

RUN chown -R appuser:appuser /app
USER appuser

ENV LISTEN_ADDR=0.0.0.0:8080
EXPOSE 8080

ENTRYPOINT ["./rok-api-start"]
