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
    pub cost_usd: Option<f64>,
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
#[derive(Debug, Clone)]
pub struct DailyAggregate {
    pub date: String,
    pub total_tokens: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,
    pub request_count: u64,
    pub models_used: Vec<String>,
}

/// Aggregated monthly usage
#[derive(Debug, Clone)]
pub struct MonthlyAggregate {
    pub month: String,
    pub total_tokens: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,
    pub request_count: u64,
    pub models_used: Vec<String>,
}

/// Aggregated session usage
#[derive(Debug, Clone)]
pub struct SessionAggregate {
    pub session_id: String,
    pub project_path: Option<String>,
    pub total_tokens: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub last_activity: Option<String>,
    pub models_used: Vec<String>,
}
