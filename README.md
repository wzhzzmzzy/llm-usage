# LLM Usage Dashboard

[中文文档](./README-zh.md)

Track token usage across Claude, Codex, Gemini, and OpenCode in a single local dashboard.

Data stays on your machine. No cloud sync, no accounts.

## Why

Multiple AI coding assistants, multiple usage trackers, no unified view. This tool aggregates token consumption from ccusage (Claude), codex CLI, gemini CLI, and opencode into one dashboard with daily/monthly/session breakdowns.

## Install

Requires: Rust 1.75+, Node.js 20+, [pnpm](https://pnpm.io/)

```bash
git clone <repo-url>
cd llm-usage
cd frontend && pnpm install && pnpm build && cd ..
cargo build --release
```

## Run

```bash
# Web dashboard at http://127.0.0.1:3766
./target/release/llm-usage serve

# One-off refresh (useful for cron)
./target/release/llm-usage refresh

# Check if ccusage and runner are available
./target/release/llm-usage health
```

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

## Desktop App

Optional Tauri wrapper for a native window:

```bash
cargo install tauri-cli
cargo tauri dev    # development
cargo tauri build  # production build
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
└── frontend/           # React + Recharts dashboard
```

Each source adapter knows how to read its tool's local data (e.g., Claude JSONL, opencode SQLite). The refresh manager runs all adapters, normalizes the output, and serves it via the HTTP API. The frontend polls this API.

## Dev

```bash
# Frontend (Vite dev server)
cd frontend && npm run dev

# Backend (auto-reload)
cargo watch -x run

# Tests
cd frontend && npm test
cargo test
```

## License

MIT
