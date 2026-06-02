# LLM 用量看板

[English](./README.md)

在一个本地看板中追踪 Claude、Codex、Gemini 和 OpenCode 的 token 用量。

数据保留在本地，无需云同步，无需账号。

## 为什么做这个

多个 AI 编程助手，多个用量追踪工具，没有统一视图。这个工具将 ccusage (Claude)、codex CLI、gemini CLI 和 opencode 的 token 消耗汇总到一个看板中，支持按日/月/会话维度查看。

## 安装

前置要求：Rust 1.75+、Node.js 20+

```bash
git clone <repo-url>
cd llm-usage
cd frontend && npm install && npm run build && cd ..
cargo build --release
```

## 运行

```bash
# Web 看板，访问 http://127.0.0.1:3766
./target/release/llm-usage serve

# 单次刷新（适合 cron 定时任务）
./target/release/llm-usage refresh

# 检查 ccusage 和 runner 是否可用
./target/release/llm-usage health
```

## 配置

配置文件位置：`~/.config/llm-usage/config.toml`

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

## 桌面应用

可选的 Tauri 封装，提供原生窗口体验：

```bash
cargo install tauri-cli
cargo tauri dev    # 开发模式
cargo tauri build  # 生产构建
```

## 工作原理

```
llm-usage
├── src/
│   ├── core/
│   │   ├── adapter/    # 各数据源适配器（claude, codex, gemini, opencode）
│   │   ├── pricing.rs  # token 成本计算
│   │   └── refresh.rs  # 协调数据采集，存储快照
│   ├── server/         # axum HTTP API + 静态文件服务
│   └── cli.rs          # 基于 clap 的命令行接口
└── frontend/           # React + Recharts 看板
```

每个数据源适配器负责读取对应工具的本地数据（如 Claude JSONL、opencode SQLite）。刷新管理器运行所有适配器，标准化输出，通过 HTTP API 提供服务。前端轮询此 API。

## 开发

```bash
# 前端（Vite 开发服务器）
cd frontend && npm run dev

# 后端（自动重载）
cargo watch -x run

# 测试
cd frontend && npm test
cargo test
```

## 许可证

MIT
