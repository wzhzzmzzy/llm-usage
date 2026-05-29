pub mod claude;
pub mod codex;
pub mod opencode;

use std::path::PathBuf;

use crate::core::model::Source;

/// Common types for all adapters
#[derive(Debug, Clone)]
pub struct UsageEntry {
    pub session_id: String,
    pub timestamp: String,
    pub model: Option<String>,
    pub input_tokens: u64,
    pub output_tokens: u64,
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
    pub request_count: u64,
    pub last_activity: Option<String>,
    pub models_used: Vec<String>,
    pub model_breakdown: Vec<ModelBreakdown>,
}

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
            total_tokens: 0,
            request_count: 0,
        });

        breakdown.input_tokens += entry.input_tokens;
        breakdown.cache_read_tokens += entry.cache_read_tokens;
        breakdown.output_tokens += entry.output_tokens;
        breakdown.total_tokens += entry.total_tokens;
        breakdown.request_count += 1;
    }

    let mut result: Vec<ModelBreakdown> = map.into_values().collect();
    result.sort_by(|a, b| b.total_tokens.cmp(&a.total_tokens));
    result
}
