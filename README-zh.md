# LLM 用量看板

[English](./README.md)

在一个本地看板中追踪 Claude、Codex、Gemini 和 OpenCode 的 token 用量。

数据保留在本地，无需云同步，无需账号。

## 为什么做这个

多个 AI 编程助手，多个用量追踪工具，没有统一视图。这个工具将 ccusage (Claude)、codex CLI、gemini CLI 和 opencode 的 token 消耗汇总到一个看板中，支持按日/月/会话维度查看。

## 运行模式

看板有两种运行方式，均从源码构建：

- **Tauri 桌面应用** —— 原生 macOS 应用，带系统托盘图标。日常使用推荐。
- **本地 server** —— 同一个看板以 HTTP 服务形式运行。适合无头环境、SSH 会话或 cron 定时刷新。

目前暂未发布签名预编译包（原因见 Tauri 一节），所以从源码构建是标准安装方式。

### 前置依赖

- Rust 1.75+
- Node.js 20+ 和 [pnpm](https://pnpm.io/)
- Tauri CLI（仅桌面应用需要）：`cargo install tauri-cli --version "^1"`
- macOS：Xcode Command Line Tools

### 模式一：Tauri 桌面应用（推荐）

#### 构建

```bash
git clone <repo-url>
cd llm-usage
pnpm --prefix frontend install
cargo tauri build
```

`cargo tauri build` 会自动构建前端（`pnpm --prefix frontend build`），产出：

- `target/release/bundle/macos/LLM Usage Dashboard.app` —— 应用本体
- `target/release/bundle/dmg/LLM Usage Dashboard_<版本号>_aarch64.dmg` —— 磁盘镜像，可留存或拷贝到自己的其他机器

#### 运行

打开 `LLM Usage Dashboard.app`。应用驻留在系统托盘，启动时加载本地用量数据并在后台刷新。

**个人使用不需要 Apple 开发者账号**。应用未签名，首次打开时 macOS 可能拦截：右键点击 `.app` → 打开 → 确认即可。未签名构建在你自己的机器上完全可用，只是无法分发给任意用户（对方的 Gatekeeper 会告警），这也是暂不发布公共 `.dmg` 下载的原因。

### 模式二：本地 server

#### 构建

```bash
git clone <repo-url>
cd llm-usage
pnpm --prefix frontend install
pnpm --prefix frontend build   # server 会内嵌 frontend/dist
cargo build --release
```

#### 运行

```bash
# Web 看板，访问 http://127.0.0.1:3766
./target/release/llm-usage serve

# 单次刷新（适合 cron 定时任务）
./target/release/llm-usage refresh

# 检查 ccusage 和 runner 是否可用
./target/release/llm-usage health
```

监听地址和端口可配置，见下文。

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
├── src-tauri/          # Tauri 桌面外壳（托盘应用，共用同一套核心逻辑）
└── frontend/           # React + Recharts 看板
```

每个数据源适配器负责读取对应工具的本地数据（如 Claude JSONL、opencode SQLite）。刷新管理器运行所有适配器，标准化输出，通过 HTTP API 提供服务。前端轮询此 API。

## 开发

```bash
# Tauri 应用（前后端同时启动）
cargo tauri dev

# 仅前端（Vite 开发服务器）
cd frontend && pnpm dev

# 仅后端（自动重载）
cargo watch -x run

# 测试
cd frontend && pnpm test
cargo test
```

## 许可证

MIT
