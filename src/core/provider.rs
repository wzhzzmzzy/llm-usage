use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::core::adapter::{claude::ClaudeAdapter, codex::CodexAdapter, opencode::OpenCodeAdapter, UsageAdapter, UsageEntry};
use crate::core::cost::{calculate_cost, CostMode};
use crate::core::model::*;
use crate::core::pricing::PricingCache;

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

/// Cached entries for a single source (claude/codex/opencode)
struct SourceCache {
    entries_with_cost: Vec<UsageEntry>,
}

/// Native usage provider with per-source entry caching
pub struct NativeUsageProvider {
    adapters: Vec<Box<dyn UsageAdapter>>,
    pricing_cache: Arc<PricingCache>,
    source_cache: RwLock<HashMap<Source, SourceCache>>,
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
            source_cache: RwLock::new(HashMap::new()),
        }
    }

    /// Pre-load all source data, pricing, and costs. Call once before executing cells.
    pub async fn preload(&self) {
        let start = std::time::Instant::now();

        self.pricing_cache.ensure_loaded().await;
        tracing::info!("pricing preload: {:?}", start.elapsed());

        let mut cache = self.source_cache.write().await;
        for adapter in &self.adapters {
            let source = adapter.source();
            let adapter_start = std::time::Instant::now();
            let raw_entries = match adapter.find_data_paths() {
                Ok(paths) => adapter.load_entries(&paths).unwrap_or_default(),
                Err(_) => Vec::new(),
            };
            let entry_count = raw_entries.len();

            let entries_with_cost = self.calculate_costs_sync(raw_entries);

            tracing::info!("{:?} adapter: {} entries loaded + costs calculated, {:?}", source, entry_count, adapter_start.elapsed());
            cache.insert(source, SourceCache { entries_with_cost });
        }

        tracing::info!("total preload: {:?}, {} total entries", start.elapsed(),
            cache.values().map(|c| c.entries_with_cost.len()).sum::<usize>());
    }

    /// Check if a source has any data without loading files
    pub fn source_has_data(&self, source: Source) -> bool {
        let adapter = match self.adapters.iter().find(|a| a.source() == source) {
            Some(a) => a,
            None => return false,
        };
        match adapter.find_data_paths() {
            Ok(paths) => !paths.is_empty(),
            Err(_) => false,
        }
    }

    pub async fn execute_cell(&self, cell: Cell) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let entries_with_cost = if cell.source == Source::All {
            let cache = self.source_cache.read().await;
            let mut all = Vec::new();
            for cached in cache.values() {
                all.extend(cached.entries_with_cost.clone());
            }
            all
        } else {
            let cache = self.source_cache.read().await;
            match cache.get(&cell.source) {
                Some(cached) => cached.entries_with_cost.clone(),
                None => Vec::new(),
            }
        };

        let any_adapter = self.adapters.first()
            .ok_or("No adapters available")?;

        let json = match cell.report {
            ReportType::Daily => {
                let aggregates = any_adapter.aggregate_daily(&entries_with_cost);
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
                let aggregates = any_adapter.aggregate_monthly(&entries_with_cost);
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
                let aggregates = any_adapter.aggregate_session(&entries_with_cost);
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

    /// Calculate costs synchronously using a snapshot of the pricing map.
    /// Called once during preload so all cells reuse pre-computed costs.
    fn calculate_costs_sync(&self, entries: Vec<UsageEntry>) -> Vec<UsageEntry> {
        let pricing_snapshot = self.pricing_cache.get_snapshot_sync();
        let mut result = Vec::with_capacity(entries.len());

        for entry in entries {
            let cost = if let Some(existing) = entry.cost_usd.filter(|c| *c > 0.0) {
                existing
            } else {
                let model = entry.model.as_deref().unwrap_or("");
                pricing_snapshot
                    .find(model)
                    .map(|pricing| {
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
                    })
                    .unwrap_or(0.0)
            };

            result.push(UsageEntry {
                cost_usd: Some(cost),
                ..entry
            });
        }

        result
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::adapter::UsageEntry;

    fn mock_entry(model: &str, input: u64, output: u64) -> UsageEntry {
        UsageEntry {
            session_id: "test-session".to_string(),
            timestamp: "2026-05-29T10:00:00Z".to_string(),
            model: Some(model.to_string()),
            input_tokens: input,
            output_tokens: output,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
            total_tokens: input + output,
            cost_usd: None,
        }
    }

    #[test]
    fn test_calculate_costs_sync_uses_existing_cost() {
        let provider = NativeUsageProvider::new();
        let entries = vec![UsageEntry {
            cost_usd: Some(0.05),
            ..mock_entry("test", 100, 200)
        }];

        let result = provider.calculate_costs_sync(entries);
        assert_eq!(result[0].cost_usd, Some(0.05));
    }

    #[tokio::test]
    async fn test_execute_cell_daily_returns_correct_json() {
        let provider = NativeUsageProvider::new();

        let cell = Cell { source: Source::Claude, report: ReportType::Daily };
        let json = provider.execute_cell(cell).await.unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert!(value.get("daily").is_some());
        assert!(value["daily"].is_array());
    }

    #[tokio::test]
    async fn test_execute_cell_monthly_returns_correct_json() {
        let provider = NativeUsageProvider::new();

        let cell = Cell { source: Source::Codex, report: ReportType::Monthly };
        let json = provider.execute_cell(cell).await.unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert!(value.get("monthly").is_some());
        assert!(value["monthly"].is_array());
    }

    #[tokio::test]
    async fn test_execute_cell_session_returns_correct_json() {
        let provider = NativeUsageProvider::new();

        let cell = Cell { source: Source::Opencode, report: ReportType::Session };
        let json = provider.execute_cell(cell).await.unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert!(value.get("sessions").is_some());
        assert!(value["sessions"].is_array());
    }

    #[tokio::test]
    async fn test_preload_populates_cache() {
        let provider = NativeUsageProvider::new();
        provider.preload().await;

        let cache = provider.source_cache.read().await;
        assert!(cache.contains_key(&Source::Claude));
        assert!(cache.contains_key(&Source::Codex));
        assert!(cache.contains_key(&Source::Opencode));
    }

    #[tokio::test]
    async fn test_daily_json_deserializes_through_normalize() {
        let provider = NativeUsageProvider::new();
        let cell = Cell { source: Source::Claude, report: ReportType::Daily };
        let json = provider.execute_cell(cell).await.unwrap();

        #[derive(serde::Deserialize)]
        struct Wrapper { daily: Vec<RawDailyRow> }

        let wrapper: Wrapper = serde_json::from_str(&json).unwrap();
        let report = crate::core::normalize::Normalizer::normalize_daily(&wrapper.daily).unwrap();
        let _ = report.days.len();
    }

    #[tokio::test]
    async fn test_session_json_deserializes_through_normalize() {
        let provider = NativeUsageProvider::new();
        let cell = Cell { source: Source::Opencode, report: ReportType::Session };
        let json = provider.execute_cell(cell).await.unwrap();

        #[derive(serde::Deserialize)]
        struct Wrapper { sessions: Vec<RawSessionRow> }

        let wrapper: Wrapper = serde_json::from_str(&json).unwrap();
        let report = crate::core::normalize::Normalizer::normalize_session(&wrapper.sessions).unwrap();
        assert_eq!(report.sessions.len(), 0);
    }

    #[tokio::test]
    async fn test_full_refresh_proces_cell_flow() {
        let provider = NativeUsageProvider::new();

        let cell = Cell { source: Source::Claude, report: ReportType::Daily };
        let json = provider.execute_cell(cell).await.unwrap();

        #[derive(serde::Deserialize)]
        struct Wrapper { daily: Vec<RawDailyRow> }

        let wrapper: Wrapper = serde_json::from_str(&json).unwrap();
        let report = crate::core::normalize::Normalizer::normalize_daily(&wrapper.daily).unwrap();

        let snapshot_json = serde_json::to_value(&report).unwrap();
        assert!(snapshot_json.get("days").is_some());
        assert!(snapshot_json.get("totals").is_some());

        let totals = &snapshot_json["totals"];
        assert!(totals.get("totalCostUsd").is_some());
        assert!(totals.get("totalCostUsdNumber").is_some());
        assert!(totals.get("costFormatted").is_some());
        assert!(totals.get("totalTokens").is_some());
        assert!(totals.get("inputTokens").is_some());
        assert!(totals.get("outputTokens").is_some());
    }
}
