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
        use std::collections::HashMap;

        let mut all_files = Vec::new();
        for base_path in paths {
            all_files.extend(collect_jsonl_files(base_path));
        }

        // Single read per rollout: entries, session_meta and cumulative
        // snapshots all come from one in-memory copy. Fork files keep their
        // content and defer entry extraction until every rollout's snapshot
        // sequence is known.
        let mut scans: Vec<FileScan> = all_files.par_iter().map(|f| FileScan::read(f)).collect();

        let mut threads: HashMap<String, (bool, Option<String>)> = HashMap::new();
        let mut root_sessions: HashMap<String, String> = HashMap::new();
        let mut cum_by_uuid: HashMap<String, &Vec<CumSnapshot>> = HashMap::new();
        for scan in &scans {
            let Some(meta) = &scan.meta else { continue };
            cum_by_uuid.entry(meta.uuid.clone()).or_insert(&scan.cum_seq);
            threads
                .entry(meta.uuid.clone())
                .or_insert_with(|| (meta.is_subagent, meta.parent_thread_id.clone()));
            if !meta.is_subagent {
                root_sessions
                    .entry(meta.uuid.clone())
                    .or_insert_with(|| extract_session_id(&scan.path));
            }
        }
        let attribution = resolve_attribution(&threads, &root_sessions);

        // Fork rollouts open with a replay of the parent thread's history;
        // parse them only now that every parent's snapshot sequence is held
        // in memory, so no file is ever read twice.
        let mut fork_entries: HashMap<PathBuf, Vec<UsageEntry>> = scans
            .par_iter()
            .filter_map(|scan| {
                let content = scan.content.as_ref()?;
                let forked_from = scan.meta.as_ref()?.forked_from.as_ref()?;
                let parent_seq = cum_by_uuid.get(forked_from).map(|v| v.as_slice());
                let entries =
                    parse_codex_jsonl_content(&scan.path, content, &attribution, parent_seq, None)
                        .unwrap_or_default();
                Some((scan.path.clone(), entries))
            })
            .collect();

        let mut all_entries = Vec::new();
        for scan in scans {
            let entries = if scan.content.is_some() {
                fork_entries.remove(&scan.path).unwrap_or_default()
            } else {
                let mut entries = scan.entries;
                // Independent subagent usage folds into the spawning session.
                if let Some(meta) = &scan.meta {
                    if meta.is_subagent {
                        if let Some(root) = attribution.get(&meta.uuid) {
                            for entry in &mut entries {
                                entry.session_id = root.clone();
                            }
                        }
                    }
                }
                entries
            };
            all_entries.extend(entries);
        }

        Ok(all_entries)
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

/// Borrowed view of a rollout line: only the fields token accounting needs.
/// Unknown fields are skipped without allocating; strings are zero-copy
/// slices of the line (no escapes occur in these fields in practice).
/// serde collapses JSON null to None, so a literal `"total_token_usage": null`
/// counts as absent; verified unreachable in real rollout data.
#[derive(serde::Deserialize)]
struct CodexLine<'a> {
    #[serde(rename = "type", borrow)]
    kind: Option<&'a str>,
    #[serde(borrow)]
    payload: Option<CodexPayload<'a>>,
    #[serde(borrow)]
    timestamp: Option<&'a str>,
}

#[derive(serde::Deserialize)]
struct CodexPayload<'a> {
    #[serde(rename = "type", borrow)]
    kind: Option<&'a str>,
    #[serde(borrow)]
    model: Option<&'a str>,
    #[serde(borrow)]
    model_name: Option<&'a str>,
    #[serde(borrow)]
    cwd: Option<&'a str>,
    info: Option<CodexUsageInfo>,
}

#[derive(serde::Deserialize)]
struct CodexUsageInfo {
    total_token_usage: Option<UsageQuad>,
    last_token_usage: Option<UsageQuad>,
}

#[derive(serde::Deserialize)]
struct UsageQuad {
    input_tokens: Option<u64>,
    cached_input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    reasoning_output_tokens: Option<u64>,
}

impl UsageQuad {
    fn fields(q: Option<&UsageQuad>) -> CumSnapshot {
        match q {
            Some(q) => (
                q.input_tokens.unwrap_or(0),
                q.cached_input_tokens.unwrap_or(0),
                q.output_tokens.unwrap_or(0),
                q.reasoning_output_tokens.unwrap_or(0),
            ),
            None => (0, 0, 0, 0),
        }
    }

    /// Fork-replay matching requires a complete snapshot; partially-filled
    /// usage objects must not be recorded.
    fn complete(q: &UsageQuad) -> Option<CumSnapshot> {
        Some((
            q.input_tokens?,
            q.cached_input_tokens?,
            q.output_tokens?,
            q.reasoning_output_tokens?,
        ))
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
#[cfg(test)]
fn parse_codex_jsonl(
    path: &Path,
    attribution: &std::collections::HashMap<String, String>,
    fork_parent_seq: Option<&[CumSnapshot]>,
) -> Result<Vec<UsageEntry>, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    parse_codex_jsonl_content(path, &content, attribution, fork_parent_seq, None)
}

/// `cum_out`, when given, collects every cumulative token_count snapshot in
/// line order (duplicates included) so fork rollouts can be replay-matched
/// against this file without re-reading it.
fn parse_codex_jsonl_content(
    path: &Path,
    content: &str,
    attribution: &std::collections::HashMap<String, String>,
    fork_parent_seq: Option<&[CumSnapshot]>,
    mut cum_out: Option<&mut Vec<CumSnapshot>>,
) -> Result<Vec<UsageEntry>, Box<dyn std::error::Error>> {
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
        let own_seq = cum_snapshots(content);
        let matched = fork_replay_prefix_len(&own_seq, parent_seq);
        if matched > 0 {
            fork_baseline = Some(own_seq[matched - 1]);
            let mut cum_seen = 0usize;
            for line in content.lines() {
                if !line.contains("token_count") {
                    continue;
                }
                let Ok(v) = serde_json::from_str::<CodexLine>(line) else {
                    continue;
                };
                let is_token = v.kind == Some("event_msg")
                    && v.payload.as_ref().and_then(|p| p.kind) == Some("token_count");
                if !is_token {
                    continue;
                }
                skip_token_events += 1;
                let has_total = v
                    .payload
                    .as_ref()
                    .and_then(|p| p.info.as_ref())
                    .and_then(|i| i.total_token_usage.as_ref())
                    .is_some();
                if has_total {
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
    let mut is_mirror = false;

    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }

        // Only token_count and turn_context lines influence the result.
        if !line.contains("token_count") && !line.contains("turn_context") {
            continue;
        }

        let Ok(value) = serde_json::from_str::<CodexLine>(line) else {
            continue;
        };

        // Check for turn_context to get model and cwd
        if value.kind == Some("turn_context") {
            if let Some(payload) = &value.payload {
                if let Some(model) = payload.model {
                    current_model = Some(model.to_string());
                }
                if let Some(cwd) = payload.cwd {
                    current_cwd = Some(cwd.to_string());
                }
            }
            continue;
        }

        // Check for event_msg with token_count
        if value.kind == Some("event_msg") {
            let Some(payload) = &value.payload else {
                continue;
            };

            if payload.kind != Some("token_count") {
                continue;
            }

            if let Some(out) = cum_out.as_deref_mut() {
                if let Some(snap) = payload
                    .info
                    .as_ref()
                    .and_then(|i| i.total_token_usage.as_ref())
                    .and_then(UsageQuad::complete)
                {
                    out.push(snap);
                }
            }

            if is_mirror {
                continue;
            }

            if skip_token_events > 0 {
                skip_token_events -= 1;
                prev_cum = fork_baseline;
                continue;
            }

            let cum = payload
                .info
                .as_ref()
                .and_then(|i| i.total_token_usage.as_ref());
            let last = payload
                .info
                .as_ref()
                .and_then(|i| i.last_token_usage.as_ref());

            let cum_fields = UsageQuad::fields(cum);
            let last_fields = UsageQuad::fields(last);

            let cum_total = cum_fields.0 + cum_fields.2 + cum_fields.3;
            let last_total = last_fields.0 + last_fields.2 + last_fields.3;

            // Mirror-stream detection: the first decisive event of a
            // subagent file already carries the parent's cumulative total.
            // Entries are dropped, but scanning continues so cum_out still
            // records this file's full snapshot sequence for fork children.
            if is_subagent && prev_cum.is_none() && cum.is_some() && last.is_some() && cum_total > last_total {
                entries.clear();
                is_mirror = true;
                continue;
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
                .model
                .or(payload.model_name)
                .map(|s| s.to_string())
                .or_else(|| current_model.clone())
                .unwrap_or_else(|| "gpt-5".to_string());

            let timestamp = value.timestamp.unwrap_or("").to_string();

            entries.push(UsageEntry {
                session_id: session_id.clone(),
                timestamp,
                model: Some(model),
                input_tokens: input_tokens.saturating_sub(cached_input_tokens),
                output_tokens,
                reasoning_tokens: reasoning_output_tokens,
                cache_creation_tokens: 0,
                cache_creation_1h_tokens: 0,
                cost_usd: None,
                is_fast: false,
                cost: 0.0,
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

struct MetaScan {
    uuid: String,
    is_subagent: bool,
    parent_thread_id: Option<String>,
    forked_from: Option<String>,
}

/// One rollout's share of the single-read pass over the sessions directory.
struct FileScan {
    path: PathBuf,
    meta: Option<MetaScan>,
    entries: Vec<UsageEntry>,
    cum_seq: Vec<CumSnapshot>,
    /// Kept only for fork files, which are parsed once parents are known.
    content: Option<String>,
}

impl FileScan {
    fn read(path: &Path) -> Self {
        let mut scan = FileScan {
            path: path.to_path_buf(),
            meta: None,
            entries: Vec::new(),
            cum_seq: Vec::new(),
            content: None,
        };
        let Ok(content) = fs::read_to_string(path) else {
            return scan;
        };
        scan.meta = scan_meta_line(&content);
        if scan.meta.as_ref().is_some_and(|m| m.forked_from.is_some()) {
            // Fork: defer entry extraction until the parent's snapshot
            // sequence is available; keep the content so the file is never
            // re-read.
            scan.cum_seq = cum_snapshots(&content);
            scan.content = Some(content);
            return scan;
        }
        let mut cum_seq = Vec::new();
        scan.entries =
            parse_codex_jsonl_content(path, &content, &std::collections::HashMap::new(), None, Some(&mut cum_seq))
                .unwrap_or_default();
        scan.cum_seq = cum_seq;
        scan
    }
}

fn scan_meta_line(content: &str) -> Option<MetaScan> {
    let first_line = content.lines().next()?;
    if first_line.trim().is_empty() {
        return None;
    }
    let meta = serde_json::from_str::<serde_json::Value>(first_line).ok()?;
    if meta.get("type").and_then(|t| t.as_str()) != Some("session_meta") {
        return None;
    }
    let payload = meta.get("payload")?;
    let uuid = payload.get("id").and_then(|i| i.as_str())?.to_string();
    let forked_from = payload
        .get("forked_from_id")
        .and_then(|f| f.as_str())
        .map(|s| s.to_string());
    let subagent = payload.get("source").and_then(|s| s.get("subagent"));
    let parent_thread_id = subagent
        .and_then(|s| s.get("thread_spawn"))
        .and_then(|t| t.get("parent_thread_id"))
        .and_then(|p| p.as_str())
        .map(|s| s.to_string());
    Some(MetaScan {
        uuid,
        is_subagent: subagent.is_some(),
        parent_thread_id,
        forked_from,
    })
}

/// Map each subagent thread uuid to the root session that (transitively)
/// spawned it, so independent subagent usage folds into the spawning session.
/// Unresolvable parent chains (missing parent, cycle) keep the file's own id.
fn resolve_attribution(
    threads: &std::collections::HashMap<String, (bool, Option<String>)>,
    root_sessions: &std::collections::HashMap<String, String>,
) -> std::collections::HashMap<String, String> {
    use std::collections::{HashMap, HashSet};

    let mut attribution = HashMap::new();
    for (uuid, (is_subagent, parent)) in threads {
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
    attribution
}

/// Pre-scan of every rollout's session_meta line.
#[cfg(test)]
#[allow(dead_code)]
struct SessionMetaScan {
    /// Subagent thread uuid -> session id of the root session that spawned it.
    attribution: std::collections::HashMap<String, String>,
    /// Fork file path -> forked_from_id.
    forks: std::collections::HashMap<PathBuf, String>,
    /// Thread uuid -> rollout path (fork parents are resolved through this).
    paths: std::collections::HashMap<String, PathBuf>,
}

#[cfg(test)]
fn scan_session_metas(files: &[PathBuf]) -> SessionMetaScan {
    use std::collections::HashMap;

    let scans: Vec<FileScan> = files.iter().map(|f| FileScan::read(f)).collect();

    let mut threads: HashMap<String, (bool, Option<String>)> = HashMap::new();
    let mut root_sessions: HashMap<String, String> = HashMap::new();
    let mut forks: HashMap<PathBuf, String> = HashMap::new();
    let mut paths: HashMap<String, PathBuf> = HashMap::new();

    for scan in &scans {
        let Some(meta) = &scan.meta else { continue };
        if let Some(forked_from) = &meta.forked_from {
            forks.insert(scan.path.clone(), forked_from.clone());
        }
        paths.entry(meta.uuid.clone()).or_insert_with(|| scan.path.clone());
        threads
            .entry(meta.uuid.clone())
            .or_insert_with(|| (meta.is_subagent, meta.parent_thread_id.clone()));
        if !meta.is_subagent {
            root_sessions
                .entry(meta.uuid.clone())
                .or_insert_with(|| extract_session_id(&scan.path));
        }
    }

    SessionMetaScan {
        attribution: resolve_attribution(&threads, &root_sessions),
        forks,
        paths,
    }
}

/// Cumulative snapshots of every token_count event in a rollout, in order.
fn cum_snapshots(content: &str) -> Vec<CumSnapshot> {
    content
        .lines()
        .filter(|line| line.contains("token_count"))
        .filter_map(|line| serde_json::from_str::<CodexLine>(line).ok())
        .filter(|v| {
            v.kind == Some("event_msg")
                && v.payload.as_ref().and_then(|p| p.kind) == Some("token_count")
        })
        .filter_map(|v| {
            v.payload
                .and_then(|p| p.info)
                .and_then(|i| i.total_token_usage)
                .and_then(|q| UsageQuad::complete(&q))
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

    const TURN_CONTEXT_THEN_EVENT_JSONL: &str = r#"{"type":"session_meta","payload":{"id":"abc","source":"vscode","timestamp":"t","cwd":"/old"}}
{"type":"turn_context","payload":{"model":"gpt-5.5","cwd":"/new/project"}}
{"type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":100,"output_tokens":50,"cached_input_tokens":30}},"model":"gpt-5"},"timestamp":"2026-04-28T03:44:57.491Z"}
"#;

    #[test]
    fn test_turn_context_supplies_cwd_when_event_lacks_it() {
        let path = write_temp_jsonl("turnctx.jsonl", TURN_CONTEXT_THEN_EVENT_JSONL);
        let entries = parse_codex_jsonl(&path, &Default::default(), None).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].model.as_deref(), Some("gpt-5"));
        assert_eq!(entries[0].project_path.as_deref(), Some("/new/project"));
        let _ = std::fs::remove_file(&path);
    }

    const TURN_CONTEXT_MODEL_FALLBACK_JSONL: &str = r#"{"type":"turn_context","payload":{"model":"gpt-5.5","cwd":"/p"}}
{"type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":100,"output_tokens":50,"cached_input_tokens":30}}},"timestamp":"t1"}
"#;

    #[test]
    fn test_model_fallback_chain() {
        let named = r#"{"type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":100,"output_tokens":50}},"model_name":"gpt-5.1"},"timestamp":"t1"}
"#;
        let path = write_temp_jsonl("modelname.jsonl", named);
        let entries = parse_codex_jsonl(&path, &Default::default(), None).unwrap();
        assert_eq!(entries[0].model.as_deref(), Some("gpt-5.1"));
        let _ = std::fs::remove_file(&path);

        let path = write_temp_jsonl("ctxmodel.jsonl", TURN_CONTEXT_MODEL_FALLBACK_JSONL);
        let entries = parse_codex_jsonl(&path, &Default::default(), None).unwrap();
        assert_eq!(entries[0].model.as_deref(), Some("gpt-5.5"));
        let _ = std::fs::remove_file(&path);

        let bare = r#"{"type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":100,"output_tokens":50}}},"timestamp":"t1"}
"#;
        let path = write_temp_jsonl("baremodel.jsonl", bare);
        let entries = parse_codex_jsonl(&path, &Default::default(), None).unwrap();
        assert_eq!(entries[0].model.as_deref(), Some("gpt-5"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_malformed_and_irrelevant_lines_skipped() {
        let content = concat!(
            "not json at all\n",
            "{broken json\n",
            "{\"type\":\"event_msg\",\"payload\":null,\"note\":\"token_count\"}\n",
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"user_message\",\"text\":\"token_count here\"}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"token_count\"}}\n",
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"info\":{\"last_token_usage\":{\"input_tokens\":10,\"output_tokens\":5}}},\"timestamp\":\"t1\"}\n",
        );
        let path = write_temp_jsonl("malformed.jsonl", content);
        let entries = parse_codex_jsonl(&path, &Default::default(), None).unwrap();
        assert_eq!(entries.len(), 1, "only the well-formed token_count event counts");
        assert_eq!(entries[0].total_tokens, 15);
        let _ = std::fs::remove_file(&path);
    }

    const CUM_WITHOUT_LAST_JSONL: &str = r#"{"type":"session_meta","payload":{"id":"abc","source":"vscode","timestamp":"t","cwd":"/tmp"}}
{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":100,"cached_input_tokens":30,"output_tokens":50,"reasoning_output_tokens":10,"total_tokens":160}}},"timestamp":"t1"}
"#;

    #[test]
    fn test_cum_without_last_uses_cum_fields() {
        let path = write_temp_jsonl("cumonly.jsonl", CUM_WITHOUT_LAST_JSONL);
        let entries = parse_codex_jsonl(&path, &Default::default(), None).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].input_tokens, 70);
        assert_eq!(entries[0].cache_read_tokens, 30);
        assert_eq!(entries[0].output_tokens, 50);
        assert_eq!(entries[0].reasoning_tokens, 10);
        assert_eq!(entries[0].total_tokens, 160);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_zero_total_event_skipped() {
        let content = r#"{"type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":0,"output_tokens":0,"cached_input_tokens":0}}},"timestamp":"t1"}
"#;
        let path = write_temp_jsonl("zerototal.jsonl", content);
        let entries = parse_codex_jsonl(&path, &Default::default(), None).unwrap();
        assert!(entries.is_empty());
        let _ = std::fs::remove_file(&path);
    }

    const CUM_RECORDING_JSONL: &str = r#"{"type":"session_meta","payload":{"id":"abc","source":"vscode","timestamp":"t","cwd":"/tmp"}}
{"type":"turn_context","payload":{"model":"gpt-5","cwd":"/tmp"}}
{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":100,"cached_input_tokens":30,"output_tokens":50,"reasoning_output_tokens":0,"total_tokens":150},"last_token_usage":{"input_tokens":100,"cached_input_tokens":30,"output_tokens":50,"reasoning_output_tokens":0,"total_tokens":150}},"model":"gpt-5"},"timestamp":"t1"}
{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":100,"cached_input_tokens":30,"output_tokens":50,"reasoning_output_tokens":0,"total_tokens":150},"last_token_usage":{"input_tokens":100,"cached_input_tokens":30,"output_tokens":50,"reasoning_output_tokens":0,"total_tokens":150}},"model":"gpt-5"},"timestamp":"t2"}
{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":250,"cached_input_tokens":80,"output_tokens":100,"reasoning_output_tokens":10,"total_tokens":360},"last_token_usage":{"input_tokens":150,"cached_input_tokens":50,"output_tokens":50,"reasoning_output_tokens":10,"total_tokens":210}},"model":"gpt-5"},"timestamp":"t3"}
{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":300,"cached_input_tokens":90,"output_tokens":120,"total_tokens":420},"last_token_usage":{"input_tokens":50,"cached_input_tokens":10,"output_tokens":20,"reasoning_output_tokens":0,"total_tokens":70}},"model":"gpt-5"},"timestamp":"t4"}
{"type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":10,"cached_input_tokens":0,"output_tokens":5,"reasoning_output_tokens":0,"total_tokens":15}},"model":"gpt-5"},"timestamp":"t5"}
"#;

    #[test]
    fn test_cum_out_records_only_complete_snapshots() {
        let path = write_temp_jsonl("cumrec.jsonl", CUM_RECORDING_JSONL);
        let mut cum_out = Vec::new();
        let content = std::fs::read_to_string(&path).unwrap();
        let entries = parse_codex_jsonl_content(
            &path,
            &content,
            &Default::default(),
            None,
            Some(&mut cum_out),
        )
        .unwrap();

        assert_eq!(
            cum_out,
            vec![(100, 30, 50, 0), (100, 30, 50, 0), (250, 80, 100, 10)]
        );

        assert_eq!(entries.len(), 4);
        assert_eq!(entries[1].total_tokens, 210);
        assert_eq!(entries[2].total_tokens, 70);
        assert_eq!(entries[3].total_tokens, 15);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_wrong_typed_usage_field_skips_line() {
        let content = r#"{"type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":"100","output_tokens":50}}},"timestamp":"t1"}
"#;
        let path = write_temp_jsonl("wrongtype.jsonl", content);
        let entries = parse_codex_jsonl(&path, &Default::default(), None).unwrap();
        assert!(entries.is_empty());
        let _ = std::fs::remove_file(&path);
    }

    fn value_str<'a>(v: &'a serde_json::Value, pointer: &str) -> Option<&'a str> {
        v.pointer(pointer).and_then(|x| x.as_str())
    }

    fn value_quad(v: &serde_json::Value, pointer: &str) -> Option<(u64, u64, u64, u64)> {
        let tt = v.pointer(pointer)?;
        if !tt.is_object() {
            return None;
        }
        let f = |k: &str| tt.get(k).and_then(|t| t.as_u64()).unwrap_or(0);
        Some((
            f("input_tokens"),
            f("cached_input_tokens"),
            f("output_tokens"),
            f("reasoning_output_tokens"),
        ))
    }

    fn value_recordable(v: &serde_json::Value, pointer: &str) -> bool {
        let Some(tt) = v.pointer(pointer) else { return false };
        tt.get("input_tokens").and_then(|t| t.as_u64()).is_some()
            && tt.get("cached_input_tokens").and_then(|t| t.as_u64()).is_some()
            && tt.get("output_tokens").and_then(|t| t.as_u64()).is_some()
            && tt.get("reasoning_output_tokens").and_then(|t| t.as_u64()).is_some()
    }

    #[test]
    fn test_borrowed_line_matches_value_extraction_on_real_data() {
        let adapter = CodexAdapter::new();
        let Ok(paths) = adapter.find_data_paths() else { return };
        let mut files = Vec::new();
        for p in &paths {
            files.extend(collect_jsonl_files(p));
        }
        files.sort();

        let mut checked = 0usize;
        'outer: for file in files.into_iter().take(80) {
            let Ok(content) = std::fs::read_to_string(&file) else { continue };
            for line in content.lines() {
                if !line.contains("token_count") && !line.contains("turn_context") {
                    continue;
                }
                let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else { continue };
                // Wrong-typed lines are skipped by the borrowed parser by
                // design; the differential covers lines both accept.
                let Ok(b) = serde_json::from_str::<CodexLine>(line) else { continue };
                checked += 1;

                assert_eq!(b.kind, value_str(&v, "/type"), "kind: {}", line);
                assert_eq!(
                    b.payload.as_ref().and_then(|p| p.kind),
                    value_str(&v, "/payload/type"),
                    "payload kind: {}",
                    line
                );
                assert_eq!(
                    b.payload.as_ref().and_then(|p| p.model),
                    value_str(&v, "/payload/model"),
                    "model: {}",
                    line
                );
                assert_eq!(
                    b.payload.as_ref().and_then(|p| p.model_name),
                    value_str(&v, "/payload/model_name"),
                    "model_name: {}",
                    line
                );
                assert_eq!(
                    b.payload.as_ref().and_then(|p| p.cwd),
                    value_str(&v, "/payload/cwd"),
                    "cwd: {}",
                    line
                );
                assert_eq!(b.timestamp, value_str(&v, "/timestamp"), "timestamp: {}", line);

                let borrowed_cum = b
                    .payload
                    .as_ref()
                    .and_then(|p| p.info.as_ref())
                    .and_then(|i| i.total_token_usage.as_ref());
                assert_eq!(
                    borrowed_cum.map(|q| UsageQuad::fields(Some(q))),
                    value_quad(&v, "/payload/info/total_token_usage"),
                    "cum fields: {}",
                    line
                );

                let borrowed_last = b
                    .payload
                    .as_ref()
                    .and_then(|p| p.info.as_ref())
                    .and_then(|i| i.last_token_usage.as_ref());
                assert_eq!(
                    borrowed_last.map(|q| UsageQuad::fields(Some(q))),
                    value_quad(&v, "/payload/info/last_token_usage"),
                    "last fields: {}",
                    line
                );

                assert_eq!(
                    borrowed_cum.and_then(UsageQuad::complete).is_some(),
                    value_recordable(&v, "/payload/info/total_token_usage"),
                    "recordable: {}",
                    line
                );
            }
            if checked > 30000 {
                break 'outer;
            }
        }
        assert!(checked > 1000, "expected to check a substantial number of real lines");
    }
}
