use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use super::{DailyAggregate, MonthlyAggregate, SessionAggregate, UsageAdapter, UsageEntry};
use crate::core::model::Source;

/// Claude Code usage adapter
pub struct ClaudeAdapter;

impl ClaudeAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl UsageAdapter for ClaudeAdapter {
    fn source(&self) -> Source {
        Source::Claude
    }

    fn find_data_paths(&self) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        let mut paths = Vec::new();
        
        // Check CLAUDE_CONFIG_DIR environment variable
        if let Ok(env_paths) = env::var("CLAUDE_CONFIG_DIR") {
            for raw in env_paths.split(',').map(str::trim).filter(|p| !p.is_empty()) {
                let path = expand_home_path(raw);
                if path.join("projects").is_dir() {
                    paths.push(path);
                }
            }
            if !paths.is_empty() {
                return Ok(paths);
            }
        }
        
        // Default paths
        let home = dirs::home_dir().ok_or("Home directory not found")?;
        let xdg = env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| home.join(".config"));
        
        for path in [xdg.join("claude"), home.join(".claude")] {
            if path.join("projects").is_dir() {
                paths.push(path);
            }
        }
        
        Ok(paths)
    }

    fn load_entries(&self, paths: &[PathBuf]) -> Result<Vec<UsageEntry>, Box<dyn std::error::Error>> {
        let mut entries = Vec::new();
        
        for base_path in paths {
            let projects_dir = base_path.join("projects");
            if !projects_dir.is_dir() {
                continue;
            }
            
            let files = collect_jsonl_files(&projects_dir);
            for file in files {
                if let Ok(file_entries) = parse_claude_jsonl(&file) {
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

/// Expand ~ to home directory
fn expand_home_path(raw: &str) -> PathBuf {
    if raw == "~" {
        if let Some(home) = dirs::home_dir() {
            return home;
        }
    }
    if let Some(rest) = raw.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(raw)
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

/// Parse a Claude JSONL file into usage entries
fn parse_claude_jsonl(path: &Path) -> Result<Vec<UsageEntry>, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let mut entries = Vec::new();
    
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        
        // Skip entries without usage data
        let Some(usage) = value.get("message").and_then(|m| m.get("usage")) else {
            continue;
        };
        
        let timestamp = value.get("timestamp")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();
        
        let model = value.get("message")
            .and_then(|m| m.get("model"))
            .and_then(|m| m.as_str())
            .map(|s| s.to_string());
        
        let input_tokens = usage.get("input_tokens")
            .and_then(|t| t.as_u64())
            .unwrap_or(0);
        
        let output_tokens = usage.get("output_tokens")
            .and_then(|t| t.as_u64())
            .unwrap_or(0);
        
        let cache_creation_tokens = usage.get("cache_creation_input_tokens")
            .and_then(|t| t.as_u64())
            .unwrap_or(0);
        
        let cache_read_tokens = usage.get("cache_read_input_tokens")
            .and_then(|t| t.as_u64())
            .unwrap_or(0);
        
        let total_tokens = input_tokens + output_tokens + cache_creation_tokens + cache_read_tokens;
        
        let cost_usd = value.get("costUSD")
            .and_then(|c| c.as_f64());
        
        let session_id = extract_session_id(path);
        
        if total_tokens > 0 {
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
    }
    
    Ok(entries)
}

/// Extract session ID from file path
fn extract_session_id(path: &Path) -> String {
    let components: Vec<_> = path.components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();
    
    // Look for projects/{project}/{session}.jsonl pattern
    if let Some(projects_idx) = components.iter().position(|c| *c == "projects") {
        if components.len() > projects_idx + 2 {
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
