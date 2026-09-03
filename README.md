# Whots 🃏🇳🇬

A modern, full-stack digital implementation of the beloved **Nigerian Whot** card game. Built with a high-performance **Rust / Axum** game engine, real-time **WebSockets**, an **Information Set Monte Carlo Tree Search (ISMCTS)** AI bot ladder, **WebAuthn passkeys**, and a responsive **SvelteKit** client with procedural Web Audio sound design.

🎮 **Play Live**: [http://whots.drreamer.digital](http://whots.drreamer.digital/)

[![Play Online](https://img.shields.io/badge/Play%20Online-whots.drreamer.digital-00c853?style=for-the-badge&logo=googlechrome&logoColor=white)](http://whots.drreamer.digital/)

[![CI](https://github.com/digitaldrreamer/whots/actions/workflows/ci.yml/badge.svg)](https://github.com/digitaldrreamer/whots/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![SvelteKit](https://img.shields.io/badge/SvelteKit-2.0-red.svg)](https://kit.svelte.dev/)
[![PostgreSQL](https://img.shields.io/badge/PostgreSQL-16-blue.svg)](https://www.postgresql.org/)
[![Redis](https://img.shields.io/badge/Redis-7-red.svg)](https://redis.io/)

---

## Table of Contents

- [The Game](#the-game)
  - [The Deck](#the-deck)
  - [Action Cards](#action-cards)
  - [Game Modes](#game-modes)
- [Screenshots](#screenshots)
- [Features](#features)
- [Architecture & Tech Stack](#architecture--tech-stack)
- [Quickstart (Local Development)](#quickstart-local-development)
  - [Option A: Docker Compose (Fastest)](#option-a-docker-compose-fastest)
  - [Option B: Running From Source](#option-b-running-from-source)
- [Environment Variables](#environment-variables)
- [AI Difficulty Ladder & Tuner](#ai-difficulty-ladder--tuner)
- [Testing & Quality](#testing--quality)
- [Production Deployment](#production-deployment)
- [Contributing](#contributing)
- [License](#license)

---

## The Game

Whots is a popular national card game in Nigeria. Players match cards by **shape** or **number**, using special action cards to disrupt opponents. The first player to shed all their cards wins ("Check up!").

### The Deck

The deck consists of **54 cards** across **5 shapes (suits)** plus wild cards:

- ⭕ **Circle**: 1, 2, 3, 4, 5, 7, 8, 10, 11, 12, 13, 14
- 🔺 **Triangle**: 1, 2, 3, 4, 5, 7, 8, 10, 11, 12, 13, 14
- ✝️ **Cross**: 1, 2, 3, 5, 7, 10, 11, 13, 14
- ⏹️ **Square**: 1, 2, 3, 5, 7, 10, 11, 13, 14
- ⭐ **Star**: 1, 2, 3, 4, 5, 7, 8 (Star counts double in penalty points!)
- 🃏 **Whot (20)**: 4 wild cards that can be played onto any card to demand any desired shape.

### Action Cards

| Card Number | Traditional Name | Effect |
|:---:|:---|:---|
| **1** | **Hold On** | Skips the next player. The player who played it can immediately follow with another card of the same shape or number. |
| **2** | **Pick Two** | Next player draws 2 cards from the stock pile. |
| **5** | **Pick Three** | Next player draws 3 cards from the stock pile. |
| **8** | **Suspension** | Suspends the next player's turn. |
| **14** | **General Market** | Every other player must draw 1 card from the stock pile. |
| **20** | **Whot (Wild)** | Can be played on any card. The player names the shape required for subsequent turns. |

### Game Modes

- **Stack Mode**: Penalty cards (2s and 5s) can be countered by consecutive matching cards, accumulating the penalty onto the next player until someone cannot counter.
- **No-Stack Mode**: Action cards resolve immediately without countering.

---

## Screenshots

| Main Menu | Gameplay (Your Turn) |
|:---:|:---:|
| ![Main Menu](docs/screenshots/menu.png) | ![Gameplay](docs/screenshots/gameplay.png) |

| Opponent's Turn & Thinking | Action Callout Banner |
|:---:|:---:|
| ![Opponent Turn](docs/screenshots/opponent-turn.png) | ![Action Callout](docs/screenshots/action-callout.png) |

| Hand Layout: Fan vs. Spread | Victory & Confetti Burst |
|:---:|:---:|
| ![Hand Fan](docs/screenshots/hand-held.png) | ![Victory](docs/screenshots/win-confetti.png) |

---

## Features

- ⚡ **Real-Time Multiplayer**: Low-latency WebSocket room management with automated reconnection and state recovery.
- 🤖 **Tiered AI Bot Ladder**: Progressive difficulties ranging from beginner bots up to **Tee-Noble**, a formidable boss bot powered by Information Set Monte Carlo Tree Search (ISMCTS) — very much beatable, but engineered to play as close to optimally as possible against card luck.
- 🔑 **Modern Passwordless Auth**: Native **WebAuthn (Passkeys)** support alongside traditional email/password and instant frictionless Guest Play.
- 🎵 **Procedural Audio Engine**: Web Audio API sound synthesis for card draws, swooshes, slam animations, and victory fanfares (with full mute toggles).
- 📱 **Adaptive UI**: Switch between compact overlapping hand fan and horizontal spread mode with smooth animations.
- 📬 **Invite System**: Shareable one-time friend invitation tokens.

---

## Architecture & Tech Stack

```
whots/
├── app/                  # SvelteKit 2 frontend (Svelte 5, TypeScript, Vite)
├── server/               # Rust backend (Axum 0.7, Tokio, SQLx, Redis)
│   ├── src/game/         # Game state machine, rules engine, and ISMCTS AI
│   ├── src/routes/       # REST routes & WebSocket room handlers
│   └── src/bin/          # Tuner, ladder tournament, and benchmark CLIs
├── deploy/               # Deployment configurations (Redis, Dokploy/Traefik)
└── docs/                 # Screenshots and detailed design documentation
```

- **Frontend**: SvelteKit 2, Svelte 5 runes, TypeScript, Tailwind CSS, Vite.
- **Backend**: Rust, Axum, Tokio, SQLx (PostgreSQL), Redis (Pub/Sub + state caching), `webauthn-rs`.
- **Database**: PostgreSQL 16 with SQLx migration tracking.
- **Cache**: Redis 7.

---

## Quickstart (Local Development)

### Option A: Docker Compose (Fastest)

Run the complete stack with a single command:

```bash
docker compose up --build
```

- **Web App**: [http://localhost:3000](http://localhost:3000)
- **API Server & Health**: [http://localhost:3001/health](http://localhost:3001/health)
- **PostgreSQL**: `localhost:5432`
- **Redis**: `localhost:6379`

### Option B: Running From Source

#### Prerequisites
- [Rust](https://rustup.rs/) (1.75+)
- [Node.js](https://nodejs.org/) (20+) and `npm`
- [Docker](https://www.docker.com/) (to run Postgres & Redis)

#### 1. Start Database & Redis Services
```bash
docker compose up postgres redis -d
```

#### 2. Start the Backend Server
```bash
cd server
cp .env.example .env
cargo run
```
The server applies SQLx database migrations automatically on startup and listens on `http://localhost:3001`.

#### 3. Start the Frontend Client
```bash
cd ../app
npm install
npm run dev
```
Open [http://localhost:5173](http://localhost:5173) in your browser.

---

## Environment Variables

### Backend (`server/.env`)
| Variable | Default | Description |
|---|---|---|
| `DATABASE_URL` | `postgres://...` | PostgreSQL connection string |
| `REDIS_URL` | `redis://127.0.0.1/` | Redis connection string |
| `JWT_SECRET` | *(required)* | Secret for signing auth tokens (min 32 chars) |
| `PORT` | `3001` | Server HTTP port |
| `FRONTEND_URL` | `http://localhost:5173` | Allowed CORS origin |
| `APP_URL` | `http://localhost:5173` | Public base URL (for WebAuthn RP verification) |
| `SMTP_HOST` | *(optional)* | SMTP host for emails (in dev, tokens print to console) |

### Frontend (`app/`)
| Variable | Default | Description |
|---|---|---|
| `BACKEND_URL` | `http://localhost:3001` | Dev proxy target for `/api` requests |
| `INTERNAL_API_URL` | `http://localhost:3001` | Server-side SSR backend endpoint |

---

## AI Difficulty Ladder & Tuner

The game includes an AI engine with 6 progressive ladder tiers plus the Tee-Noble boss challenge:

1. **Pikin**: Pure random play — learning card matching.
2. **Smallz**: Hand-thinning heuristic (sheds dominant suits).
3. **iSabiSmall**: Adds action card awareness (strategic Pick-Two / Suspension).
4. **Chief**: Adds threat detection (blocks players with few cards).
5. **Ẹgbọn Àdúgbò**: Adds card counting, Whot 20 conservation, and setup plays.
6. **Jagaban**: Advanced anticipation and multi-turn planning.
7. **Tee-Noble**: Boss encounter powered by Information Set Monte Carlo Tree Search (ISMCTS) — beatable, but the best play possible against card luck.

To optimize difficulty parameters using coordinate descent, run the built-in Rust tuner:

```bash
cargo run --release --bin tuner --manifest-path server/Cargo.toml
```

For full details on the continuous tuning pipeline, see [TUNING.md](TUNING.md).

---

## Testing & Quality

Run backend tests and linters:
```bash
cd server
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

Run frontend type-checking and linters:
```bash
cd app
npm run check
npm run lint
npm run build
```

---

## Production Deployment

A production deployment configuration using Dokploy / Traefik reverse proxy is provided in [deploy/docker-compose.prod.yml](deploy/docker-compose.prod.yml).

Copy `.env.production.example` to `.env` on your production server:
```bash
cp .env.production.example .env
docker compose -f deploy/docker-compose.prod.yml up -d --build
```

---

## Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) for details on code formatting, running tests, and opening pull requests.

Please review our [Code of Conduct](CODE_OF_CONDUCT.md) before participating.

---

## License

This project is licensed under the **MIT License** - see the [LICENSE](LICENSE) file for details.