use std::env;
use std::path::{Path, PathBuf};

use super::{build_model_breakdown, DailyAggregate, MonthlyAggregate, SessionAggregate, UsageAdapter, UsageEntry};
use crate::core::model::Source;

/// OpenCode usage adapter - reads from SQLite database
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
                if path.join("opencode.db").exists() {
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
        if opencode_path.join("opencode.db").exists() {
            paths.push(opencode_path);
        }

        Ok(paths)
    }

    fn load_entries(&self, paths: &[PathBuf]) -> Result<Vec<UsageEntry>, Box<dyn std::error::Error>> {
        let mut entries = Vec::new();

        for base_path in paths {
            let db_path = base_path.join("opencode.db");
            if db_path.exists() {
                if let Ok(file_entries) = load_from_sqlite(&db_path) {
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
}

/// Load usage entries from OpenCode SQLite database
fn load_from_sqlite(db_path: &Path) -> Result<Vec<UsageEntry>, Box<dyn std::error::Error>> {
    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;

    let session_dirs = load_session_dirs(&conn)?;

    let mut stmt = conn.prepare(
        "SELECT data FROM message WHERE json_extract(data, '$.role') = 'assistant'",
    )?;

    let entries = stmt
        .query_map([], |row| {
            let data_str: String = row.get(0)?;
            Ok(data_str)
        })?
        .filter_map(|r| r.ok())
        .filter_map(|data_str| parse_message_json(&data_str, &session_dirs))
        .collect();

    Ok(entries)
}

fn load_session_dirs(conn: &rusqlite::Connection) -> Result<std::collections::HashMap<String, String>, Box<dyn std::error::Error>> {
    let mut stmt = conn.prepare("SELECT id, directory FROM session")?;
    let map = stmt.query_map([], |row| {
        let id: String = row.get(0)?;
        let directory: String = row.get(1)?;
        Ok((id, directory))
    })?
    .filter_map(|r| r.ok())
    .collect();
    Ok(map)
}

/// Parse a message JSON string into a UsageEntry
fn parse_message_json(json_str: &str, session_dirs: &std::collections::HashMap<String, String>) -> Option<UsageEntry> {
    let value: serde_json::Value = serde_json::from_str(json_str).ok()?;

    // Skip entries without tokens
    let tokens = value.get("tokens")?;

    let input_tokens = tokens.get("input").and_then(|t| t.as_u64()).unwrap_or(0);
    let output_tokens = tokens.get("output").and_then(|t| t.as_u64()).unwrap_or(0);
    let cache = tokens.get("cache");
    let cache_creation_tokens = cache
        .and_then(|c| c.get("write"))
        .and_then(|t| t.as_u64())
        .unwrap_or(0);
    let cache_read_tokens = cache
        .and_then(|c| c.get("read"))
        .and_then(|t| t.as_u64())
        .unwrap_or(0);

    let total_tokens = tokens
        .get("total")
        .and_then(|t| t.as_u64())
        .unwrap_or(input_tokens + output_tokens + cache_creation_tokens + cache_read_tokens);

    if total_tokens == 0 {
        return None;
    }

    let model = value
        .get("modelID")
        .and_then(|m| m.as_str())
        .map(|s| s.to_string());

    let timestamp_ms = value
        .get("time")
        .and_then(|t| t.get("created"))
        .and_then(|t| t.as_i64())
        .unwrap_or(0);

    let timestamp = if timestamp_ms > 0 {
        let dt = chrono::DateTime::from_timestamp_millis(timestamp_ms).unwrap_or_default();
        dt.to_rfc3339()
    } else {
        String::new()
    };

    let session_id = value
        .get("sessionID")
        .and_then(|s| s.as_str())
        .unwrap_or("unknown")
        .to_string();

    let project_path = session_dirs.get(&session_id).cloned();

    Some(UsageEntry {
        session_id,
        timestamp,
        model,
        input_tokens,
        output_tokens,
        cache_creation_tokens,
        cache_read_tokens,
        total_tokens,
        project_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_message_json() {
        let json = r#"{
            "role": "assistant",
            "time": {"created": 1768902371377},
            "modelID": "antigravity-gemini-3-pro-high",
            "sessionID": "test-session",
            "tokens": {
                "input": 65680,
                "output": 4091,
                "cache": {"read": 1000, "write": 0}
            }
        }"#;

        let entry = parse_message_json(json, &std::collections::HashMap::new()).unwrap();
        assert_eq!(entry.input_tokens, 65680);
        assert_eq!(entry.output_tokens, 4091);
        assert_eq!(entry.cache_read_tokens, 1000);
        assert_eq!(entry.cache_creation_tokens, 0);
        assert_eq!(entry.total_tokens, 65680 + 4091 + 1000);
        assert_eq!(entry.model.unwrap(), "antigravity-gemini-3-pro-high");
    }

    #[test]
    fn test_parse_message_json_no_cache() {
        let json = r#"{
            "role": "assistant",
            "time": {"created": 1768902371377},
            "modelID": "gpt-5.5",
            "sessionID": "test-session",
            "tokens": {"input": 100, "output": 50}
        }"#;

        let entry = parse_message_json(json, &std::collections::HashMap::new()).unwrap();
        assert_eq!(entry.input_tokens, 100);
        assert_eq!(entry.output_tokens, 50);
        assert_eq!(entry.cache_read_tokens, 0);
        assert_eq!(entry.total_tokens, 150);
    }

    #[test]
    fn test_parse_message_json_skip_zero_total() {
        let json = r#"{
            "role": "assistant",
            "time": {"created": 1768902371377},
            "tokens": {"input": 0, "output": 0}
        }"#;

        assert!(parse_message_json(json, &std::collections::HashMap::new()).is_none());
    }

    #[test]
    fn test_parse_message_json_total_includes_cache() {
        let json = r#"{
            "role": "assistant",
            "time": {"created": 1768902371377},
            "modelID": "gpt-5.5",
            "sessionID": "test-session",
            "tokens": {
                "input": 1000,
                "output": 500,
                "cache": {"read": 2000, "write": 0}
            }
        }"#;

        let entry = parse_message_json(json, &std::collections::HashMap::new()).unwrap();
        assert_eq!(entry.input_tokens, 1000);
        assert_eq!(entry.output_tokens, 500);
        assert_eq!(entry.cache_read_tokens, 2000);
        // total should include cache_read
        assert_eq!(entry.total_tokens, 1000 + 500 + 2000);
    }

    #[test]
    fn test_parse_message_json_skip_no_tokens() {
        let json = r#"{"role": "assistant"}"#;
        assert!(parse_message_json(json, &std::collections::HashMap::new()).is_none());
    }

    #[test]
    fn test_aggregate_daily() {
        let adapter = OpenCodeAdapter;
        let entries = vec![
            UsageEntry {
                session_id: "s1".to_string(),
                timestamp: "2026-05-29T10:00:00Z".to_string(),
                model: Some("gpt-5.5".to_string()),
                input_tokens: 100,
                output_tokens: 50,
                cache_creation_tokens: 0,
                cache_read_tokens: 30,
                total_tokens: 180,
                project_path: None,
            },
            UsageEntry {
                session_id: "s1".to_string(),
                timestamp: "2026-05-29T11:00:00Z".to_string(),
                model: Some("gpt-5.5".to_string()),
                input_tokens: 200,
                output_tokens: 80,
                cache_creation_tokens: 0,
                cache_read_tokens: 60,
                total_tokens: 340,
                project_path: None,
            },
            UsageEntry {
                session_id: "s2".to_string(),
                timestamp: "2026-05-30T10:00:00Z".to_string(),
                model: Some("claude-sonnet-4".to_string()),
                input_tokens: 150,
                output_tokens: 70,
                cache_creation_tokens: 0,
                cache_read_tokens: 40,
                total_tokens: 260,
                project_path: None,
            },
        ];

        let daily = adapter.aggregate_daily(&entries);
        assert_eq!(daily.len(), 2);

        // Sorted by date desc
        assert_eq!(daily[0].date, "2026-05-30");
        assert_eq!(daily[0].total_tokens, 260);
        assert_eq!(daily[0].input_tokens, 150);
        assert_eq!(daily[0].cache_read_tokens, 40);
        assert_eq!(daily[0].output_tokens, 70);
        assert_eq!(daily[0].request_count, 1);
        assert_eq!(daily[0].model_breakdown.len(), 1);
        assert_eq!(daily[0].model_breakdown[0].model, "claude-sonnet-4");

        assert_eq!(daily[1].date, "2026-05-29");
        assert_eq!(daily[1].total_tokens, 520);
        assert_eq!(daily[1].input_tokens, 300);
        assert_eq!(daily[1].cache_read_tokens, 90);
        assert_eq!(daily[1].output_tokens, 130);
        assert_eq!(daily[1].request_count, 2);
        assert_eq!(daily[1].model_breakdown.len(), 1);
        assert_eq!(daily[1].model_breakdown[0].model, "gpt-5.5");
        assert_eq!(daily[1].model_breakdown[0].input_tokens, 300);
        assert_eq!(daily[1].model_breakdown[0].cache_read_tokens, 90);
    }

    #[test]
    fn test_aggregate_monthly() {
        let adapter = OpenCodeAdapter;
        let entries = vec![
            UsageEntry {
                session_id: "s1".to_string(),
                timestamp: "2026-05-29T10:00:00Z".to_string(),
                model: Some("gpt-5.5".to_string()),
                input_tokens: 100,
                output_tokens: 50,
                cache_creation_tokens: 0,
                cache_read_tokens: 30,
                total_tokens: 180,
                project_path: None,
            },
            UsageEntry {
                session_id: "s2".to_string(),
                timestamp: "2026-06-01T10:00:00Z".to_string(),
                model: Some("claude-sonnet-4".to_string()),
                input_tokens: 200,
                output_tokens: 80,
                cache_creation_tokens: 0,
                cache_read_tokens: 60,
                total_tokens: 340,
                project_path: None,
            },
        ];

        let monthly = adapter.aggregate_monthly(&entries);
        assert_eq!(monthly.len(), 2);
        assert_eq!(monthly[0].month, "2026-06");
        assert_eq!(monthly[1].month, "2026-05");
    }

    #[test]
    fn test_aggregate_session() {
        let adapter = OpenCodeAdapter;
        let entries = vec![
            UsageEntry {
                session_id: "s1".to_string(),
                timestamp: "2026-05-29T10:00:00Z".to_string(),
                model: Some("gpt-5.5".to_string()),
                input_tokens: 100,
                output_tokens: 50,
                cache_creation_tokens: 0,
                cache_read_tokens: 30,
                total_tokens: 180,
                project_path: Some("/Users/test/project".to_string()),
            },
            UsageEntry {
                session_id: "s1".to_string(),
                timestamp: "2026-05-29T11:00:00Z".to_string(),
                model: Some("gpt-5.5".to_string()),
                input_tokens: 200,
                output_tokens: 80,
                cache_creation_tokens: 0,
                cache_read_tokens: 60,
                total_tokens: 340,
                project_path: Some("/Users/test/project".to_string()),
            },
        ];

        let sessions = adapter.aggregate_session(&entries);
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].session_id, "s1");
        assert_eq!(sessions[0].total_tokens, 520);
        assert_eq!(sessions[0].request_count, 2);
        assert_eq!(sessions[0].last_activity.as_ref().unwrap(), "2026-05-29T11:00:00Z");
        assert_eq!(sessions[0].project_path.as_ref().unwrap(), "/Users/test/project");
    }
}
