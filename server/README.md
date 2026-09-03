# Whots Backend Server 🦀

The game server, WebSocket room manager, and AI engine for Whots, built with [Axum](https://github.com/tokio-rs/axum) and [Tokio](https://tokio.rs/).

---

## Architecture Overview

```
server/
├── migrations/           # SQLx database schema migrations
├── src/
│   ├── auth/             # JWT token handling and password hashing (Argon2)
│   ├── game/             # Game rules engine, card deck, actions, and AI
│   │   ├── ai/           # Multi-tiered AI engine & ISMCTS Monte Carlo search
│   │   ├── engine.rs     # Core game state transition logic
│   │   └── types.rs      # Cards, actions, moves, and game phase types
│   ├── routes/           # Axum REST endpoints and WebSocket room handlers
│   ├── store/            # PostgreSQL queries and Redis pub/sub state
│   ├── bin/              # Standalone binaries for AI tuning and evaluation
│   ├── config.rs         # Environment variable configuration loader
│   ├── lib.rs            # Axum router construction & app initialization
│   └── main.rs           # Server entrypoint
└── tests/                # Integration test suite
```

---

## Local Development Setup

### 1. Prerequisites
- Rust 1.75+
- PostgreSQL 16+
- Redis 7+

### 2. Configure Environment
```bash
cp .env.example .env
```

Edit `.env` to configure your PostgreSQL credentials and JWT secret.

### 3. Run the Server
```bash
cargo run
```
On startup, the server automatically runs all pending migrations in `migrations/` and listens on `http://localhost:3001`.

---

## Standalone AI Binaries

The server crate includes several CLI binaries in `src/bin/`:

- **`tuner`**: Coordinate-descent optimizer for AI difficulty parameters.
  ```bash
  cargo run --release --bin tuner
  ```
- **`ladder`**: Round-robin tournament simulation across all AI tiers.
  ```bash
  cargo run --release --bin ladder
  ```
- **`sweep`**: Parameter sweep across individual AI heuristic modules.
  ```bash
  cargo run --release --bin sweep
  ```
- **`identity`**: Benchmarks playouts and ISMCTS node search consistency.
  ```bash
  cargo run --release --bin identity
  ```

---

## Testing

Run unit and integration tests:
```bash
cargo test
```

Check code formatting and run Clippy lints:
```bash
cargo fmt --check
cargo clippy -- -D warnings
```
