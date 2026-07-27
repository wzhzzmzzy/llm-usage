pub mod claude;
pub mod codex;
pub mod gemini;
pub mod opencode;

use std::path::PathBuf;

use chrono::{DateTime, Utc};

use crate::core::model::Source;

/// Common types for all adapters
#[derive(Debug, Clone)]
pub struct UsageEntry {
    pub session_id: String,
    pub timestamp: String,
    pub model: Option<String>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    /// Reasoning/thinking tokens (billed at the output rate by providers)
    pub reasoning_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,
    pub total_tokens: u64,
    pub project_path: Option<String>,
}

/// Per-model token breakdown
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelBreakdown {
    pub model: String,
    pub input_tokens: u64,
    pub cache_read_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_tokens: u64,
    pub total_tokens: u64,
    pub request_count: u64,
}

/// Adapter trait for loading usage data from different sources
pub trait UsageAdapter: Send + Sync {
    /// Get the source type
    fn source(&self) -> Source;

    /// Find all usage data directories
    fn find_data_paths(&self) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>>;

    /// Load all usage entries from the data directories
    fn load_entries(&self, paths: &[PathBuf]) -> Result<Vec<UsageEntry>, Box<dyn std::error::Error>>;

    /// Aggregate entries into daily report
    fn aggregate_daily(&self, entries: &[UsageEntry]) -> Vec<DailyAggregate>;

    /// Aggregate entries into monthly report
    fn aggregate_monthly(&self, entries: &[UsageEntry]) -> Vec<MonthlyAggregate>;

    /// Aggregate entries into session report
    fn aggregate_session(&self, entries: &[UsageEntry]) -> Vec<SessionAggregate>;

    /// Aggregate entries into block report (billing windows)
    fn aggregate_blocks(&self, entries: &[UsageEntry], block_duration_hours: i64) -> Vec<BlockAggregate>;
}

/// Aggregated daily usage
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyAggregate {
    pub date: String,
    pub total_tokens: u64,
    pub input_tokens: u64,
    pub cache_read_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_tokens: u64,
    pub request_count: u64,
    pub models_used: Vec<String>,
    pub model_breakdown: Vec<ModelBreakdown>,
}

/// Aggregated monthly usage
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthlyAggregate {
    pub month: String,
    pub total_tokens: u64,
    pub input_tokens: u64,
    pub cache_read_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_tokens: u64,
    pub request_count: u64,
    pub models_used: Vec<String>,
    pub model_breakdown: Vec<ModelBreakdown>,
}

/// Aggregated session usage
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionAggregate {
    pub session_id: String,
    pub project_path: Option<String>,
    pub total_tokens: u64,
    pub input_tokens: u64,
    pub cache_read_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_tokens: u64,
    pub request_count: u64,
    pub last_activity: Option<String>,
    pub models_used: Vec<String>,
    pub model_breakdown: Vec<ModelBreakdown>,
}

/// Aggregated block usage (5-hour billing windows)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockAggregate {
    pub block_id: String,
    pub start_time: String,
    pub end_time: String,
    pub actual_end_time: Option<String>,
    pub is_active: bool,
    pub total_tokens: u64,
    pub input_tokens: u64,
    pub cache_read_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_tokens: u64,
    pub request_count: u64,
    pub models_used: Vec<String>,
    pub model_breakdown: Vec<ModelBreakdown>,
}

/// Default block duration in hours (Claude's billing window)
pub const DEFAULT_BLOCK_DURATION_HOURS: i64 = 5;

/// Helper to build model breakdown from entries
pub fn build_model_breakdown(entries: &[UsageEntry]) -> Vec<ModelBreakdown> {
    use std::collections::HashMap;

    let mut map: HashMap<String, ModelBreakdown> = HashMap::new();

    for entry in entries {
        let model = entry.model.clone().unwrap_or_else(|| "unknown".to_string());
        let breakdown = map.entry(model.clone()).or_insert_with(|| ModelBreakdown {
            model,
            input_tokens: 0,
            cache_read_tokens: 0,
            output_tokens: 0,
            reasoning_tokens: 0,
            total_tokens: 0,
            request_count: 0,
        });

        breakdown.input_tokens += entry.input_tokens;
        breakdown.cache_read_tokens += entry.cache_read_tokens;
        breakdown.output_tokens += entry.output_tokens;
        breakdown.reasoning_tokens += entry.reasoning_tokens;
        breakdown.total_tokens += entry.total_tokens;
        breakdown.request_count += 1;
    }

    let mut result: Vec<ModelBreakdown> = map.into_values().collect();
    result.sort_by(|a, b| b.total_tokens.cmp(&a.total_tokens));
    result
}

/// Shared blocks aggregation logic - groups entries into time-based blocks
pub fn aggregate_blocks_impl(entries: &[UsageEntry], block_duration_hours: i64) -> Vec<BlockAggregate> {
    use chrono::{DateTime, Duration, Utc};

    if entries.is_empty() {
        return Vec::new();
    }

    let block_duration_ms = Duration::hours(block_duration_hours);
    let now = Utc::now();

    let mut sorted_entries: Vec<&UsageEntry> = entries.iter().collect();
    sorted_entries.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

    let mut blocks: Vec<BlockAggregate> = Vec::new();
    let mut current_block_start: Option<DateTime<Utc>> = None;
    let mut current_block_entries: Vec<&UsageEntry> = Vec::new();

    for entry in &sorted_entries {
        let entry_time = DateTime::parse_from_rfc3339(&entry.timestamp)
            .ok()
            .map(|dt| dt.with_timezone(&Utc));

        let Some(entry_time) = entry_time else {
            continue;
        };

        if let Some(block_start) = current_block_start {
            let time_since_start = entry_time - block_start;
            let last_entry_time = current_block_entries.last()
                .and_then(|e| DateTime::parse_from_rfc3339(&e.timestamp).ok())
                .map(|dt| dt.with_timezone(&Utc));

            let should_start_new_block = time_since_start > block_duration_ms
                || last_entry_time.is_some_and(|last| entry_time - last > block_duration_ms);

            if should_start_new_block {
                if !current_block_entries.is_empty() {
                    blocks.push(create_block(
                        block_start,
                        &current_block_entries,
                        now,
                        block_duration_ms,
                    ));
                }
                current_block_start = Some(entry_time);
                current_block_entries = vec![entry];
            } else {
                current_block_entries.push(entry);
            }
        } else {
            current_block_start = Some(entry_time);
            current_block_entries = vec![entry];
        }
    }

    if let Some(block_start) = current_block_start {
        if !current_block_entries.is_empty() {
            blocks.push(create_block(
                block_start,
                &current_block_entries,
                now,
                block_duration_ms,
            ));
        }
    }

    blocks
}

fn create_block(
    start_time: DateTime<Utc>,
    entries: &[&UsageEntry],
    now: DateTime<Utc>,
    block_duration: chrono::Duration,
) -> BlockAggregate {
    let end_time = start_time + block_duration;
    let actual_end_time = entries.last()
        .and_then(|e| DateTime::parse_from_rfc3339(&e.timestamp).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let is_active = actual_end_time
        .map(|actual| now - actual < block_duration && now < end_time)
        .unwrap_or(false);

    let total_tokens: u64 = entries.iter().map(|e| e.total_tokens).sum();
    let input_tokens: u64 = entries.iter().map(|e| e.input_tokens).sum();
    let cache_read_tokens: u64 = entries.iter().map(|e| e.cache_read_tokens).sum();
    let output_tokens: u64 = entries.iter().map(|e| e.output_tokens).sum();
    let reasoning_tokens: u64 = entries.iter().map(|e| e.reasoning_tokens).sum();

    let mut models_used: Vec<String> = entries
        .iter()
        .filter_map(|e| e.model.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    models_used.sort();

    let model_breakdown = build_model_breakdown(
        &entries.iter().map(|e| (*e).clone()).collect::<Vec<_>>(),
    );

    BlockAggregate {
        block_id: start_time.to_rfc3339(),
        start_time: start_time.to_rfc3339(),
        end_time: end_time.to_rfc3339(),
        actual_end_time: actual_end_time.map(|t| t.to_rfc3339()),
        is_active,
        total_tokens,
        input_tokens,
        cache_read_tokens,
        output_tokens,
        reasoning_tokens,
        request_count: entries.len() as u64,
        models_used,
        model_breakdown,
    }
}
