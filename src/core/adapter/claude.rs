use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use super::{aggregate_blocks_impl, build_model_breakdown, BlockAggregate, DailyAggregate, MonthlyAggregate, SessionAggregate, UsageAdapter, UsageEntry};
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
        use rayon::prelude::*;

        let mut all_files = Vec::new();
        for base_path in paths {
            let projects_dir = base_path.join("projects");
            if !projects_dir.is_dir() {
                continue;
            }
            all_files.extend(collect_jsonl_files(&projects_dir));
        }

        let entries = all_files
            .par_iter()
            .filter_map(|file| parse_claude_jsonl(file).ok())
            .flatten()
            .collect();

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
                let reasoning_tokens: u64 = day_entries.iter().map(|e| e.reasoning_tokens).sum();
                let request_count = day_entries.len() as u64;

                let mut models_used: Vec<String> = day_entries
                    .iter()
                    .filter_map(|e| e.model.clone())
                    .collect::<std::collections::HashSet<_>>()
                    .into_iter()
                    .collect();
                models_used.sort();

                let model_breakdown = build_model_breakdown(&day_entries);

                DailyAggregate {
                    date,
                    total_tokens,
                    input_tokens,
                    cache_read_tokens,
                    output_tokens,
                    reasoning_tokens,
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
                let reasoning_tokens: u64 = month_entries.iter().map(|e| e.reasoning_tokens).sum();
                let request_count = month_entries.len() as u64;

                let mut models_used: Vec<String> = month_entries
                    .iter()
                    .filter_map(|e| e.model.clone())
                    .collect::<std::collections::HashSet<_>>()
                    .into_iter()
                    .collect();
                models_used.sort();

                let model_breakdown = build_model_breakdown(&month_entries);

                MonthlyAggregate {
                    month,
                    total_tokens,
                    input_tokens,
                    cache_read_tokens,
                    output_tokens,
                    reasoning_tokens,
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
                let reasoning_tokens: u64 = session_entries.iter().map(|e| e.reasoning_tokens).sum();
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

                let model_breakdown = build_model_breakdown(&session_entries);

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
                    reasoning_tokens,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_session_id() {
        let path = PathBuf::from("/Users/amber/.claude/projects/-Users-amber-Projects-ralph-llm-usage/abc123.jsonl");
        assert_eq!(extract_session_id(&path), "abc123");
    }

    fn write_temp_jsonl(name: &str, content: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("llm-usage-tests");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, content).unwrap();
        path
    }

    const ASSISTANT_LINE: &str = r#"{"type":"assistant","timestamp":"2026-05-29T10:00:00.000Z","message":{"model":"claude-sonnet-4","usage":{"input_tokens":65680,"output_tokens":4091,"cache_creation_input_tokens":500,"cache_read_input_tokens":1000}}}"#;

    #[test]
    fn test_basic_usage_entry() {
        let path = write_temp_jsonl("claude-basic.jsonl", ASSISTANT_LINE);
        let entries = parse_claude_jsonl(&path).unwrap();
        assert_eq!(entries.len(), 1);
        let e = &entries[0];
        assert_eq!(e.session_id, "claude-basic");
        assert_eq!(e.timestamp, "2026-05-29T10:00:00.000Z");
        assert_eq!(e.model.as_deref(), Some("claude-sonnet-4"));
        assert_eq!(e.input_tokens, 65680);
        assert_eq!(e.output_tokens, 4091);
        assert_eq!(e.cache_creation_tokens, 500);
        assert_eq!(e.cache_read_tokens, 1000);
        assert_eq!(e.reasoning_tokens, 0);
        assert_eq!(e.total_tokens, 65680 + 4091 + 500 + 1000);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_lines_without_usage_skipped() {
        let content = concat!(
            "{\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":\"hello\"}}\n",
            "{\"type\":\"summary\",\"summary\":\"token usage discussed\"}\n",
            "not even json\n",
        );
        let path = write_temp_jsonl("claude-nousage.jsonl", content);
        let entries = parse_claude_jsonl(&path).unwrap();
        assert!(entries.is_empty());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_cwd_extracted_to_project_path() {
        let content = [
            "{\"type\":\"user\",\"cwd\":\"/Users/test/proj\",\"message\":{\"role\":\"user\",\"content\":\"hi\"}}\n",
            ASSISTANT_LINE,
            "\n",
        ]
        .concat();
        let path = write_temp_jsonl("claude-cwd.jsonl", &content);
        let entries = parse_claude_jsonl(&path).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].project_path.as_deref(), Some("/Users/test/proj"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_null_usage_and_missing_message_no_entry() {
        let content = concat!(
            "{\"message\":{\"model\":\"claude-sonnet-4\",\"usage\":null}}\n",
            "{\"timestamp\":\"2026-05-29T10:00:00.000Z\",\"note\":\"mentions \\\"usage\\\" here\"}\n",
        );
        let path = write_temp_jsonl("claude-nullusage.jsonl", content);
        let entries = parse_claude_jsonl(&path).unwrap();
        assert!(entries.is_empty());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_zero_total_no_entry() {
        let content = r#"{"message":{"model":"claude-sonnet-4","usage":{"input_tokens":0,"output_tokens":0}}}"#;
        let path = write_temp_jsonl("claude-zero.jsonl", content);
        let entries = parse_claude_jsonl(&path).unwrap();
        assert!(entries.is_empty());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_missing_model_and_timestamp_default() {
        let content = r#"{"message":{"usage":{"input_tokens":100,"output_tokens":50}}}"#;
        let path = write_temp_jsonl("claude-nomodel.jsonl", content);
        let entries = parse_claude_jsonl(&path).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].model, None);
        assert_eq!(entries[0].timestamp, "");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_multiple_entries_preserve_file_order() {
        let first = r#"{"timestamp":"2026-05-29T10:00:00.000Z","message":{"model":"claude-sonnet-4","usage":{"input_tokens":100,"output_tokens":50}}}"#;
        let second = r#"{"timestamp":"2026-05-29T11:00:00.000Z","message":{"model":"claude-opus-4","usage":{"input_tokens":200,"output_tokens":80}}}"#;
        let content = format!("{}\n{}\n", first, second);
        let path = write_temp_jsonl("claude-multi.jsonl", &content);
        let entries = parse_claude_jsonl(&path).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].total_tokens, 150);
        assert_eq!(entries[1].total_tokens, 280);
        assert_eq!(entries[1].model.as_deref(), Some("claude-opus-4"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_wrong_typed_usage_field_skips_line() {
        let content = r#"{"message":{"model":"claude-sonnet-4","usage":{"input_tokens":"100","output_tokens":50}}}"#;
        let path = write_temp_jsonl("claude-wrongtype.jsonl", content);
        let entries = parse_claude_jsonl(&path).unwrap();
        assert!(entries.is_empty());
        let _ = std::fs::remove_file(&path);
    }

    type EntryTuple = (String, String, Option<String>, u64, u64, u64, u64, Option<String>);

    fn entry_tuple(e: &UsageEntry) -> EntryTuple {
        (
            e.session_id.clone(),
            e.timestamp.clone(),
            e.model.clone(),
            e.input_tokens,
            e.output_tokens,
            e.cache_creation_tokens,
            e.cache_read_tokens,
            e.project_path.clone(),
        )
    }

    /// Reference implementation (Value DOM) kept as the differential oracle.
    fn reference_parse(path: &Path) -> Vec<EntryTuple> {
        let Ok(content) = fs::read_to_string(path) else { return Vec::new() };
        let mut out = Vec::new();
        let mut project_path: Option<String> = None;
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if !line.contains("\"usage\"") && !(project_path.is_none() && line.contains("\"cwd\"")) {
                continue;
            }
            let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else { continue };
            if project_path.is_none() {
                if let Some(cwd) = value.get("cwd").and_then(|c| c.as_str()) {
                    project_path = Some(cwd.to_string());
                }
            }
            let Some(usage) = value.get("message").and_then(|m| m.get("usage")) else { continue };
            let timestamp = value.get("timestamp").and_then(|t| t.as_str()).unwrap_or("").to_string();
            let model = value
                .get("message")
                .and_then(|m| m.get("model"))
                .and_then(|m| m.as_str())
                .map(|s| s.to_string());
            let input = usage.get("input_tokens").and_then(|t| t.as_u64()).unwrap_or(0);
            let output = usage.get("output_tokens").and_then(|t| t.as_u64()).unwrap_or(0);
            let cc = usage.get("cache_creation_input_tokens").and_then(|t| t.as_u64()).unwrap_or(0);
            let cr = usage.get("cache_read_input_tokens").and_then(|t| t.as_u64()).unwrap_or(0);
            if input + output + cc + cr > 0 {
                out.push((
                    extract_session_id(path),
                    timestamp,
                    model,
                    input,
                    output,
                    cc,
                    cr,
                    project_path.clone(),
                ));
            }
        }
        out
    }

    #[test]
    fn test_borrowed_parse_matches_value_dom_on_real_files() {
        let adapter = ClaudeAdapter::new();
        let Ok(paths) = adapter.find_data_paths() else { return };
        let mut files = Vec::new();
        for base in &paths {
            let projects = base.join("projects");
            if projects.is_dir() {
                files.extend(collect_jsonl_files(&projects));
            }
        }
        files.sort();

        let mut compared = 0usize;
        for file in files.into_iter().take(100) {
            let new: Vec<EntryTuple> = parse_claude_jsonl(&file)
                .unwrap()
                .iter()
                .map(entry_tuple)
                .collect();
            let reference = reference_parse(&file);
            assert_eq!(new, reference, "parse mismatch on {:?}", file);
            compared += 1;
        }
        assert!(compared > 10, "expected to compare a meaningful number of real files");
    }
}

/// Borrowed view of a Claude Code JSONL line: only the fields usage
/// accounting needs, zero-copy.
#[derive(serde::Deserialize)]
struct ClaudeLine<'a> {
    #[serde(borrow)]
    cwd: Option<&'a str>,
    #[serde(borrow)]
    timestamp: Option<&'a str>,
    #[serde(borrow)]
    message: Option<ClaudeMessage<'a>>,
}

#[derive(serde::Deserialize)]
struct ClaudeMessage<'a> {
    #[serde(borrow)]
    model: Option<&'a str>,
    usage: Option<ClaudeUsage>,
}

#[derive(serde::Deserialize)]
struct ClaudeUsage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cache_creation_input_tokens: Option<u64>,
    cache_read_input_tokens: Option<u64>,
}

/// Parse a Claude JSONL file into usage entries
fn parse_claude_jsonl(path: &Path) -> Result<Vec<UsageEntry>, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let mut entries = Vec::new();
    let mut project_path: Option<String> = None;

    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }

        // Skip the full JSON parse unless the line can carry usage data or
        // the cwd still being looked for.
        if !line.contains("\"usage\"") && !(project_path.is_none() && line.contains("\"cwd\"")) {
            continue;
        }

        let Ok(value) = serde_json::from_str::<ClaudeLine>(line) else {
            continue;
        };

        // Extract cwd from the first line that has it
        if project_path.is_none() {
            if let Some(cwd) = value.cwd {
                project_path = Some(cwd.to_string());
            }
        }

        // Skip entries without usage data
        let Some(usage) = value.message.as_ref().and_then(|m| m.usage.as_ref()) else {
            continue;
        };

        let timestamp = value.timestamp.unwrap_or("").to_string();

        let model = value
            .message
            .as_ref()
            .and_then(|m| m.model)
            .map(|s| s.to_string());

        let input_tokens = usage.input_tokens.unwrap_or(0);
        let output_tokens = usage.output_tokens.unwrap_or(0);
        let cache_creation_tokens = usage.cache_creation_input_tokens.unwrap_or(0);
        let cache_read_tokens = usage.cache_read_input_tokens.unwrap_or(0);

        let total_tokens = input_tokens + output_tokens + cache_creation_tokens + cache_read_tokens;

        let session_id = extract_session_id(path);

        if total_tokens > 0 {
            entries.push(UsageEntry {
                session_id,
                timestamp,
                model,
                input_tokens,
                output_tokens,
                // Claude's usage.output_tokens already includes thinking tokens
                reasoning_tokens: 0,
                cache_creation_tokens,
                cache_read_tokens,
                total_tokens,
                project_path: project_path.clone(),
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
