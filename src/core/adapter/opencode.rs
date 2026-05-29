use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use super::{DailyAggregate, MonthlyAggregate, SessionAggregate, UsageAdapter, UsageEntry};
use crate::core::model::Source;

/// OpenCode usage adapter
pub struct OpenCodeAdapter;

impl OpenCodeAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl UsageAdapter for OpenCodeAdapter {
    fn source(&self) -> Source {
        Source::Opencode
    }

    fn find_data_paths(&self) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        let mut paths = Vec::new();
        
        // Check OPENCODE_DATA_DIR environment variable
        if let Ok(env_paths) = env::var("OPENCODE_DATA_DIR") {
            for raw in env_paths.split(',').map(str::trim).filter(|p| !p.is_empty()) {
                let path = PathBuf::from(raw);
                if path.is_dir() {
                    paths.push(path);
                }
            }
            if !paths.is_empty() {
                return Ok(paths);
            }
        }
        
        // Default path
        let home = dirs::home_dir().ok_or("Home directory not found")?;
        let opencode_path = home.join(".local/share/opencode");
        if opencode_path.is_dir() {
            paths.push(opencode_path);
        }
        
        Ok(paths)
    }

    fn load_entries(&self, paths: &[PathBuf]) -> Result<Vec<UsageEntry>, Box<dyn std::error::Error>> {
        let mut entries = Vec::new();
        
        for base_path in paths {
            let files = collect_jsonl_files(base_path);
            for file in files {
                if let Ok(file_entries) = parse_opencode_jsonl(&file) {
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

/// Parse an OpenCode JSONL file into usage entries
fn parse_opencode_jsonl(path: &Path) -> Result<Vec<UsageEntry>, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let mut entries = Vec::new();
    
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        
        // Skip entries without tokens
        let Some(tokens) = value.get("tokens") else {
            continue;
        };
        
        let input_tokens = tokens.get("input")
            .and_then(|t| t.as_u64())
            .unwrap_or(0);
        
        let output_tokens = tokens.get("output")
            .and_then(|t| t.as_u64())
            .unwrap_or(0);
        
        let cache = tokens.get("cache");
        let cache_creation_tokens = cache
            .and_then(|c| c.get("write"))
            .and_then(|t| t.as_u64())
            .unwrap_or(0);
        
        let cache_read_tokens = cache
            .and_then(|c| c.get("read"))
            .and_then(|t| t.as_u64())
            .unwrap_or(0);
        
        let total_tokens = tokens.get("total")
            .and_then(|t| t.as_u64())
            .unwrap_or(input_tokens + output_tokens + cache_creation_tokens + cache_read_tokens);
        
        if total_tokens == 0 {
            continue;
        }
        
        let model = value.get("modelID")
            .and_then(|m| m.as_str())
            .map(|s| s.to_string());
        
        let timestamp_ms = value.get("time")
            .and_then(|t| t.get("created"))
            .and_then(|t| t.as_i64())
            .unwrap_or(0);
        
        let timestamp = if timestamp_ms > 0 {
            let dt = chrono::DateTime::from_timestamp_millis(timestamp_ms)
                .unwrap_or_default();
            dt.to_rfc3339()
        } else {
            String::new()
        };
        
        let session_id = value.get("sessionID")
            .and_then(|s| s.as_str())
            .unwrap_or("unknown")
            .to_string();
        
        let cost_usd = value.get("cost")
            .and_then(|c| c.as_f64())
            .filter(|c| *c > 0.0);
        
        entries.push(UsageEntry {
            session_id,
            timestamp,
            model,
            input_tokens,
            output_tokens,
            cache_creation_tokens,
            cache_read_tokens,
            total_tokens,
            cost_usd,
        });
    }
    
    Ok(entries)
}
