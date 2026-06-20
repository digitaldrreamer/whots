# ── Stage 1: chef (dependency layer cache) ────────────────────────────────────
FROM rust:1.78-slim AS chef
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
# Disable sqlx offline check — migrations are embedded via sqlx::migrate!
ENV SQLX_OFFLINE=false
# No live DB at build time; we rely on the builder's own offline-mode files if present,
# or set SQLX_OFFLINE=true and commit the .sqlx query cache.
RUN cargo build --release --bin whots-server

# ── Stage 4: runtime ──────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/whots-server /usr/local/bin/whots-server

EXPOSE 3001
CMD ["whots-server"]
