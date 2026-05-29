use crate::core::model::Cell;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawCcUsageOutput {
    pub daily: Option<Vec<RawDailyRow>>,
    pub monthly: Option<Vec<RawMonthlyRow>>,
    pub session: Option<Vec<RawSessionRow>>,
    pub blocks: Option<Vec<RawBlockRow>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawDailyRow {
    #[serde(alias = "date")]
    pub period: String,
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_creation_tokens: Option<u64>,
    #[serde(default)]
    pub cache_read_tokens: Option<u64>,
    #[serde(alias = "costUSD", default)]
    pub total_cost: f64,
    #[serde(default)]
    pub total_tokens: Option<u64>,
    #[serde(default)]
    pub models_used: Option<Vec<String>>,
    #[serde(default)]
    pub model_breakdowns: Option<Vec<RawModelBreakdown>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawMonthlyRow {
    #[serde(alias = "month")]
    pub period: String,
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(alias = "cachedInputTokens", default)]
    pub cache_creation_tokens: Option<u64>,
    #[serde(default)]
    pub cache_read_tokens: Option<u64>,
    #[serde(alias = "costUSD", default)]
    pub total_cost: f64,
    #[serde(default)]
    pub total_tokens: Option<u64>,
    #[serde(default)]
    pub models_used: Option<Vec<String>>,
    #[serde(default)]
    pub model_breakdowns: Option<Vec<RawModelBreakdown>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawSessionRow {
    #[serde(alias = "sessionId")]
    pub period: String,
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_creation_tokens: Option<u64>,
    #[serde(default)]
    pub cache_read_tokens: Option<u64>,
    #[serde(alias = "costUSD", default)]
    pub total_cost: f64,
    #[serde(default)]
    pub total_tokens: Option<u64>,
    #[serde(default)]
    pub models_used: Option<Vec<String>>,
    #[serde(alias = "models", default)]
    pub model_breakdowns: Option<Vec<RawModelBreakdown>>,
    #[serde(default)]
    pub metadata: Option<RawSessionMetadata>,
    #[serde(default)]
    pub last_activity: Option<String>,
    #[serde(default)]
    pub project_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawSessionMetadata {
    pub last_activity: Option<String>,
    pub project_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawBlockRow {
    pub id: String,
    pub start_time: String,
    pub end_time: Option<String>,
    pub actual_end_time: Option<String>,
    #[serde(default)]
    pub is_active: bool,
    #[serde(default)]
    pub is_gap: Option<bool>,
    #[serde(default)]
    pub models: Option<Vec<String>>,
    #[serde(alias = "costUSD", default)]
    pub cost_usd: Option<f64>,
    #[serde(default)]
    pub total_tokens: Option<u64>,
    #[serde(default)]
    pub token_counts: Option<RawTokenCounts>,
    #[serde(default)]
    pub entries: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawTokenCounts {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_creation_input_tokens: Option<u64>,
    #[serde(default)]
    pub cache_read_input_tokens: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawModelBreakdown {
    #[serde(alias = "modelName")]
    pub model_name: String,
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_creation_tokens: Option<u64>,
    #[serde(default)]
    pub cache_read_tokens: Option<u64>,
    #[serde(default)]
    pub cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderHealth {
    pub runner_found: bool,
    pub runner_path: Option<String>,
    pub runner_version: Option<String>,
    pub ccusage_available: bool,
    pub ccusage_version: Option<String>,
}

#[async_trait::async_trait]
pub trait UsageProvider: Send + Sync {
    async fn execute_cell(&self, cell: Cell) -> Result<String, crate::core::error::ProviderError>;
    async fn health_check(&self) -> ProviderHealth;
}
