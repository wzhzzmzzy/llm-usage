use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use super::{aggregate_blocks_impl, build_model_breakdown, BlockAggregate, DailyAggregate, MonthlyAggregate, SessionAggregate, UsageAdapter, UsageEntry};
use crate::core::model::Source;

/// Codex usage adapter
pub struct CodexAdapter;

impl CodexAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl UsageAdapter for CodexAdapter {
    fn source(&self) -> Source {
        Source::Codex
    }

    fn find_data_paths(&self) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        let mut paths = Vec::new();

        // Check CODEX_HOME environment variable
        if let Ok(env_paths) = env::var("CODEX_HOME") {
            for raw in env_paths.split(',').map(str::trim).filter(|p| !p.is_empty()) {
                let path = PathBuf::from(raw);
                let sessions = path.join("sessions");
                if sessions.is_dir() {
                    paths.push(sessions);
                } else if path.is_dir() {
                    paths.push(path);
                }
            }
            if !paths.is_empty() {
                return Ok(paths);
            }
        }

        // Default path
        let home = dirs::home_dir().ok_or("Home directory not found")?;
        let codex_home = home.join(".codex");
        if codex_home.is_dir() {
            paths.push(codex_home);
        }

        Ok(paths)
    }

    fn load_entries(&self, paths: &[PathBuf]) -> Result<Vec<UsageEntry>, Box<dyn std::error::Error>> {
        let mut entries = Vec::new();

        for base_path in paths {
            let files = collect_jsonl_files(base_path);
            for file in files {
                if let Ok(file_entries) = parse_codex_jsonl(&file) {
                    entries.extend(file_entries);
                }
            }
        }

        Ok(entries)
    }

    fn aggregate_daily(&self, entries: &[UsageEntry]) -> Vec<DailyAggregate> {
        use std::collections::HashMap;

        let mut daily: HashMap<String, Vec<&UsageEntry>> = HashMap::new();

        for entry in entries {
            let date = entry.timestamp.split('T').next().unwrap_or(&entry.timestamp).to_string();
            daily.entry(date).or_default().push(entry);
        }

        let mut result: Vec<DailyAggregate> = daily
            .into_iter()
            .map(|(date, day_entries)| {
                let total_tokens: u64 = day_entries.iter().map(|e| e.total_tokens).sum();
                let input_tokens: u64 = day_entries.iter().map(|e| e.input_tokens).sum();
                let cache_read_tokens: u64 = day_entries.iter().map(|e| e.cache_read_tokens).sum();
                let output_tokens: u64 = day_entries.iter().map(|e| e.output_tokens).sum();
                let request_count = day_entries.len() as u64;

                let mut models_used: Vec<String> = day_entries
                    .iter()
                    .filter_map(|e| e.model.clone())
                    .collect::<std::collections::HashSet<_>>()
                    .into_iter()
                    .collect();
                models_used.sort();

                let model_breakdown = build_model_breakdown(
                    &day_entries.iter().map(|e| (*e).clone()).collect::<Vec<_>>(),
                );

                DailyAggregate {
                    date,
                    total_tokens,
                    input_tokens,
                    cache_read_tokens,
                    output_tokens,
                    request_count,
                    models_used,
                    model_breakdown,
                }
            })
            .collect();

        result.sort_by(|a, b| b.date.cmp(&a.date));
        result
    }

    fn aggregate_monthly(&self, entries: &[UsageEntry]) -> Vec<MonthlyAggregate> {
        use std::collections::HashMap;

        let mut monthly: HashMap<String, Vec<&UsageEntry>> = HashMap::new();

        for entry in entries {
            let month = entry.timestamp.split('-').take(2).collect::<Vec<_>>().join("-");
            monthly.entry(month).or_default().push(entry);
        }

        let mut result: Vec<MonthlyAggregate> = monthly
            .into_iter()
            .map(|(month, month_entries)| {
                let total_tokens: u64 = month_entries.iter().map(|e| e.total_tokens).sum();
                let input_tokens: u64 = month_entries.iter().map(|e| e.input_tokens).sum();
                let cache_read_tokens: u64 = month_entries.iter().map(|e| e.cache_read_tokens).sum();
                let output_tokens: u64 = month_entries.iter().map(|e| e.output_tokens).sum();
                let request_count = month_entries.len() as u64;

                let mut models_used: Vec<String> = month_entries
                    .iter()
                    .filter_map(|e| e.model.clone())
                    .collect::<std::collections::HashSet<_>>()
                    .into_iter()
                    .collect();
                models_used.sort();

                let model_breakdown = build_model_breakdown(
                    &month_entries.iter().map(|e| (*e).clone()).collect::<Vec<_>>(),
                );

                MonthlyAggregate {
                    month,
                    total_tokens,
                    input_tokens,
                    cache_read_tokens,
                    output_tokens,
                    request_count,
                    models_used,
                    model_breakdown,
                }
            })
            .collect();

        result.sort_by(|a, b| b.month.cmp(&a.month));
        result
    }

    fn aggregate_session(&self, entries: &[UsageEntry]) -> Vec<SessionAggregate> {
        use std::collections::HashMap;

        let mut sessions: HashMap<String, Vec<&UsageEntry>> = HashMap::new();

        for entry in entries {
            sessions
                .entry(entry.session_id.clone())
                .or_default()
                .push(entry);
        }

        let mut result: Vec<SessionAggregate> = sessions
            .into_iter()
            .map(|(session_id, session_entries)| {
                let total_tokens: u64 = session_entries.iter().map(|e| e.total_tokens).sum();
                let input_tokens: u64 = session_entries.iter().map(|e| e.input_tokens).sum();
                let cache_read_tokens: u64 = session_entries.iter().map(|e| e.cache_read_tokens).sum();
                let output_tokens: u64 = session_entries.iter().map(|e| e.output_tokens).sum();
                let request_count = session_entries.len() as u64;

                let last_activity = session_entries
                    .iter()
                    .map(|e| &e.timestamp)
                    .max()
                    .cloned();

                let mut models_used: Vec<String> = session_entries
                    .iter()
                    .filter_map(|e| e.model.clone())
                    .collect::<std::collections::HashSet<_>>()
                    .into_iter()
                    .collect();
                models_used.sort();

                let model_breakdown = build_model_breakdown(
                    &session_entries.iter().map(|e| (*e).clone()).collect::<Vec<_>>(),
                );

                let project_path = session_entries
                    .iter()
                    .find_map(|e| e.project_path.clone());

                SessionAggregate {
                    session_id,
                    project_path,
                    total_tokens,
                    input_tokens,
                    cache_read_tokens,
                    output_tokens,
                    request_count,
                    last_activity,
                    models_used,
                    model_breakdown,
                }
            })
            .collect();

        result.sort_by(|a, b| {
            b.last_activity
                .as_ref()
                .unwrap_or(&String::new())
                .cmp(a.last_activity.as_ref().unwrap_or(&String::new()))
        });
        result
    }

    fn aggregate_blocks(&self, entries: &[UsageEntry], block_duration_hours: i64) -> Vec<BlockAggregate> {
        aggregate_blocks_impl(entries, block_duration_hours)
    }
}

/// Collect all .jsonl files recursively
fn collect_jsonl_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_files_with_extension(dir, "jsonl", &mut files);
    files
}

fn collect_files_with_extension(dir: &Path, extension: &str, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.filter_map(std::result::Result::ok) {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        if file_type.is_file() && path.extension().is_some_and(|ext| ext == extension) {
            files.push(path);
        } else if file_type.is_dir() {
            collect_files_with_extension(&path, extension, files);
        }
    }
}

/// Parse a Codex JSONL file into usage entries
fn parse_codex_jsonl(path: &Path) -> Result<Vec<UsageEntry>, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let mut entries = Vec::new();
    let mut current_model: Option<String> = None;
    let mut current_cwd: Option<String> = None;

    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }

        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };

        // Check for turn_context to get model and cwd
        if value.get("type").and_then(|t| t.as_str()) == Some("turn_context") {
            let payload = value.get("payload");
            if let Some(model) = payload
                .and_then(|p| p.get("model"))
                .and_then(|m| m.as_str())
            {
                current_model = Some(model.to_string());
            }
            if let Some(cwd) = payload
                .and_then(|p| p.get("cwd"))
                .and_then(|c| c.as_str())
            {
                current_cwd = Some(cwd.to_string());
            }
            continue;
        }

        // Check for event_msg with token_count
        if value.get("type").and_then(|t| t.as_str()) == Some("event_msg") {
            let Some(payload) = value.get("payload") else {
                continue;
            };

            if payload.get("type").and_then(|t| t.as_str()) != Some("token_count") {
                continue;
            }

            let info = payload.get("info");
            let usage = info
                .and_then(|i| i.get("last_token_usage"))
                .or_else(|| info.and_then(|i| i.get("total_token_usage")));

            let Some(usage) = usage else {
                continue;
            };

            let input_tokens = usage
                .get("input_tokens")
                .and_then(|t| t.as_u64())
                .unwrap_or(0);

            let output_tokens = usage
                .get("output_tokens")
                .and_then(|t| t.as_u64())
                .unwrap_or(0);

            let cached_input_tokens = usage
                .get("cached_input_tokens")
                .and_then(|t| t.as_u64())
                .unwrap_or(0);

            let reasoning_output_tokens = usage
                .get("reasoning_output_tokens")
                .and_then(|t| t.as_u64())
                .unwrap_or(0);

            let total_tokens = input_tokens + output_tokens + reasoning_output_tokens;

            if total_tokens == 0 {
                continue;
            }

            let model = payload
                .get("model")
                .or_else(|| payload.get("model_name"))
                .and_then(|m| m.as_str())
                .map(|s| s.to_string())
                .or_else(|| current_model.clone())
                .unwrap_or_else(|| "gpt-5".to_string());

            let timestamp = value
                .get("timestamp")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();

            let session_id = extract_session_id(path);

            entries.push(UsageEntry {
                session_id,
                timestamp,
                model: Some(model),
                input_tokens: input_tokens.saturating_sub(cached_input_tokens),
                output_tokens,
                cache_creation_tokens: 0,
                cache_read_tokens: cached_input_tokens,
                total_tokens,
                project_path: current_cwd.clone(),
            });
        }
    }

    Ok(entries)
}

/// Extract session ID from file path
fn extract_session_id(path: &Path) -> String {
    let components: Vec<_> = path
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();

    // Look for sessions/{session_id}.jsonl pattern
    if let Some(sessions_idx) = components.iter().position(|c| *c == "sessions") {
        if components.len() > sessions_idx + 1 {
            let session_file = components.last().unwrap();
            return session_file.replace(".jsonl", "");
        }
    }

    // Fallback to filename
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string()
}
