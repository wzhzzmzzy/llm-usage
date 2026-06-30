use llm_usage::core::adapter::*;
use llm_usage::core::adapter::claude::ClaudeAdapter;
use llm_usage::core::adapter::codex::CodexAdapter;
use llm_usage::core::adapter::gemini::GeminiAdapter;
use llm_usage::core::adapter::opencode::OpenCodeAdapter;
use llm_usage::core::model::*;
use llm_usage::core::normalize::*;
use serde::Serialize;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{
    CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu,
    SystemTrayMenuItem,
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
    let adapters: Vec<Box<dyn UsageAdapter>> = vec![
        Box::new(ClaudeAdapter::new()),
        Box::new(CodexAdapter::new()),
        Box::new(GeminiAdapter::new()),
        Box::new(OpenCodeAdapter::new()),
    ];

    adapters
        .into_iter()
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

        let daily_raw: Vec<RawDailyAggregate> = daily_agg.iter().map(|a| RawDailyAggregate {
            date: a.date.clone(),
            total_tokens: a.total_tokens,
            input_tokens: a.input_tokens,
            cache_read_tokens: a.cache_read_tokens,
            output_tokens: a.output_tokens,
            request_count: Some(a.request_count),
            models_used: Some(a.models_used.clone()),
            model_breakdown: Some(a.model_breakdown.clone()),
        }).collect();

        let monthly_raw: Vec<RawMonthlyAggregate> = monthly_agg.iter().map(|a| RawMonthlyAggregate {
            month: a.month.clone(),
            total_tokens: a.total_tokens,
            input_tokens: a.input_tokens,
            cache_read_tokens: a.cache_read_tokens,
            output_tokens: a.output_tokens,
            request_count: Some(a.request_count),
            models_used: Some(a.models_used.clone()),
            model_breakdown: Some(a.model_breakdown.clone()),
        }).collect();

        let session_raw: Vec<RawSessionAggregate> = session_agg.iter().map(|a| RawSessionAggregate {
            session_id: a.session_id.clone(),
            project_path: a.project_path.clone(),
            total_tokens: a.total_tokens,
            input_tokens: a.input_tokens,
            cache_read_tokens: a.cache_read_tokens,
            output_tokens: a.output_tokens,
            request_count: Some(a.request_count),
            last_activity: a.last_activity.clone(),
            models_used: Some(a.models_used.clone()),
            model_breakdown: Some(a.model_breakdown.clone()),
        }).collect();

        let blocks_raw: Vec<RawBlockAggregate> = blocks_agg.iter().map(|a| RawBlockAggregate {
            block_id: a.block_id.clone(),
            start_time: a.start_time.clone(),
            end_time: a.end_time.clone(),
            actual_end_time: a.actual_end_time.clone(),
            is_active: a.is_active,
            total_tokens: a.total_tokens,
            input_tokens: a.input_tokens,
            cache_read_tokens: a.cache_read_tokens,
            output_tokens: a.output_tokens,
            request_count: Some(a.request_count),
            models_used: Some(a.models_used.clone()),
            model_breakdown: Some(a.model_breakdown.clone()),
        }).collect();

        if let Ok(report) = Normalizer::normalize_daily(&daily_raw) {
            daily.insert(format!("{}_daily", source_str), serde_json::to_value(report).unwrap());
        }
        if let Ok(report) = Normalizer::normalize_monthly(&monthly_raw) {
            monthly.insert(format!("{}_monthly", source_str), serde_json::to_value(report).unwrap());
        }
        if let Ok(report) = Normalizer::normalize_session(&session_raw) {
            session.insert(format!("{}_session", source_str), serde_json::to_value(report).unwrap());
        }
        if let Ok(report) = Normalizer::normalize_blocks(&blocks_raw) {
            blocks.insert(format!("{}_blocks", source_str), serde_json::to_value(report).unwrap());
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

    let all_daily_raw: Vec<RawDailyAggregate> = all_daily.iter().map(|a| RawDailyAggregate {
        date: a.date.clone(),
        total_tokens: a.total_tokens,
        input_tokens: a.input_tokens,
        cache_read_tokens: a.cache_read_tokens,
        output_tokens: a.output_tokens,
        request_count: Some(a.request_count),
        models_used: Some(a.models_used.clone()),
        model_breakdown: Some(a.model_breakdown.clone()),
    }).collect();

    let all_monthly_raw: Vec<RawMonthlyAggregate> = all_monthly.iter().map(|a| RawMonthlyAggregate {
        month: a.month.clone(),
        total_tokens: a.total_tokens,
        input_tokens: a.input_tokens,
        cache_read_tokens: a.cache_read_tokens,
        output_tokens: a.output_tokens,
        request_count: Some(a.request_count),
        models_used: Some(a.models_used.clone()),
        model_breakdown: Some(a.model_breakdown.clone()),
    }).collect();

    let all_session_raw: Vec<RawSessionAggregate> = all_session.iter().map(|a| RawSessionAggregate {
        session_id: a.session_id.clone(),
        project_path: a.project_path.clone(),
        total_tokens: a.total_tokens,
        input_tokens: a.input_tokens,
        cache_read_tokens: a.cache_read_tokens,
        output_tokens: a.output_tokens,
        request_count: Some(a.request_count),
        last_activity: a.last_activity.clone(),
        models_used: Some(a.models_used.clone()),
        model_breakdown: Some(a.model_breakdown.clone()),
    }).collect();

    let all_blocks_raw: Vec<RawBlockAggregate> = all_blocks.iter().map(|a| RawBlockAggregate {
        block_id: a.block_id.clone(),
        start_time: a.start_time.clone(),
        end_time: a.end_time.clone(),
        actual_end_time: a.actual_end_time.clone(),
        is_active: a.is_active,
        total_tokens: a.total_tokens,
        input_tokens: a.input_tokens,
        cache_read_tokens: a.cache_read_tokens,
        output_tokens: a.output_tokens,
        request_count: Some(a.request_count),
        models_used: Some(a.models_used.clone()),
        model_breakdown: Some(a.model_breakdown.clone()),
    }).collect();

    if let Ok(report) = Normalizer::normalize_daily(&all_daily_raw) {
        daily.insert("all_daily".into(), serde_json::to_value(report).unwrap());
    }
    if let Ok(report) = Normalizer::normalize_monthly(&all_monthly_raw) {
        monthly.insert("all_monthly".into(), serde_json::to_value(report).unwrap());
    }
    if let Ok(report) = Normalizer::normalize_session(&all_session_raw) {
        session.insert("all_session".into(), serde_json::to_value(report).unwrap());
    }
    if let Ok(report) = Normalizer::normalize_blocks(&all_blocks_raw) {
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

#[tauri::command]
async fn refresh(state: tauri::State<'_, AppState>) -> Result<RefreshStatus, String> {
    let entries = tauri::async_runtime::spawn_blocking(load_all_entries)
        .await
        .map_err(|e| e.to_string())?;

    let timestamp = chrono::Utc::now();
    let all_entries: Vec<UsageEntry> = entries.iter().flat_map(|(_, e)| e.clone()).collect();

    let cached = CachedEntries {
        timestamp,
        all_entries,
        by_source: entries,
    };

    let snapshot = build_snapshot(&cached);
    let status = snapshot.status.clone();

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

fn build_tray_menu(current_lang: &str) -> SystemTrayMenu {
    let mut menu = SystemTrayMenu::new();
    for (id, label) in LANG_OPTIONS {
        // 当前语言前加 ✓，其他前加空格对齐
        let display = if *id == current_lang {
            format!("✓ {}", label)
        } else {
            format!("  {}", label)
        };
        menu = menu.add_item(CustomMenuItem::new(id.to_string(), display));
    }
    menu = menu
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(CustomMenuItem::new("quit", "退出 / Quit"));
    menu
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
    let tray_menu = build_tray_menu(&config.language);

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

            tauri::WindowBuilder::new(
                app,
                "main",
                tauri::WindowUrl::App("index.html".into()),
            )
            .title("LLM Usage Dashboard")
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
                        let new_menu = build_tray_menu(lang);
                        if let Err(e) = app.tray_handle().set_menu(new_menu) {
                            eprintln!("[i18n] Failed to update tray menu: {}", e);
                        }

                        // 3. 通知前端实时切换语言
                        if let Err(e) = app.emit_all("language-changed", lang) {
                            eprintln!("[i18n] Failed to emit language-changed: {}", e);
                        }
                    }
                    "quit" => std::process::exit(0),
                    _ => {}
                }
            }
        })
        .manage(AppState {
            entries: Mutex::new(None),
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
        let config = AppConfig { language: "zh-CN".to_string() };
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
        let config: AppConfig = serde_json::from_str(r#"{"language":"ja","theme":"dark"}"#).unwrap();
        assert_eq!(config.language, "ja");
    }
}
