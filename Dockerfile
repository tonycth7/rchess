# ── Build stage ───────────────────────────────────────────────────────────────
FROM rust:1.78-slim AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY server/ server/
COPY src/ src/

# Build only the server binary (no GUI deps needed)
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config && \
    cargo build --release --bin rchess-server && \
    strip target/release/rchess-server

# ── Runtime stage ─────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN useradd -m -u 1001 rchess
WORKDIR /app

COPY --from=builder /app/target/release/rchess-server /usr/local/bin/rchess-server

# Data directory (mount a volume here for persistence)
RUN mkdir -p /app/rchess_data && chown rchess:rchess /app/rchess_data

USER rchess

# Game port | Admin dashboard
EXPOSE 9001 9002

# Environment overrides (alternative to CLI flags)
# ENV RCHESS_PUBLIC_HOST=chess.example.com

VOLUME ["/app/rchess_data"]

ENTRYPOINT ["rchess-server"]
CMD ["--host", "0.0.0.0", "--port", "9001", "--admin", "9002", "--data", "/app/rchess_data"]
