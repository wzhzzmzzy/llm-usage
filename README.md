# LLM Usage Dashboard

[中文文档](./README-zh.md)

Track token usage across Claude, Codex, Gemini, and OpenCode in a single local dashboard.

Data stays on your machine. No cloud sync, no accounts.

## Why

Multiple AI coding assistants, multiple usage trackers, no unified view. This tool aggregates token consumption from ccusage (Claude), codex CLI, gemini CLI, and opencode into one dashboard with daily/monthly/session breakdowns.

## Run modes

There are two ways to run the dashboard, both built from source:

- **Tauri desktop app** — a native macOS app with a system tray icon. Recommended for daily use.
- **Local server** — the same dashboard served over HTTP. Recommended for headless setups, SSH sessions, or cron-driven refreshes.

No signed prebuilt binaries are published for now (see the note in the Tauri section), so building from source is the way to go.

### Prerequisites

- Rust 1.75+
- Node.js 20+ and [pnpm](https://pnpm.io/)
- Tauri CLI (desktop app only): `cargo install tauri-cli --version "^1"`
- macOS: Xcode Command Line Tools

### Mode 1: Tauri desktop app (recommended)

#### Build

```bash
git clone <repo-url>
cd llm-usage
pnpm --prefix frontend install
cargo tauri build
```

`cargo tauri build` builds the frontend automatically (`pnpm --prefix frontend build`) and produces:

- `target/release/bundle/macos/LLM Usage Dashboard.app` — the app itself
- `target/release/bundle/dmg/LLM Usage Dashboard_<version>_aarch64.dmg` — a disk image for keeping or copying to your other machines

#### Run

Open `LLM Usage Dashboard.app`. It sits in the system tray, loads your local usage data on launch, and refreshes in the background.

**No Apple Developer account is needed** for personal use. The app is unsigned, so on first launch macOS may block it: right-click the `.app` → Open → confirm. This unsigned build works fine on your own machines; it just can't be distributed to arbitrary users (Gatekeeper would warn them), which is why no public `.dmg` download is published.

### Mode 2: Local server

#### Build

```bash
git clone <repo-url>
cd llm-usage
pnpm --prefix frontend install
pnpm --prefix frontend build   # the server embeds frontend/dist
cargo build --release
```

#### Run

```bash
# Web dashboard at http://127.0.0.1:3766
./target/release/llm-usage serve

# One-off refresh (useful for cron)
./target/release/llm-usage refresh

# Check if ccusage and runner are available
./target/release/llm-usage health
```

Host and port are configurable, see below.

## Config

`~/.config/llm-usage/config.toml`:

```toml
[sources]
claude = true
codex = true
gemini = true
opencode = true

[server]
host = "127.0.0.1"
port = 3766
```

## How It Works

```
llm-usage
├── src/
│   ├── core/
│   │   ├── adapter/    # per-source data fetchers (claude, codex, gemini, opencode)
│   │   ├── pricing.rs  # token-to-cost calculation
│   │   └── refresh.rs  # orchestrates data collection, stores snapshots
│   ├── server/         # axum HTTP API + static file serving
│   └── cli.rs          # clap-based CLI
├── src-tauri/          # Tauri desktop shell (tray app, shares the same core)
└── frontend/           # React + Recharts dashboard
```

Each source adapter knows how to read its tool's local data (e.g., Claude JSONL, opencode SQLite). The refresh manager runs all adapters, normalizes the output, and serves it via the HTTP API. The frontend polls this API.

## Dev

```bash
# Tauri app (frontend + backend together)
cargo tauri dev

# Frontend only (Vite dev server)
cd frontend && pnpm dev

# Backend only (auto-reload)
cargo watch -x run

# Tests
cd frontend && pnpm test
cargo test
```

## License

MIT
