use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use super::{DailyAggregate, MonthlyAggregate, SessionAggregate, UsageAdapter, UsageEntry};
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
        let mut daily: std::collections::HashMap<String, DailyAggregate> = std::collections::HashMap::new();
        
        for entry in entries {
            let date = entry.timestamp.split('T').next().unwrap_or(&entry.timestamp).to_string();
            let aggregate = daily.entry(date.clone()).or_insert_with(|| DailyAggregate {
                date,
                total_tokens: 0,
                input_tokens: 0,
                output_tokens: 0,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
                request_count: 0,
                models_used: Vec::new(),
            });
            
            aggregate.total_tokens += entry.total_tokens;
            aggregate.input_tokens += entry.input_tokens;
            aggregate.output_tokens += entry.output_tokens;
            aggregate.cache_creation_tokens += entry.cache_creation_tokens;
            aggregate.cache_read_tokens += entry.cache_read_tokens;
            aggregate.request_count += 1;
            
            if let Some(model) = &entry.model {
                if !aggregate.models_used.contains(model) {
                    aggregate.models_used.push(model.clone());
                }
            }
        }
        
        let mut result: Vec<DailyAggregate> = daily.into_values().collect();
        result.sort_by(|a, b| b.date.cmp(&a.date));
        result
    }

    fn aggregate_monthly(&self, entries: &[UsageEntry]) -> Vec<MonthlyAggregate> {
        let mut monthly: std::collections::HashMap<String, MonthlyAggregate> = std::collections::HashMap::new();
        
        for entry in entries {
            let month = entry.timestamp.split('-').take(2).collect::<Vec<_>>().join("-");
            let aggregate = monthly.entry(month.clone()).or_insert_with(|| MonthlyAggregate {
                month,
                total_tokens: 0,
                input_tokens: 0,
                output_tokens: 0,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
                request_count: 0,
                models_used: Vec::new(),
            });
            
            aggregate.total_tokens += entry.total_tokens;
            aggregate.input_tokens += entry.input_tokens;
            aggregate.output_tokens += entry.output_tokens;
            aggregate.cache_creation_tokens += entry.cache_creation_tokens;
            aggregate.cache_read_tokens += entry.cache_read_tokens;
            aggregate.request_count += 1;
            
            if let Some(model) = &entry.model {
                if !aggregate.models_used.contains(model) {
                    aggregate.models_used.push(model.clone());
                }
            }
        }
        
        let mut result: Vec<MonthlyAggregate> = monthly.into_values().collect();
        result.sort_by(|a, b| b.month.cmp(&a.month));
        result
    }

    fn aggregate_session(&self, entries: &[UsageEntry]) -> Vec<SessionAggregate> {
        let mut sessions: std::collections::HashMap<String, SessionAggregate> = std::collections::HashMap::new();
        
        for entry in entries {
            let aggregate = sessions.entry(entry.session_id.clone()).or_insert_with(|| SessionAggregate {
                session_id: entry.session_id.clone(),
                project_path: None,
                total_tokens: 0,
                input_tokens: 0,
                output_tokens: 0,
                last_activity: None,
                models_used: Vec::new(),
            });
            
            aggregate.total_tokens += entry.total_tokens;
            aggregate.input_tokens += entry.input_tokens;
            aggregate.output_tokens += entry.output_tokens;
            
            if let Some(last) = &aggregate.last_activity {
                if entry.timestamp > *last {
                    aggregate.last_activity = Some(entry.timestamp.clone());
                }
            } else {
                aggregate.last_activity = Some(entry.timestamp.clone());
            }
            
            if let Some(model) = &entry.model {
                if !aggregate.models_used.contains(model) {
                    aggregate.models_used.push(model.clone());
                }
            }
        }
        
        let mut result: Vec<SessionAggregate> = sessions.into_values().collect();
        result.sort_by(|a, b| {
            b.last_activity
                .as_ref()
                .unwrap_or(&String::new())
                .cmp(a.last_activity.as_ref().unwrap_or(&String::new()))
        });
        result
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
    
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        
        // Check for turn_context to get model
        if value.get("type").and_then(|t| t.as_str()) == Some("turn_context") {
            if let Some(model) = value.get("payload")
                .and_then(|p| p.get("model"))
                .and_then(|m| m.as_str())
            {
                current_model = Some(model.to_string());
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
            let usage = info.and_then(|i| i.get("last_token_usage"))
                .or_else(|| info.and_then(|i| i.get("total_token_usage")));
            
            let Some(usage) = usage else {
                continue;
            };
            
            let input_tokens = usage.get("input_tokens")
                .and_then(|t| t.as_u64())
                .unwrap_or(0);
            
            let output_tokens = usage.get("output_tokens")
                .and_then(|t| t.as_u64())
                .unwrap_or(0);
            
            let cached_input_tokens = usage.get("cached_input_tokens")
                .and_then(|t| t.as_u64())
                .unwrap_or(0);
            
            let reasoning_output_tokens = usage.get("reasoning_output_tokens")
                .and_then(|t| t.as_u64())
                .unwrap_or(0);
            
            let total_tokens = input_tokens + output_tokens + reasoning_output_tokens;
            
            if total_tokens == 0 {
                continue;
            }
            
            let model = payload.get("model")
                .or_else(|| payload.get("model_name"))
                .and_then(|m| m.as_str())
                .map(|s| s.to_string())
                .or_else(|| current_model.clone())
                .unwrap_or_else(|| "gpt-5".to_string());
            
            let timestamp = value.get("timestamp")
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
                cost_usd: None,
            });
        }
    }
    
    Ok(entries)
}

/// Extract session ID from file path
fn extract_session_id(path: &Path) -> String {
    let components: Vec<_> = path.components()
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
