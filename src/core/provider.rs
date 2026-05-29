use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::core::adapter::{claude::ClaudeAdapter, codex::CodexAdapter, opencode::OpenCodeAdapter, UsageAdapter};
use crate::core::cost::{calculate_cost, CostMode};
use crate::core::model::*;
use crate::core::pricing::PricingCache;

/// Raw types for JSON deserialization from native adapters
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
    #[serde(default)]
    pub total_tokens: Option<u64>,
    #[serde(alias = "costUSD", default)]
    pub total_cost: f64,
    #[serde(default)]
    pub request_count: Option<u64>,
    #[serde(default)]
    pub models_used: Option<Vec<String>>,
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
    #[serde(default)]
    pub cache_creation_tokens: Option<u64>,
    #[serde(default)]
    pub cache_read_tokens: Option<u64>,
    #[serde(default)]
    pub total_tokens: Option<u64>,
    #[serde(alias = "costUSD", default)]
    pub total_cost: f64,
    #[serde(default)]
    pub request_count: Option<u64>,
    #[serde(default)]
    pub models_used: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawSessionRow {
    #[serde(alias = "sessionId")]
    pub session_id: String,
    #[serde(default)]
    pub project_path: Option<String>,
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub total_tokens: Option<u64>,
    #[serde(alias = "costUSD", default)]
    pub total_cost: f64,
    #[serde(default)]
    pub last_activity: Option<String>,
    #[serde(default)]
    pub models_used: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawBlockRow {
    pub block_id: String,
    pub start_time: String,
    pub end_time: Option<String>,
    #[serde(default)]
    pub is_active: bool,
    #[serde(default)]
    pub is_gap: Option<bool>,
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub total_tokens: Option<u64>,
    #[serde(default)]
    pub cost_usd: Option<f64>,
    #[serde(default)]
    pub models_used: Option<Vec<String>>,
}

/// Native usage provider that reads directly from agent data directories
pub struct NativeUsageProvider {
    adapters: Vec<Box<dyn UsageAdapter>>,
    pricing_cache: Arc<PricingCache>,
}

impl NativeUsageProvider {
    pub fn new() -> Self {
        Self {
            adapters: vec![
                Box::new(ClaudeAdapter::new()),
                Box::new(CodexAdapter::new()),
                Box::new(OpenCodeAdapter::new()),
            ],
            pricing_cache: Arc::new(PricingCache::new().expect("Failed to create pricing cache")),
        }
    }

    pub async fn execute_cell(&self, cell: Cell) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let adapter = self.find_adapter(cell.source)?;
        let paths = adapter.find_data_paths().map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())) as Box<dyn std::error::Error + Send + Sync>)?;
        let entries = adapter.load_entries(&paths).map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())) as Box<dyn std::error::Error + Send + Sync>)?;
        
        let entries_with_cost = self.calculate_costs(entries).await?;
        
        let json = match cell.report {
            ReportType::Daily => {
                let aggregates = adapter.aggregate_daily(&entries_with_cost);
                let daily_rows: Vec<serde_json::Value> = aggregates.iter().map(|agg| {
                    serde_json::json!({
                        "date": agg.date,
                        "inputTokens": agg.input_tokens,
                        "outputTokens": agg.output_tokens,
                        "cacheCreationTokens": agg.cache_creation_tokens,
                        "cacheReadTokens": agg.cache_read_tokens,
                        "totalTokens": agg.total_tokens,
                        "requestCount": agg.request_count,
                        "modelsUsed": agg.models_used,
                    })
                }).collect();
                serde_json::json!({ "daily": daily_rows }).to_string()
            }
            ReportType::Monthly => {
                let aggregates = adapter.aggregate_monthly(&entries_with_cost);
                let monthly_rows: Vec<serde_json::Value> = aggregates.iter().map(|agg| {
                    serde_json::json!({
                        "month": agg.month,
                        "inputTokens": agg.input_tokens,
                        "outputTokens": agg.output_tokens,
                        "cacheCreationTokens": agg.cache_creation_tokens,
                        "cacheReadTokens": agg.cache_read_tokens,
                        "totalTokens": agg.total_tokens,
                        "requestCount": agg.request_count,
                        "modelsUsed": agg.models_used,
                    })
                }).collect();
                serde_json::json!({ "monthly": monthly_rows }).to_string()
            }
            ReportType::Session => {
                let aggregates = adapter.aggregate_session(&entries_with_cost);
                let session_rows: Vec<serde_json::Value> = aggregates.iter().map(|agg| {
                    serde_json::json!({
                        "sessionId": agg.session_id,
                        "projectPath": agg.project_path,
                        "inputTokens": agg.input_tokens,
                        "outputTokens": agg.output_tokens,
                        "totalTokens": agg.total_tokens,
                        "lastActivity": agg.last_activity,
                        "modelsUsed": agg.models_used,
                    })
                }).collect();
                serde_json::json!({ "sessions": session_rows }).to_string()
            }
            ReportType::Blocks => {
                serde_json::json!({ "blocks": [] }).to_string()
            }
        };
        
        Ok(json)
    }

    pub async fn health_check(&self) -> ProviderHealth {
        ProviderHealth {
            runner_found: true,
            runner_path: Some("native".to_string()),
            runner_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            ccusage_available: true,
            ccusage_version: Some("native".to_string()),
        }
    }

    fn find_adapter(&self, source: Source) -> Result<&dyn UsageAdapter, Box<dyn std::error::Error + Send + Sync>> {
        self.adapters
            .iter()
            .find(|a| a.source() == source || source == Source::All)
            .map(|a| a.as_ref())
            .ok_or_else(|| format!("No adapter found for source: {:?}", source).into())
    }

    async fn calculate_costs(&self, entries: Vec<super::adapter::UsageEntry>) -> Result<Vec<super::adapter::UsageEntry>, Box<dyn std::error::Error + Send + Sync>> {
        let mut result = Vec::with_capacity(entries.len());
        
        for entry in entries {
            let cost = if entry.cost_usd.is_some() {
                entry.cost_usd.unwrap()
            } else {
                let model = entry.model.as_deref().unwrap_or("");
                if let Some(pricing) = self.pricing_cache.get_pricing(model).await {
                    calculate_cost(
                        entry.model.as_deref(),
                        entry.input_tokens,
                        entry.output_tokens,
                        entry.cache_creation_tokens,
                        entry.cache_read_tokens,
                        None,
                        CostMode::Calculate,
                        Some(&pricing),
                    )
                } else {
                    0.0
                }
            };
            
            result.push(super::adapter::UsageEntry {
                cost_usd: Some(cost),
                ..entry
            });
        }
        
        Ok(result)
    }
}

#[async_trait]
pub trait UsageProvider: Send + Sync {
    async fn execute_cell(&self, cell: Cell) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;
    async fn health_check(&self) -> ProviderHealth;
}

#[async_trait]
impl UsageProvider for NativeUsageProvider {
    async fn execute_cell(&self, cell: Cell) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        self.execute_cell(cell).await
    }

    async fn health_check(&self) -> ProviderHealth {
        self.health_check().await
    }
}

pub struct ProviderHealth {
    pub runner_found: bool,
    pub runner_path: Option<String>,
    pub runner_version: Option<String>,
    pub ccusage_available: bool,
    pub ccusage_version: Option<String>,
}
