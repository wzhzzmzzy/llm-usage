use llm_usage::core::adapter::claude::ClaudeAdapter;
use llm_usage::core::adapter::codex::CodexAdapter;
use llm_usage::core::adapter::gemini::GeminiAdapter;
use llm_usage::core::adapter::opencode::OpenCodeAdapter;
use llm_usage::core::adapter::*;
use llm_usage::core::model::*;
use llm_usage::core::normalize::*;
use serde::Deserialize;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{
    CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem,
    SystemTraySubmenu,
};

#[derive(Serialize, Deserialize, Default, Clone)]
struct AppConfig {
    #[serde(default)]
    language: String,
}

fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("llm-usage").join("config.json"))
}

fn load_config() -> AppConfig {
    config_path()
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str::<AppConfig>(&s).ok())
        .unwrap_or_default()
}

fn save_config(config: &AppConfig) {
    if let Some(path) = config_path() {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(config) {
            let _ = fs::write(path, json);
        }
    }
}

struct AppState {
    entries: Mutex<Option<CachedEntries>>,
    today_label: Mutex<String>,
}

const TODAY_ITEM_ID: &str = "today-total";

// Deliberately different from the web dashboard's `formatTokens`: scale from
// 1K upward and always keep exactly 2 decimals once scaled.
fn format_tokens_compact(n: u64) -> String {
    const K: u64 = 1_000;
    const M: u64 = 1_000_000;
    const B: u64 = 1_000_000_000;
    if n >= B {
        format!("{:.2}B", n as f64 / B as f64)
    } else if n >= M {
        format!("{:.2}M", n as f64 / M as f64)
    } else if n >= K {
        format!("{:.2}K", n as f64 / K as f64)
    } else {
        n.to_string()
    }
}

// Uses the dashboard's notion of "today": entry timestamps carry an ISO date
// prefix, which the web app compares against the local date.
fn today_total_tokens(entries: &[UsageEntry]) -> u64 {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    entries
        .iter()
        .filter(|e| e.timestamp.starts_with(&today))
        .map(|e| e.total_tokens)
        .sum()
}

fn today_item_title(label: &str) -> String {
    format!("Today: {}", label)
}

struct CachedEntries {
    timestamp: chrono::DateTime<chrono::Utc>,
    all_entries: Vec<UsageEntry>,
    by_source: Vec<(Source, Vec<UsageEntry>)>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    status: String,
    last_refresh: Option<String>,
    last_success: Option<String>,
    last_error: Option<String>,
    cells: serde_json::Value,
    daily: serde_json::Value,
    monthly: serde_json::Value,
    session: serde_json::Value,
    blocks: serde_json::Value,
    timezone: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RefreshStatus {
    is_refreshing: bool,
    last_refresh: Option<String>,
    last_success: Option<String>,
    last_error: Option<String>,
    snapshot_status: String,
}

fn load_all_entries() -> Vec<(Source, Vec<UsageEntry>)> {
    use rayon::prelude::*;

    let adapters: Vec<Box<dyn UsageAdapter>> = vec![
        Box::new(ClaudeAdapter::new()),
        Box::new(CodexAdapter::new()),
        Box::new(GeminiAdapter::new()),
        Box::new(OpenCodeAdapter::new()),
    ];

    adapters
        .into_par_iter()
        .filter_map(|adapter| {
            let source = adapter.source();
            let paths = adapter.find_data_paths().ok()?;
            let entries = adapter.load_entries(&paths).ok()?;
            if entries.is_empty() {
                None
            } else {
                Some((source, entries))
            }
        })
        .collect()
}

fn build_snapshot(cached: &CachedEntries) -> Snapshot {
    let mut cells = serde_json::Map::new();
    let mut daily = serde_json::Map::new();
    let mut monthly = serde_json::Map::new();
    let mut session = serde_json::Map::new();
    let mut blocks = serde_json::Map::new();

    let adapters: Vec<Box<dyn UsageAdapter>> = vec![
        Box::new(ClaudeAdapter::new()),
        Box::new(CodexAdapter::new()),
        Box::new(GeminiAdapter::new()),
        Box::new(OpenCodeAdapter::new()),
    ];

    for (source, entries) in &cached.by_source {
        let adapter = adapters.iter().find(|a| a.source() == *source).unwrap();
        let source_str = source.as_str();

        let daily_agg = adapter.aggregate_daily(entries);
        let monthly_agg = adapter.aggregate_monthly(entries);
        let session_agg = adapter.aggregate_session(entries);
        let blocks_agg = adapter.aggregate_blocks(entries, 5);

        if let Ok(report) = Normalizer::normalize_daily(&daily_agg) {
            daily.insert(
                format!("{}_daily", source_str),
                serde_json::to_value(report).unwrap(),
            );
        }
        if let Ok(report) = Normalizer::normalize_monthly(&monthly_agg) {
            monthly.insert(
                format!("{}_monthly", source_str),
                serde_json::to_value(report).unwrap(),
            );
        }
        if let Ok(report) = Normalizer::normalize_session(&session_agg) {
            session.insert(
                format!("{}_session", source_str),
                serde_json::to_value(report).unwrap(),
            );
        }
        if let Ok(report) = Normalizer::normalize_blocks(&blocks_agg) {
            blocks.insert(
                format!("{}_blocks", source_str),
                serde_json::to_value(report).unwrap(),
            );
        }

        for report in ReportType::all_variants() {
            let key = format!("{}_{}", source_str, report.as_str());
            cells.insert(
                key,
                serde_json::json!({
                    "cell": { "source": source_str, "report": report.as_str() },
                    "status": "success",
                    "durationMs": 0
                }),
            );
        }
    }

    let all_adapter = adapters.into_iter().next().unwrap();
    let all_daily = all_adapter.aggregate_daily(&cached.all_entries);
    let all_monthly = all_adapter.aggregate_monthly(&cached.all_entries);
    let all_session = all_adapter.aggregate_session(&cached.all_entries);
    let all_blocks = all_adapter.aggregate_blocks(&cached.all_entries, 5);

    if let Ok(report) = Normalizer::normalize_daily(&all_daily) {
        daily.insert("all_daily".into(), serde_json::to_value(report).unwrap());
    }
    if let Ok(report) = Normalizer::normalize_monthly(&all_monthly) {
        monthly.insert("all_monthly".into(), serde_json::to_value(report).unwrap());
    }
    if let Ok(report) = Normalizer::normalize_session(&all_session) {
        session.insert("all_session".into(), serde_json::to_value(report).unwrap());
    }
    if let Ok(report) = Normalizer::normalize_blocks(&all_blocks) {
        blocks.insert("all_blocks".into(), serde_json::to_value(report).unwrap());
    }

    for report in ReportType::all_variants() {
        cells.insert(
            format!("all_{}", report.as_str()),
            serde_json::json!({
                "cell": { "source": "all", "report": report.as_str() },
                "status": "success",
                "durationMs": 0
            }),
        );
    }

    Snapshot {
        status: "success".into(),
        last_refresh: Some(cached.timestamp.to_rfc3339()),
        last_success: Some(cached.timestamp.to_rfc3339()),
        last_error: None,
        cells: serde_json::Value::Object(cells),
        daily: serde_json::Value::Object(daily),
        monthly: serde_json::Value::Object(monthly),
        session: serde_json::Value::Object(session),
        blocks: serde_json::Value::Object(blocks),
        timezone: "UTC".into(),
    }
}

#[tauri::command]
fn get_snapshot(state: tauri::State<'_, AppState>) -> Snapshot {
    let cached = state.entries.lock().unwrap();
    match &*cached {
        Some(cached) => build_snapshot(cached),
        None => Snapshot {
            status: "nodata".into(),
            last_refresh: None,
            last_success: None,
            last_error: None,
            cells: serde_json::json!({}),
            daily: serde_json::json!({}),
            monthly: serde_json::json!({}),
            session: serde_json::json!({}),
            blocks: serde_json::json!({}),
            timezone: "UTC".into(),
        },
    }
}

async fn perform_refresh(app: &tauri::AppHandle) -> Result<RefreshStatus, String> {
    let entries = tauri::async_runtime::spawn_blocking(load_all_entries)
        .await
        .map_err(|e| e.to_string())?;

    let timestamp = chrono::Utc::now();
    let all_entries: Vec<UsageEntry> = entries.iter().flat_map(|(_, e)| e.clone()).collect();

    let today_label = format_tokens_compact(today_total_tokens(&all_entries));

    let cached = CachedEntries {
        timestamp,
        all_entries,
        by_source: entries,
    };

    let snapshot = build_snapshot(&cached);
    let status = snapshot.status.clone();

    let state = app.state::<AppState>();
    *state.today_label.lock().unwrap() = today_label.clone();
    if let Err(e) = app
        .tray_handle()
        .get_item(TODAY_ITEM_ID)
        .set_title(&today_item_title(&today_label))
    {
        eprintln!("[tray] Failed to update today tokens: {}", e);
    }

    *state.entries.lock().unwrap() = Some(cached);

    Ok(RefreshStatus {
        is_refreshing: false,
        last_refresh: Some(timestamp.to_rfc3339()),
        last_success: Some(timestamp.to_rfc3339()),
        last_error: None,
        snapshot_status: status,
    })
}

#[tauri::command]
async fn refresh(app: tauri::AppHandle) -> Result<RefreshStatus, String> {
    perform_refresh(&app).await
}

#[tauri::command]
fn refresh_status(state: tauri::State<'_, AppState>) -> RefreshStatus {
    let cached = state.entries.lock().unwrap();
    match &*cached {
        Some(cached) => RefreshStatus {
            is_refreshing: false,
            last_refresh: Some(cached.timestamp.to_rfc3339()),
            last_success: Some(cached.timestamp.to_rfc3339()),
            last_error: None,
            snapshot_status: "success".into(),
        },
        None => RefreshStatus {
            is_refreshing: false,
            last_refresh: None,
            last_success: None,
            last_error: None,
            snapshot_status: "nodata".into(),
        },
    }
}

#[tauri::command]
fn get_language() -> String {
    load_config().language
}

const LANG_OPTIONS: &[(&str, &str)] = &[
    ("zh-CN", "简体中文"),
    ("zh-TW", "繁體中文"),
    ("ja", "日本語"),
    ("en", "English"),
];

fn build_tray_menu(current_lang: &str, today_label: &str) -> SystemTrayMenu {
    let mut lang_menu = SystemTrayMenu::new();
    for (id, label) in LANG_OPTIONS {
        // 当前语言前加 ✓，其他前加空格对齐
        let display = if *id == current_lang {
            format!("✓ {}", label)
        } else {
            format!("  {}", label)
        };
        lang_menu = lang_menu.add_item(CustomMenuItem::new(id.to_string(), display));
    }

    SystemTrayMenu::new()
        .add_item(CustomMenuItem::new(TODAY_ITEM_ID, today_item_title(today_label)).disabled())
        .add_item(CustomMenuItem::new("refresh", "刷新 / Refresh"))
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_submenu(SystemTraySubmenu::new("语言 / Language", lang_menu))
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(CustomMenuItem::new("quit", "退出 / Quit"))
}

#[tauri::command]
fn health() -> serde_json::Value {
    serde_json::json!({
        "status": "healthy",
        "runner": { "found": true, "path": "native", "version": env!("CARGO_PKG_VERSION") },
        "ccusage": { "available": true, "version": "native" },
        "configPath": ""
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config = load_config();
    let initial_lang = config.language.clone();
    let tray_menu = build_tray_menu(&config.language, "-");

    tauri::Builder::default()
        .system_tray(SystemTray::new().with_menu(tray_menu))
        .setup(move |app| {
            // 初始语言注入：在所有页面 JS 执行前设置 window.__INITIAL_LANG__
            // 如果没有保存的语言偏好，不注入（前端 detectLang() 接管）
            let valid_langs = ["zh-CN", "zh-TW", "ja", "en"];
            let init_script = if valid_langs.contains(&initial_lang.as_str()) {
                format!("window.__INITIAL_LANG__ = '{}';", initial_lang)
            } else {
                String::new()
            };

            tauri::WindowBuilder::new(app, "main", tauri::WindowUrl::App("index.html".into()))
                .title("Tokender")
                .inner_size(1055.0, 800.0)
                .max_inner_size(1055.0, f64::MAX)
                .resizable(true)
                .initialization_script(&init_script)
                .build()?;

            Ok(())
        })
        .on_system_tray_event(|app, event| {
            if let SystemTrayEvent::MenuItemClick { id, .. } = event {
                match id.as_str() {
                    lang @ ("zh-CN" | "zh-TW" | "ja" | "en") => {
                        // 1. 持久化到配置文件
                        let mut cfg = load_config();
                        cfg.language = lang.to_string();
                        save_config(&cfg);

                        // 2. 更新托盘菜单勾选项
                        let today_label = app
                            .state::<AppState>()
                            .today_label
                            .lock()
                            .unwrap()
                            .clone();
                        let new_menu = build_tray_menu(lang, &today_label);
                        if let Err(e) = app.tray_handle().set_menu(new_menu) {
                            eprintln!("[i18n] Failed to update tray menu: {}", e);
                        }

                        // 3. 通知前端实时切换语言
                        if let Err(e) = app.emit_all("language-changed", lang) {
                            eprintln!("[i18n] Failed to emit language-changed: {}", e);
                        }
                    }
                    "refresh" => {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            match perform_refresh(&app).await {
                                Ok(_) => {
                                    if let Err(e) = app.emit_all("snapshot-updated", ()) {
                                        eprintln!("[tray] Failed to emit snapshot-updated: {}", e);
                                    }
                                }
                                Err(e) => eprintln!("[tray] Refresh failed: {}", e),
                            }
                        });
                    }
                    "quit" => std::process::exit(0),
                    _ => {}
                }
            }
        })
        .manage(AppState {
            entries: Mutex::new(None),
            today_label: Mutex::new("-".into()),
        })
        .invoke_handler(tauri::generate_handler![
            health,
            refresh,
            refresh_status,
            get_snapshot,
            get_language
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_appconfig_default_language_is_empty() {
        let config = AppConfig::default();
        assert_eq!(config.language, "");
    }

    #[test]
    fn test_appconfig_serialization_roundtrip() {
        let config = AppConfig {
            language: "zh-CN".to_string(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.language, "zh-CN");
    }

    #[test]
    fn test_appconfig_deserialize_empty_json() {
        // config.json 不含 language 字段时应使用默认空字符串
        let config: AppConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(config.language, "");
    }

    #[test]
    fn test_appconfig_deserialize_unknown_fields_ignored() {
        // 未来加入新字段时旧版本能容错
        let config: AppConfig =
            serde_json::from_str(r#"{"language":"ja","theme":"dark"}"#).unwrap();
        assert_eq!(config.language, "ja");
    }

    #[test]
    fn test_format_tokens_compact() {
        assert_eq!(format_tokens_compact(0), "0");
        assert_eq!(format_tokens_compact(999), "999");
        assert_eq!(format_tokens_compact(1_000), "1.00K");
        assert_eq!(format_tokens_compact(1_500), "1.50K");
        assert_eq!(format_tokens_compact(999_999), "1000.00K");
        assert_eq!(format_tokens_compact(1_000_000), "1.00M");
        assert_eq!(format_tokens_compact(2_345_678), "2.35M");
        assert_eq!(format_tokens_compact(1_000_000_000), "1.00B");
        assert_eq!(format_tokens_compact(3_456_789_012), "3.46B");
    }

    fn usage_entry_at(timestamp: &str, total_tokens: u64) -> UsageEntry {
        UsageEntry {
            session_id: String::new(),
            timestamp: timestamp.to_string(),
            model: None,
            input_tokens: 0,
            output_tokens: 0,
            reasoning_tokens: 0,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
            total_tokens,
            project_path: None,
        }
    }

    #[test]
    fn test_today_total_tokens_sums_today_only() {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let entries = vec![
            usage_entry_at(&format!("{}T01:00:00Z", today), 100),
            usage_entry_at(&format!("{}T23:59:59Z", today), 200),
            usage_entry_at("2000-01-01T00:00:00Z", 500),
            usage_entry_at("", 700),
        ];
        assert_eq!(today_total_tokens(&entries), 300);
    }

    #[test]
    fn test_today_total_tokens_empty() {
        assert_eq!(today_total_tokens(&[]), 0);
    }
}
