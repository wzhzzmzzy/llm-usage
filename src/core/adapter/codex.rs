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
        use rayon::prelude::*;

        let mut all_files = Vec::new();
        for base_path in paths {
            all_files.extend(collect_jsonl_files(base_path));
        }
        let SessionMetaScan {
            attribution,
            forks,
            paths: thread_paths,
        } = scan_session_metas(&all_files);

        let mut parent_seqs: std::collections::HashMap<String, Vec<CumSnapshot>> =
            std::collections::HashMap::new();
        for (file, parent_uuid) in &forks {
            let Some(parent_path) = thread_paths.get(parent_uuid) else {
                continue;
            };
            parent_seqs
                .entry(parent_uuid.clone())
                .or_insert_with(|| {
                    fs::read_to_string(parent_path)
                        .map(|c| cum_snapshots(&c))
                        .unwrap_or_default()
                });
        }

        let entries = all_files
            .par_iter()
            .map(|file| {
                let fork_seq = forks.get(file).and_then(|u| parent_seqs.get(u));
                parse_codex_jsonl(file, &attribution, fork_seq.map(|v| &**v)).unwrap_or_default()
            })
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

                let model_breakdown = build_model_breakdown(
                    &day_entries.iter().map(|e| (*e).clone()).collect::<Vec<_>>(),
                );

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

                let model_breakdown = build_model_breakdown(
                    &month_entries.iter().map(|e| (*e).clone()).collect::<Vec<_>>(),
                );

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

/// Parse a Codex JSONL file into usage entries.
///
/// Subagent rollouts (source.subagent in session_meta) come in two flavors:
/// mirror streams duplicate the parent thread's token_count events with a
/// cumulative counter inherited from the parent (first event's cumulative
/// total exceeds its per-call usage) and are dropped entirely; independent
/// streams are real, disjoint API calls and are kept, attributed to the
/// spawning session via `attribution` (thread uuid -> root session id).
fn parse_codex_jsonl(
    path: &Path,
    attribution: &std::collections::HashMap<String, String>,
    fork_parent_seq: Option<&[CumSnapshot]>,
) -> Result<Vec<UsageEntry>, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;

    let mut file_uuid: Option<String> = None;
    let mut is_subagent = false;
    if let Some(first_line) = content.lines().next() {
        if !first_line.trim().is_empty() {
            if let Ok(meta) = serde_json::from_str::<serde_json::Value>(first_line) {
                if meta.get("type").and_then(|t| t.as_str()) == Some("session_meta") {
                    file_uuid = meta
                        .pointer("/payload/id")
                        .and_then(|i| i.as_str())
                        .map(|s| s.to_string());
                    is_subagent = meta.pointer("/payload/source/subagent").is_some();
                }
            }
        }
    }

    let own_session_id = extract_session_id(path);
    let session_id = if is_subagent {
        file_uuid
            .as_ref()
            .and_then(|u| attribution.get(u))
            .cloned()
            .unwrap_or_else(|| own_session_id.clone())
    } else {
        own_session_id.clone()
    };

    // A forked rollout opens with a replay of the parent thread's history;
    // skip those copied events and resume counting from the copied baseline.
    let mut skip_token_events = 0usize;
    let mut fork_baseline: Option<CumSnapshot> = None;
    if let Some(parent_seq) = fork_parent_seq {
        let own_seq = cum_snapshots(&content);
        let matched = fork_replay_prefix_len(&own_seq, parent_seq);
        if matched > 0 {
            fork_baseline = Some(own_seq[matched - 1]);
            let mut cum_seen = 0usize;
            for line in content.lines() {
                if !line.contains("token_count") {
                    continue;
                }
                let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
                    continue;
                };
                let is_token = v.get("type").and_then(|t| t.as_str()) == Some("event_msg")
                    && v.pointer("/payload/type").and_then(|t| t.as_str())
                        == Some("token_count");
                if !is_token {
                    continue;
                }
                skip_token_events += 1;
                if v.pointer("/payload/info/total_token_usage").is_some() {
                    cum_seen += 1;
                    if cum_seen == matched {
                        break;
                    }
                }
            }
        }
    }

    let mut entries = Vec::new();
    let mut current_model: Option<String> = None;
    let mut current_cwd: Option<String> = None;

    // token_count events are re-emitted with an unchanged cumulative
    // snapshot (~14% of events in real rollouts), and the counter resets to
    // a lower value after compaction/rollback. Summing `last_token_usage`
    // therefore over-counts; per-call usage is derived from cumulative
    // deltas instead. Tuple fields: (input, cached_input, output, reasoning).
    let mut prev_cum: Option<(u64, u64, u64, u64)> = None;

    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }

        // Only token_count and turn_context lines influence the result.
        if !line.contains("token_count") && !line.contains("turn_context") {
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

            if skip_token_events > 0 {
                skip_token_events -= 1;
                prev_cum = fork_baseline;
                continue;
            }

            let info = payload.get("info");
            let cum = info.and_then(|i| i.get("total_token_usage"));
            let last = info.and_then(|i| i.get("last_token_usage"));

            let field = |u: Option<&serde_json::Value>, key: &str| -> u64 {
                u.and_then(|v| v.get(key))
                    .and_then(|t| t.as_u64())
                    .unwrap_or(0)
            };
            let cum_fields = (
                field(cum, "input_tokens"),
                field(cum, "cached_input_tokens"),
                field(cum, "output_tokens"),
                field(cum, "reasoning_output_tokens"),
            );
            let last_fields = (
                field(last, "input_tokens"),
                field(last, "cached_input_tokens"),
                field(last, "output_tokens"),
                field(last, "reasoning_output_tokens"),
            );

            let cum_total = cum_fields.0 + cum_fields.2 + cum_fields.3;
            let last_total = last_fields.0 + last_fields.2 + last_fields.3;

            // Mirror-stream detection: the first decisive event of a
            // subagent file already carries the parent's cumulative total.
            if is_subagent && prev_cum.is_none() && cum.is_some() && last.is_some() && cum_total > last_total {
                return Ok(Vec::new());
            }

            let (input_tokens, cached_input_tokens, output_tokens, reasoning_output_tokens) =
                match (prev_cum, cum) {
                    (None, None) | (Some(_), None) => {
                        // No cumulative snapshot: nothing to diff against.
                        if last.is_none() {
                            continue;
                        }
                        last_fields
                    }
                    (None, Some(_)) => {
                        // First event: `last` is the call's usage (equals
                        // `cum` for a fresh counter).
                        prev_cum = Some(cum_fields);
                        if last.is_some() {
                            last_fields
                        } else {
                            cum_fields
                        }
                    }
                    (Some(p), Some(_)) => {
                        let p_total = p.0 + p.2 + p.3;
                        if cum_total == p_total {
                            // Duplicate re-emission of the same snapshot.
                            continue;
                        }
                        prev_cum = Some(cum_fields);
                        if cum_total < p_total {
                            // Counter reset (compaction/rollback): `last`
                            // holds the call that produced the new snapshot.
                            if last.is_some() {
                                last_fields
                            } else {
                                cum_fields
                            }
                        } else {
                            (
                                cum_fields.0.saturating_sub(p.0),
                                cum_fields.1.saturating_sub(p.1),
                                cum_fields.2.saturating_sub(p.2),
                                cum_fields.3.saturating_sub(p.3),
                            )
                        }
                    }
                };

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

            entries.push(UsageEntry {
                session_id: session_id.clone(),
                timestamp,
                model: Some(model),
                input_tokens: input_tokens.saturating_sub(cached_input_tokens),
                output_tokens,
                reasoning_tokens: reasoning_output_tokens,
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

/// (input, cached_input, output, reasoning) from one token_count snapshot.
type CumSnapshot = (u64, u64, u64, u64);

/// Pre-scan of every rollout's session_meta line.
struct SessionMetaScan {
    /// Subagent thread uuid -> session id of the root session that spawned it.
    attribution: std::collections::HashMap<String, String>,
    /// Fork file path -> forked_from_id.
    forks: std::collections::HashMap<PathBuf, String>,
    /// Thread uuid -> rollout path (fork parents are resolved through this).
    paths: std::collections::HashMap<String, PathBuf>,
}

/// Read every rollout's first line once: subagent uuids are mapped to the
/// root session that (transitively) spawned them so independent subagent
/// usage folds into the spawning session; forks are indexed for replay
/// trimming. Threads with unresolvable parent chains (parent file missing,
/// cycle, or a variant like {"subagent":{"other":..}} that records no
/// parent_thread_id) keep their own session id.
fn scan_session_metas(files: &[PathBuf]) -> SessionMetaScan {
    use std::collections::{HashMap, HashSet};
    use std::io::BufRead;

    let mut threads: HashMap<String, (bool, Option<String>)> = HashMap::new();
    let mut root_sessions: HashMap<String, String> = HashMap::new();
    let mut forks: HashMap<PathBuf, String> = HashMap::new();
    let mut paths: HashMap<String, PathBuf> = HashMap::new();

    for path in files {
        let Some(first_line) = (|| {
            let file = fs::File::open(path).ok()?;
            let mut line = String::new();
            std::io::BufReader::new(file).read_line(&mut line).ok()?;
            Some(line)
        })() else {
            continue;
        };
        let Ok(meta) = serde_json::from_str::<serde_json::Value>(&first_line) else {
            continue;
        };
        if meta.get("type").and_then(|t| t.as_str()) != Some("session_meta") {
            continue;
        }
        let payload = meta.get("payload").cloned().unwrap_or_default();
        let Some(uuid) = payload
            .get("id")
            .and_then(|i| i.as_str())
            .map(|s| s.to_string())
        else {
            continue;
        };
        if let Some(forked_from) = payload.get("forked_from_id").and_then(|f| f.as_str()) {
            forks.insert(path.clone(), forked_from.to_string());
        }
        paths.entry(uuid.clone()).or_insert_with(|| path.clone());

        let subagent = payload
            .get("source")
            .and_then(|s| s.get("subagent"));
        threads.entry(uuid.clone()).or_insert_with(|| {
            let parent = subagent
                .and_then(|s| s.get("thread_spawn"))
                .and_then(|t| t.get("parent_thread_id"))
                .and_then(|p| p.as_str())
                .map(|s| s.to_string());
            (subagent.is_some(), parent)
        });
        if subagent.is_none() {
            root_sessions
                .entry(uuid)
                .or_insert_with(|| extract_session_id(path));
        }
    }

    let mut attribution = HashMap::new();
    for (uuid, (is_subagent, parent)) in &threads {
        if !is_subagent {
            continue;
        }
        let mut visited = HashSet::new();
        let mut current = parent.clone();
        let root = loop {
            let Some(candidate) = current else { break None };
            if !visited.insert(candidate.clone()) {
                break None;
            }
            match threads.get(&candidate) {
                Some((false, _)) => break root_sessions.get(&candidate).cloned(),
                Some((true, next)) => current = next.clone(),
                None => break None,
            }
        };
        if let Some(root_session_id) = root {
            attribution.insert(uuid.clone(), root_session_id);
        }
    }

    SessionMetaScan {
        attribution,
        forks,
        paths,
    }
}

/// Cumulative snapshots of every token_count event in a rollout, in order.
fn cum_snapshots(content: &str) -> Vec<CumSnapshot> {
    content
        .lines()
        .filter(|line| line.contains("token_count"))
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|v| {
            v.get("type").and_then(|t| t.as_str()) == Some("event_msg")
                && v.pointer("/payload/type").and_then(|t| t.as_str()) == Some("token_count")
        })
        .filter_map(|v| {
            let tt = v.pointer("/payload/info/total_token_usage")?;
            Some((
                tt.get("input_tokens")?.as_u64()?,
                tt.get("cached_input_tokens")?.as_u64()?,
                tt.get("output_tokens")?.as_u64()?,
                tt.get("reasoning_output_tokens")?.as_u64()?,
            ))
        })
        .collect()
}

/// A forked rollout opens by replaying the parent thread's history: its
/// snapshot sequence is a verbatim contiguous run (prefix in current Codex
/// versions, an infix at the fork point in older ones) of the parent's
/// sequence. Returns how many leading fork events are replay.
fn fork_replay_prefix_len(fork: &[CumSnapshot], parent: &[CumSnapshot]) -> usize {
    let Some(first) = fork.first() else { return 0 };
    let mut best = 0;
    for start in 0..parent.len() {
        if &parent[start] != first {
            continue;
        }
        let mut k = 0;
        while k < fork.len() && start + k < parent.len() && fork[k] == parent[start + k] {
            k += 1;
        }
        best = best.max(k);
        if best == fork.len() {
            break;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIRROR_SUBAGENT_JSONL: &str = r#"{"type":"session_meta","payload":{"id":"sub-mirror","source":{"subagent":{"thread_spawn":{"parent_thread_id":"parent-123","depth":1,"agent_nickname":"Anscombe","agent_role":"explorer"}}},"originator":"Codex Desktop","timestamp":"2026-04-28T03:44:37.728Z","cwd":"/tmp"}}
{"type":"turn_context","payload":{"model":"gpt-5","cwd":"/tmp"}}
{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":3650000,"cached_input_tokens":3000000,"output_tokens":7270,"reasoning_output_tokens":0,"total_tokens":3657270},"last_token_usage":{"input_tokens":170000,"cached_input_tokens":140000,"output_tokens":6243,"reasoning_output_tokens":0,"total_tokens":176243}},"model":"gpt-5"},"timestamp":"2026-04-28T03:44:57.491Z"}
"#;

    const INDEPENDENT_SUBAGENT_JSONL: &str = r#"{"type":"session_meta","payload":{"id":"sub-indep","source":{"subagent":{"thread_spawn":{"parent_thread_id":"parent-123","depth":1,"agent_nickname":"Anscombe","agent_role":"explorer"}}},"originator":"Codex Desktop","timestamp":"2026-04-28T03:44:37.728Z","cwd":"/tmp"}}
{"type":"turn_context","payload":{"model":"gpt-5","cwd":"/tmp"}}
{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":100,"cached_input_tokens":30,"output_tokens":50,"reasoning_output_tokens":0,"total_tokens":150},"last_token_usage":{"input_tokens":100,"cached_input_tokens":30,"output_tokens":50,"reasoning_output_tokens":0,"total_tokens":150}},"model":"gpt-5"},"timestamp":"2026-04-28T03:44:57.491Z"}
"#;

    const GUARDIAN_SUBAGENT_JSONL: &str = r#"{"type":"session_meta","payload":{"id":"sub-guardian","source":{"subagent":{"other":"guardian"}},"originator":"Codex Desktop","timestamp":"2026-04-28T03:44:37.728Z","cwd":"/tmp"}}
{"type":"turn_context","payload":{"model":"gpt-5","cwd":"/tmp"}}
{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":100,"cached_input_tokens":30,"output_tokens":50,"reasoning_output_tokens":0,"total_tokens":150},"last_token_usage":{"input_tokens":100,"cached_input_tokens":30,"output_tokens":50,"reasoning_output_tokens":0,"total_tokens":150}},"model":"gpt-5"},"timestamp":"2026-04-28T03:44:57.491Z"}
"#;

    const NORMAL_JSONL: &str = r#"{"type":"session_meta","payload":{"id":"abc","source":"vscode","originator":"Codex Desktop","timestamp":"2026-04-28T03:44:37.728Z","cwd":"/tmp"}}
{"type":"turn_context","payload":{"model":"gpt-5","cwd":"/tmp"}}
{"type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":100,"output_tokens":50,"cached_input_tokens":30}},"model":"gpt-5"},"timestamp":"2026-04-28T03:44:57.491Z"}
"#;

    const NO_META_JSONL: &str = r#"{"type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":100,"output_tokens":50,"cached_input_tokens":30}},"model":"gpt-5"},"timestamp":"2026-04-28T03:44:57.491Z"}
"#;

    // Real-rollout pattern: the same usage snapshot is emitted twice with
    // an unchanged cumulative total.
    const DUPLICATED_SNAPSHOT_JSONL: &str = r#"{"type":"session_meta","payload":{"id":"abc","source":"vscode","timestamp":"2026-04-28T03:44:37.728Z","cwd":"/tmp"}}
{"type":"turn_context","payload":{"model":"gpt-5","cwd":"/tmp"}}
{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":14387,"cached_input_tokens":9600,"output_tokens":294,"reasoning_output_tokens":0,"total_tokens":14681},"last_token_usage":{"input_tokens":14387,"cached_input_tokens":9600,"output_tokens":294,"reasoning_output_tokens":0,"total_tokens":14681}},"timestamp":"2026-04-28T03:44:49.041Z"}}
{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":14387,"cached_input_tokens":9600,"output_tokens":294,"reasoning_output_tokens":0,"total_tokens":14681},"last_token_usage":{"input_tokens":14387,"cached_input_tokens":9600,"output_tokens":294,"reasoning_output_tokens":0,"total_tokens":14681}},"timestamp":"2026-04-28T03:45:44.339Z"}}
"#;

    // last_token_usage of the second event disagrees with the cumulative
    // delta on purpose: the delta must win.
    const ADVANCING_SNAPSHOT_JSONL: &str = r#"{"type":"session_meta","payload":{"id":"abc","source":"vscode","timestamp":"2026-04-28T03:44:37.728Z","cwd":"/tmp"}}
{"type":"turn_context","payload":{"model":"gpt-5","cwd":"/tmp"}}
{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":100,"cached_input_tokens":30,"output_tokens":50,"reasoning_output_tokens":0,"total_tokens":150},"last_token_usage":{"input_tokens":100,"cached_input_tokens":30,"output_tokens":50,"reasoning_output_tokens":0,"total_tokens":150}},"timestamp":"2026-04-28T03:44:49.041Z"}}
{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":250,"cached_input_tokens":80,"output_tokens":100,"reasoning_output_tokens":10,"total_tokens":360},"last_token_usage":{"input_tokens":160,"cached_input_tokens":55,"output_tokens":50,"reasoning_output_tokens":10,"total_tokens":220}},"timestamp":"2026-04-28T03:45:44.339Z"}}
"#;

    // Compaction/rollback drops the cumulative counter below its previous
    // value; the event's last_token_usage is the usage of the compacting call.
    const RESET_SNAPSHOT_JSONL: &str = r#"{"type":"session_meta","payload":{"id":"abc","source":"vscode","timestamp":"2026-04-28T03:44:37.728Z","cwd":"/tmp"}}
{"type":"turn_context","payload":{"model":"gpt-5","cwd":"/tmp"}}
{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":1000,"cached_input_tokens":300,"output_tokens":200,"reasoning_output_tokens":0,"total_tokens":1200},"last_token_usage":{"input_tokens":1000,"cached_input_tokens":300,"output_tokens":200,"reasoning_output_tokens":0,"total_tokens":1200}},"timestamp":"2026-04-28T03:44:49.041Z"}}
{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":40,"cached_input_tokens":10,"output_tokens":20,"reasoning_output_tokens":5,"total_tokens":65},"last_token_usage":{"input_tokens":40,"cached_input_tokens":10,"output_tokens":20,"reasoning_output_tokens":5,"total_tokens":65}},"timestamp":"2026-04-28T03:50:00.000Z"}}
"#;

    fn write_temp_jsonl(name: &str, content: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("llm-usage-tests");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_mirror_subagent_dropped() {
        let path = write_temp_jsonl("mirror.jsonl", MIRROR_SUBAGENT_JSONL);
        let entries = parse_codex_jsonl(&path, &Default::default(), None).unwrap();
        assert!(
            entries.is_empty(),
            "mirror-stream subagent files duplicate the parent and must be dropped"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_independent_subagent_attributed_to_parent() {
        let path = write_temp_jsonl("indep.jsonl", INDEPENDENT_SUBAGENT_JSONL);
        let attribution = std::collections::HashMap::from([(
            "sub-indep".to_string(),
            "rollout-parent".to_string(),
        )]);
        let entries = parse_codex_jsonl(&path, &attribution, None).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].session_id, "rollout-parent");
        assert_eq!(entries[0].total_tokens, 150);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_independent_subagent_without_attribution_keeps_own_id() {
        let path = write_temp_jsonl("indep-own.jsonl", INDEPENDENT_SUBAGENT_JSONL);
        let entries = parse_codex_jsonl(&path, &Default::default(), None).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].session_id, "indep-own");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_guardian_variant_counted() {
        let path = write_temp_jsonl("guardian.jsonl", GUARDIAN_SUBAGENT_JSONL);
        let entries = parse_codex_jsonl(&path, &Default::default(), None).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].total_tokens, 150);
        let _ = std::fs::remove_file(&path);
    }

    const FORK_REPLAY_JSONL: &str = r#"{"type":"session_meta","payload":{"id":"fork-1","forked_from_id":"parent-1","source":"vscode","timestamp":"2026-07-23T09:38:17.667Z","cwd":"/tmp"}}
{"type":"turn_context","payload":{"model":"gpt-5","cwd":"/tmp"}}
{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":100,"cached_input_tokens":30,"output_tokens":50,"reasoning_output_tokens":0,"total_tokens":150},"last_token_usage":{"input_tokens":100,"cached_input_tokens":30,"output_tokens":50,"reasoning_output_tokens":0,"total_tokens":150}}},"timestamp":"2026-07-23T09:38:18.010Z"}
{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":250,"cached_input_tokens":80,"output_tokens":100,"reasoning_output_tokens":10,"total_tokens":360},"last_token_usage":{"input_tokens":150,"cached_input_tokens":50,"output_tokens":50,"reasoning_output_tokens":10,"total_tokens":210}}},"timestamp":"2026-07-23T09:38:18.011Z"}
"#;

    const FORK_NEW_CALL_JSONL: &str = r#"{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":330,"cached_input_tokens":100,"output_tokens":130,"reasoning_output_tokens":15,"total_tokens":475},"last_token_usage":{"input_tokens":80,"cached_input_tokens":20,"output_tokens":30,"reasoning_output_tokens":5,"total_tokens":115}}},"timestamp":"2026-07-23T10:00:00.000Z"}
"#;

    #[test]
    fn test_fork_pure_replay_counts_nothing() {
        let parent_seq: Vec<CumSnapshot> = vec![(100, 30, 50, 0), (250, 80, 100, 10)];
        let path = write_temp_jsonl("fork-pure.jsonl", FORK_REPLAY_JSONL);
        let entries = parse_codex_jsonl(&path, &Default::default(), Some(&parent_seq)).unwrap();
        assert!(
            entries.is_empty(),
            "a fork that only replays the parent history adds no usage"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_fork_counts_only_post_fork_calls() {
        let fork_with_new = format!("{}{}", FORK_REPLAY_JSONL, FORK_NEW_CALL_JSONL);
        let parent_seq: Vec<CumSnapshot> = vec![(100, 30, 50, 0), (250, 80, 100, 10)];
        let path = write_temp_jsonl("fork-new.jsonl", &fork_with_new);
        let entries = parse_codex_jsonl(&path, &Default::default(), Some(&parent_seq)).unwrap();
        assert_eq!(entries.len(), 1, "only the post-fork call counts");
        assert_eq!(entries[0].input_tokens, 60);
        assert_eq!(entries[0].cache_read_tokens, 20);
        assert_eq!(entries[0].output_tokens, 30);
        assert_eq!(entries[0].total_tokens, 115);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_fork_missing_parent_counts_normally() {
        let path = write_temp_jsonl("fork-orphan.jsonl", FORK_REPLAY_JSONL);
        let entries = parse_codex_jsonl(&path, &Default::default(), None).unwrap();
        assert_eq!(entries.len(), 2);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_fork_replay_prefix_len_infix() {
        let parent: Vec<CumSnapshot> = vec![(10, 0, 0, 0), (20, 0, 0, 0), (30, 0, 0, 0), (40, 0, 0, 0)];
        let fork: Vec<CumSnapshot> = vec![(30, 0, 0, 0), (40, 0, 0, 0), (55, 0, 0, 0)];
        assert_eq!(
            fork_replay_prefix_len(&fork, &parent),
            2,
            "older Codex versions start the fork file at the fork point"
        );
        assert_eq!(fork_replay_prefix_len(&parent, &fork), 0);
        assert_eq!(fork_replay_prefix_len(&[], &parent), 0);
    }

    #[test]
    fn test_attribution_resolves_transitively() {        let root = write_temp_jsonl(
            "root.jsonl",
            r#"{"type":"session_meta","payload":{"id":"root","source":"vscode","timestamp":"t","cwd":"/tmp"}}
"#,
        );
        let sub1 = write_temp_jsonl(
            "sub1.jsonl",
            r#"{"type":"session_meta","payload":{"id":"sub1","source":{"subagent":{"thread_spawn":{"parent_thread_id":"root","depth":1}}},"timestamp":"t","cwd":"/tmp"}}
"#,
        );
        let sub2 = write_temp_jsonl(
            "sub2.jsonl",
            r#"{"type":"session_meta","payload":{"id":"sub2","source":{"subagent":{"thread_spawn":{"parent_thread_id":"sub1","depth":2}}},"timestamp":"t","cwd":"/tmp"}}
"#,
        );
        let orphan = write_temp_jsonl(
            "orphan.jsonl",
            r#"{"type":"session_meta","payload":{"id":"orphan","source":{"subagent":{"thread_spawn":{"parent_thread_id":"missing","depth":1}}},"timestamp":"t","cwd":"/tmp"}}
"#,
        );
        let files = vec![root.clone(), sub1.clone(), sub2.clone(), orphan.clone()];
        let map = scan_session_metas(&files).attribution;
        assert_eq!(map.get("sub1").map(|s| s.as_str()), Some("root"));
        assert_eq!(
            map.get("sub2").map(|s| s.as_str()),
            Some("root"),
            "depth-2 subagent should resolve to the root session"
        );
        assert!(
            !map.contains_key("orphan"),
            "unresolvable parent chain keeps the file's own session id"
        );
        for f in files {
            let _ = std::fs::remove_file(&f);
        }
    }

    #[test]
    fn test_normal_session_parsed() {
        let path = write_temp_jsonl("normal.jsonl", NORMAL_JSONL);
        let entries = parse_codex_jsonl(&path, &Default::default(), None).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].cache_read_tokens, 30);
        assert_eq!(entries[0].input_tokens, 70);
        assert_eq!(entries[0].output_tokens, 50);
        assert_eq!(entries[0].total_tokens, 150);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_no_session_meta_still_parsed() {
        let path = write_temp_jsonl("no_meta.jsonl", NO_META_JSONL);
        let entries = parse_codex_jsonl(&path, &Default::default(), None).unwrap();
        assert_eq!(
            entries.len(),
            1,
            "files without session_meta should still be parsed"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_duplicated_snapshot_counted_once() {
        let path = write_temp_jsonl("dup.jsonl", DUPLICATED_SNAPSHOT_JSONL);
        let entries = parse_codex_jsonl(&path, &Default::default(), None).unwrap();
        assert_eq!(
            entries.len(),
            1,
            "re-emitted snapshot with unchanged cumulative total must be skipped"
        );
        assert_eq!(entries[0].total_tokens, 14681);
        assert_eq!(entries[0].input_tokens, 4787);
        assert_eq!(entries[0].cache_read_tokens, 9600);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_cumulative_delta_wins_over_last() {
        let path = write_temp_jsonl("delta.jsonl", ADVANCING_SNAPSHOT_JSONL);
        let entries = parse_codex_jsonl(&path, &Default::default(), None).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[1].input_tokens, 100);
        assert_eq!(entries[1].cache_read_tokens, 50);
        assert_eq!(entries[1].output_tokens, 50);
        assert_eq!(entries[1].total_tokens, 210);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_counter_reset_uses_last_token_usage() {
        let path = write_temp_jsonl("reset.jsonl", RESET_SNAPSHOT_JSONL);
        let entries = parse_codex_jsonl(&path, &Default::default(), None).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[1].input_tokens, 30);
        assert_eq!(entries[1].cache_read_tokens, 10);
        assert_eq!(entries[1].output_tokens, 20);
        assert_eq!(entries[1].total_tokens, 65);
        let _ = std::fs::remove_file(&path);
    }
}
