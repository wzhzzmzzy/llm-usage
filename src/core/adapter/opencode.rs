use std::env;
use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde_json::Value;

use super::{DailyAggregate, MonthlyAggregate, SessionAggregate, UsageAdapter, UsageEntry};
use crate::core::model::Source;

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
            let db_path = base_path.join("opencode.db");
            if db_path.is_file() {
                match load_from_sqlite(&db_path) {
                    Ok(db_entries) => {
                        tracing::info!("OpenCode: loaded {} entries from {}", db_entries.len(), db_path.display());
                        entries.extend(db_entries);
                    }
                    Err(e) => {
                        tracing::warn!("OpenCode: failed to load from {}: {}", db_path.display(), e);
                    }
                }
            } else {
                tracing::debug!("OpenCode: no opencode.db at {}", db_path.display());
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

            if aggregate.project_path.is_none() {
                aggregate.project_path = entry.session_id.split('/').next().map(|s| s.to_string());
            }

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

fn load_from_sqlite(db_path: &Path) -> Result<Vec<UsageEntry>, Box<dyn std::error::Error>> {
    let conn = Connection::open_with_flags(db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;

    let mut stmt = conn.prepare(
        "SELECT id, directory, model, tokens_input, tokens_output, tokens_reasoning, \
         tokens_cache_read, tokens_cache_write, cost, time_created \
         FROM session WHERE tokens_input > 0 OR tokens_output > 0"
    )?;

    let entries = stmt.query_map([], |row| {
        let id: String = row.get(0)?;
        let directory: String = row.get(1)?;
        let model_json: Option<String> = row.get(2)?;
        let tokens_input: i64 = row.get(3)?;
        let tokens_output: i64 = row.get(4)?;
        let tokens_reasoning: i64 = row.get(5)?;
        let tokens_cache_read: i64 = row.get(6)?;
        let tokens_cache_write: i64 = row.get(7)?;
        let cost: f64 = row.get(8)?;
        let time_created: i64 = row.get(9)?;

        let model = model_json.and_then(|json| parse_model_id(&json));

        let timestamp = if time_created > 0 {
            let dt = chrono::DateTime::from_timestamp_millis(time_created)
                .unwrap_or_default();
            dt.to_rfc3339()
        } else {
            String::new()
        };

        let input_tokens = tokens_input.max(0) as u64;
        let output_tokens = (tokens_output + tokens_reasoning).max(0) as u64;
        let cache_read = tokens_cache_read.max(0) as u64;
        let cache_write = tokens_cache_write.max(0) as u64;

        Ok(UsageEntry {
            session_id: id,
            timestamp,
            model,
            input_tokens,
            output_tokens,
            cache_creation_tokens: cache_write,
            cache_read_tokens: cache_read,
            total_tokens: input_tokens + output_tokens + cache_read + cache_write,
            cost_usd: if cost > 0.0 { Some(cost) } else { None },
        })
    })?.collect::<Result<Vec<_>, _>>()?;

    Ok(entries)
}

fn parse_model_id(model_json: &str) -> Option<String> {
    let value: Value = serde_json::from_str(model_json).ok()?;
    value.get("id").and_then(|v| v.as_str()).map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_model_id() {
        assert_eq!(
            parse_model_id(r#"{"id":"mimo-v2.5-pro","providerID":"xiaomi-token-plan-cn"}"#),
            Some("mimo-v2.5-pro".to_string())
        );
        assert_eq!(parse_model_id("invalid"), None);
    }
}
