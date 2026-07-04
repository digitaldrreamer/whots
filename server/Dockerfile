# ── Stage 1: chef (dependency layer cache) ────────────────────────────────────
FROM rust:1-slim-bookworm AS chef
# webauthn-rs pulls in openssl-sys, which needs the OpenSSL headers + pkg-config
# to build (the rest of the stack is rustls). Install them here so the shared
# chef base covers both `cargo chef cook` and the real build.
RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
    && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef --locked
WORKDIR /app

# ── Stage 2: planner ──────────────────────────────────────────────────────────
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ── Stage 3: builder ──────────────────────────────────────────────────────────
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build only deps first (cached unless Cargo.toml changes)
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
# We use sqlx's runtime (builder) API throughout — no query! macros — so no
# live database is needed at compile time.
RUN cargo build --release --bin whots-server

# ── Stage 4: runtime ──────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/whots-server /usr/local/bin/whots-server

EXPOSE 3001
CMD ["whots-server"]
