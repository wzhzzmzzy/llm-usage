# 历史数据缓存设计文档

**日期：** 2026-06-29  
**状态：** 已审批，待实现

---

## 问题

Claude Code 内置自动清理机制，会在某些条件下删除 `~/.claude/projects/<hash>/` 下的旧 JSONL 会话文件。本应用每次 Refresh 都从这些文件重新读取，文件一旦被删，对应日期的历史用量数据即永久丢失。

---

## 目标

在每次成功 Refresh 后，将处理好的日级聚合数据 (`DailyRow`) 持久化到本地 JSON 文件，下次 Refresh 时自动将缓存中的历史日期合并回 Snapshot，使前端无感知地展示完整历史数据。

---

## 方案选型

**缓存粒度：** 日级聚合（`DailyRow`）  
- 原始 `UsageEntry` 体积大、信息冗余；会话/区块数据随时间膨胀且无法从 JSONL 清理后重建  
- `DailyRow` 足够驱动前端所有视图（metric 卡、热力图、日报表、月报表可从日数据推导）

**合并策略：** JSONL 优先  
- 每次 Refresh 能读到的日期，以 JSONL 最新数据为权威写入缓存  
- 缓存中有、JSONL 没有的日期，补充回 Snapshot

**存储位置：** `~/.config/llm-usage/cache/daily_cache.json`  
（macOS 实际路径：`~/Library/Application Support/llm-usage/cache/daily_cache.json`）  
使用 `dirs::config_dir()` 定位，与现有 `config.json` 同根目录。

---

## 架构

### 数据流（Refresh 修改后）

```
1. provider.preload()
   从 JSONL 读取 UsageEntry（被清理的文件此处读不到）

2. execute_cells()
   聚合各 source → 写入 Snapshot.daily

3. cache.update_from_snapshot(&snapshot)   ← 新增
   把 JSONL 读到的所有 DailyRow 按 source 写入缓存（覆盖旧值）

4. cache.merge_into_snapshot(&mut snapshot)  ← 新增
   把缓存里有、但本次读不到的历史日期注入各 source 的 DailyReport
   注入后重新计算 DailyReport.totals
   重新聚合 "all_daily"（从各 source 合并）

5. cache.save()   ← 新增
   原子写入（先写 .tmp，再 rename）
   写失败只 warn log，不影响 Snapshot 数据
```

### 受影响文件

| 文件 | 变更类型 |
|------|---------|
| `src/core/cache.rs` | 新增：`DailyCache` struct |
| `src/core/mod.rs` | 修改：`pub mod cache` |
| `src/core/refresh.rs` | 修改：集成 cache，`RefreshManager` 加 `cache_dir` |
| `src-tauri/src/lib.rs` | 修改：构建 `RefreshManager` 时传入 cache 目录 |
| `src/Cargo.toml` | 修改：视情况补充 `serde_json` 依赖 |

---

## `DailyCache` 详细设计

### 缓存文件格式

```json
{
  "claude": {
    "2025-11-01": {
      "date": "2025-11-01",
      "totalTokens": 12345,
      "inputTokens": 8000,
      "cacheReadTokens": 2000,
      "outputTokens": 2345,
      "requestCount": 42,
      "modelsUsed": ["claude-opus-4-5"],
      "modelBreakdown": [
        { "model": "claude-opus-4-5", "totalTokens": 12345, "inputTokens": 8000, "outputTokens": 2345, "cacheReadTokens": 2000, "requestCount": 42 }
      ]
    }
  },
  "codex":    { "2025-11-01": { ... } },
  "gemini":   {},
  "opencode": {}
}
```

字段完全对应 `DailyRow`（`#[serde(rename_all = "camelCase")]`），可直接序列化/反序列化。

### Rust 结构

```rust
// src/core/cache.rs

use std::collections::HashMap;
use std::path::PathBuf;
use crate::core::model::{DailyRow, DailyReport, Snapshot, Source, ReportType, UsageMetric};

pub struct DailyCache {
    path: PathBuf,
    /// source_key → (date_str → DailyRow)
    /// source_key: "claude" | "codex" | "gemini" | "opencode"
    data: HashMap<String, HashMap<String, DailyRow>>,
}

impl DailyCache {
    /// 从文件加载。文件不存在返回空缓存；解析失败 warn log 后返回空缓存。
    pub fn load(path: PathBuf) -> Self

    /// 把 snapshot.daily 中各 source 的 DailyRow 写入缓存。
    /// 仅处理非 "all" 的 source（"all_daily" 由 merge 步重建）。
    /// JSONL 能读到的日期覆盖已有缓存条目。
    pub fn update_from_snapshot(&mut self, snapshot: &Snapshot)

    /// 把缓存里有、但 snapshot 里没有的历史日期注入 snapshot.daily。
    /// 注入完成后重新计算每个 DailyReport.totals。
    /// 最后重新聚合 "all_daily"：把所有非 "all" source 的 DailyRow 按日期合并求和。
    pub fn merge_into_snapshot(&self, snapshot: &mut Snapshot)

    /// 原子写入：serde_json::to_string_pretty → 写 path.with_extension("tmp") → rename。
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>>
}
```

### `merge_into_snapshot` 内部逻辑

```
any_injected = false

for source in [claude, codex, gemini, opencode]:
    cache_key = source.as_str()          // "claude"
    snapshot_key = "{source}_daily"      // "claude_daily"
    cached_days = cache.data.get(cache_key)  // 可能为空

    if cached_days 为空或为 None:
        continue

    if snapshot.daily 没有 snapshot_key:
        // JSONL 全部被清理，从缓存整体重建该 source 的报告
        snapshot.daily.insert(snapshot_key, DailyReport {
            days: cached_days.values().cloned().sorted_by_date(),
            totals: sum_all_days(cached_days.values()),
        })
        any_injected = true
        continue

    // snapshot 有该 source，只注入缺失日期
    existing_dates = snapshot.daily[snapshot_key].days 的所有 date

    for (date, cached_row) in cached_days:
        if date not in existing_dates:
            snapshot.daily[snapshot_key].days.push(cached_row)
            any_injected = true

    // 按日期升序排列
    snapshot.daily[snapshot_key].days.sort_by_key(|r| r.date)

    // 重新计算 totals（包含新注入的历史日期）
    snapshot.daily[snapshot_key].totals = sum_all_days(...)

// 有任何注入时重建 all_daily
if any_injected:
    rebuild_all_daily(snapshot)
```

### `rebuild_all_daily` 逻辑

```
all_map: HashMap<NaiveDate, DailyRow> = {}

for source in [claude, codex, gemini, opencode]:
    for row in snapshot.daily["{source}_daily"].days:
        if date in all_map:
            all_map[date].total_tokens += row.total_tokens
            // ... 其余 token 字段累加
            // models_used 取并集（去重）
            // model_breakdown 按 model 名合并累加
        else:
            all_map[date] = row.clone()

snapshot.daily["all_daily"].days = all_map.values().sorted_by_date()
snapshot.daily["all_daily"].totals = sum_all_days(all_daily.days)
```

> `rebuild_all_daily` 在有任何历史日期注入时执行。若注入后 `snapshot.daily` 中存在至少一个非 "all" source 报告，则重新计算并写入 `"all_daily"`，即使原来不存在该 key 也会创建。

---

## `RefreshManager` 修改

```rust
pub struct RefreshManager {
    provider: Arc<NativeUsageProvider>,
    snapshot: Arc<Mutex<Snapshot>>,
    is_refreshing: Arc<RwLock<bool>>,
    last_error: Arc<Mutex<Option<String>>>,
    cache_dir: Option<PathBuf>,   // None = 不启用缓存（web server 模式）
}

impl RefreshManager {
    // 原有 new()，cache_dir = None，向后兼容
    pub fn new(config: AppConfig) -> Self

    // Tauri desktop 专用，传入 cache 目录
    pub fn with_cache(config: AppConfig, cache_dir: PathBuf) -> Self
}
```

`execute_refresh_inner` 末尾（在 `snapshot_guard` 更新后、return 前）插入：

```rust
if let Some(dir) = &cache_dir {
    let cache_path = dir.join("daily_cache.json");
    let mut cache = DailyCache::load(cache_path);
    cache.update_from_snapshot(&snapshot_guard);
    cache.merge_into_snapshot(&mut snapshot_guard);
    if let Err(e) = cache.save() {
        tracing::warn!("Failed to save daily cache: {}", e);
    }
}
```

---

## 初始化（`src-tauri/src/lib.rs`）

```rust
// dirs::config_dir() 已在 lib.rs 中用于语言配置，此处复用
let cache_dir = dirs::config_dir()
    .map(|d| d.join("llm-usage").join("cache"));

let manager = match cache_dir {
    Some(dir) => RefreshManager::with_cache(config, dir),
    None => {
        tracing::warn!("Cannot determine cache dir, history cache disabled");
        RefreshManager::new(config)
    }
};
```

`dirs::config_dir()` 在正常桌面环境中不应返回 `None`；`None` 的情况只在特殊沙盒环境下才会出现，作为 warn log 处理即可。

---

## 错误处理

| 场景 | 处理方式 |
|------|---------|
| 缓存文件不存在 | 静默创建空缓存，不报错 |
| 缓存文件 JSON 解析失败 | `tracing::warn!`，使用空缓存继续 |
| 缓存写入失败（磁盘满等）| `tracing::warn!`，不影响 Snapshot 和前端展示 |
| cache_dir 创建失败 | `tracing::warn!`，不影响 Snapshot 和前端展示 |

所有缓存错误不影响正常刷新流程。缓存是增强层，不是关键路径。

---

## 不在此次范围内

- **缓存淘汰/清除**：数据只增不减，用户需手动删除 `daily_cache.json`
- **Web server 模式缓存**：`cache_dir = None`，不启用
- **Session / Block 缓存**：粒度太细，且 session ID 依赖原始文件，JSONL 删除后无法重建
- **月报表独立缓存**：月数据可从日数据推导，不重复存储

---

## 测试策略

- `DailyCache::load` — 文件不存在、空文件、损坏 JSON 各一个 case
- `update_from_snapshot` + `merge_into_snapshot` — 合并后日期集合 = 缓存日期 ∪ 快照日期
- `save` + `load` 往返验证 — 序列化/反序列化无数据丢失
- `rebuild_all_daily` — token 字段累加正确，models_used 去重正确
- totals 重算 — 注入历史日期后 totals 等于所有 days 之和
