# Whots Web Client 🃏

The frontend web client for Whots, built with [SvelteKit 2](https://kit.svelte.dev/) and [Svelte 5](https://svelte.dev/).

---

## Development Setup

### Install Dependencies
```bash
npm install
```

### Run Local Development Server
```bash
npm run dev
```
Open [http://localhost:5173](http://localhost:5173) in your browser.

By default, the Vite dev server proxies `/api` and WebSocket connections to `http://localhost:3001`.
To target a different backend server:
```bash
BACKEND_URL=http://localhost:3001 npm run dev
```

---

## Scripts

| Command | Action |
|---|---|
| `npm run dev` | Starts Vite development server with hot-module reload |
| `npm run build` | Builds the production bundle using `@sveltejs/adapter-node` |
| `npm run preview` | Previews the production build locally |
| `npm run check` | Runs SvelteKit sync and `svelte-check` for TypeScript validation |
| `npm run lint` | Runs Prettier format checks and ESLint |
| `npm run format` | Automatically formats codebase with Prettier |

---

## Architecture Overview

- `src/routes/`: SvelteKit file-based routing (game board, lobby, authentication, profile, and invite redemption).
- `src/lib/ui/`: Modular Svelte components (game board, hand layout toggles, sound controls, action callout banners, confetti animations).
- `src/lib/stores/`: Client-side state stores for game sessions, notifications, and user profiles.
- `src/lib/api/`: Typed REST client and WebSocket connection manager.
- `src/lib/server/`: Server-side API proxying and secure HTTP-only refresh cookie handling.
