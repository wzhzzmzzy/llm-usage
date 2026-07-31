use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::core::adapter::{claude::ClaudeAdapter, codex::CodexAdapter, gemini::GeminiAdapter, opencode::OpenCodeAdapter, BlockAggregate, DailyAggregate, MonthlyAggregate, SessionAggregate, UsageAdapter, UsageEntry};
use crate::core::cost::apply_costs;
use crate::core::model::*;
use crate::core::pricing::PricingCache;

/// Cached entries for a single source (claude/codex/opencode)
struct SourceCache {
    entries: Arc<Vec<UsageEntry>>,
}

/// Typed per-cell aggregation result, handed to the refresh pipeline as-is
/// (avoids a JSON serialize/parse round trip between provider and normalizer).
pub enum CellAggregates {
    Daily(Vec<DailyAggregate>),
    Monthly(Vec<MonthlyAggregate>),
    Session(Vec<SessionAggregate>),
    Blocks(Vec<BlockAggregate>),
}

/// Native usage provider with per-source entry caching
pub struct NativeUsageProvider {
    adapters: Vec<Box<dyn UsageAdapter>>,
    source_cache: RwLock<HashMap<Source, SourceCache>>,
    all_entries: RwLock<Arc<Vec<UsageEntry>>>,
    block_duration_hours: i64,
}

impl NativeUsageProvider {
    pub fn new() -> Self {
        Self::with_block_duration(5)
    }

    pub fn with_block_duration(block_duration_hours: i64) -> Self {
        Self {
            adapters: vec![
                Box::new(ClaudeAdapter::new()),
                Box::new(CodexAdapter::new()),
                Box::new(GeminiAdapter::new()),
                Box::new(OpenCodeAdapter::new()),
            ],
            source_cache: RwLock::new(HashMap::new()),
            all_entries: RwLock::new(Arc::new(Vec::new())),
            block_duration_hours,
        }
    }

    /// Pre-load all source data. Call once before executing cells.
    pub async fn preload(&self) {
        let start = std::time::Instant::now();

        let pricing = match PricingCache::new() {
            Ok(cache) => cache.full_snapshot().await,
            Err(_) => Default::default(),
        };

        let mut handles = Vec::new();
        for source in [Source::Claude, Source::Codex, Source::Gemini, Source::Opencode] {
            let pricing = pricing.clone();
            handles.push(tokio::task::spawn_blocking(move || {
                let adapter_start = std::time::Instant::now();
                let adapter: Box<dyn UsageAdapter> = match source {
                    Source::Claude => Box::new(ClaudeAdapter::new()),
                    Source::Codex => Box::new(CodexAdapter::new()),
                    Source::Gemini => Box::new(GeminiAdapter::new()),
                    Source::Opencode => Box::new(OpenCodeAdapter::new()),
                    Source::All => unreachable!("All is not a data source"),
                };
                let entries = match adapter.find_data_paths() {
                    Ok(paths) => adapter.load_entries(&paths).unwrap_or_default(),
                    Err(_) => Vec::new(),
                };
                let mut entries = entries;
                apply_costs(&mut entries, &pricing);
                (source, entries, adapter_start.elapsed())
            }));
        }

        let mut cache = self.source_cache.write().await;
        let mut all = Vec::new();
        for handle in handles {
            if let Ok((source, entries, elapsed)) = handle.await {
                tracing::info!("{:?} adapter: {} entries loaded, {:?}", source, entries.len(), elapsed);
                all.extend(entries.iter().cloned());
                cache.insert(source, SourceCache { entries: Arc::new(entries) });
            }
        }
        drop(cache);
        *self.all_entries.write().await = Arc::new(all);

        let cache = self.source_cache.read().await;
        tracing::info!("total preload: {:?}, {} total entries", start.elapsed(),
            cache.values().map(|c| c.entries.len()).sum::<usize>());
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

    pub async fn execute_cell(&self, cell: Cell) -> Result<CellAggregates, Box<dyn std::error::Error + Send + Sync>> {
        let entries: Arc<Vec<UsageEntry>> = if cell.source == Source::All {
            self.all_entries.read().await.clone()
        } else {
            let cache = self.source_cache.read().await;
            match cache.get(&cell.source) {
                Some(cached) => cached.entries.clone(),
                None => Arc::new(Vec::new()),
            }
        };

        let any_adapter = self.adapters.first()
            .ok_or("No adapters available")?;

        let aggregates = match cell.report {
            ReportType::Daily => CellAggregates::Daily(any_adapter.aggregate_daily(&entries)),
            ReportType::Monthly => CellAggregates::Monthly(any_adapter.aggregate_monthly(&entries)),
            ReportType::Session => CellAggregates::Session(any_adapter.aggregate_session(&entries)),
            ReportType::Blocks => CellAggregates::Blocks(any_adapter.aggregate_blocks(&entries, self.block_duration_hours)),
        };

        Ok(aggregates)
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
}

#[async_trait]
pub trait UsageProvider: Send + Sync {
    async fn execute_cell(&self, cell: Cell) -> Result<CellAggregates, Box<dyn std::error::Error + Send + Sync>>;
    async fn health_check(&self) -> ProviderHealth;
}

#[async_trait]
impl UsageProvider for NativeUsageProvider {
    async fn execute_cell(&self, cell: Cell) -> Result<CellAggregates, Box<dyn std::error::Error + Send + Sync>> {
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

    #[tokio::test]
    async fn test_preload_populates_cache() {
        let provider = NativeUsageProvider::new();
        provider.preload().await;

        let cache = provider.source_cache.read().await;
        assert!(cache.contains_key(&Source::Claude));
        assert!(cache.contains_key(&Source::Codex));
        assert!(cache.contains_key(&Source::Gemini));
        assert!(cache.contains_key(&Source::Opencode));
    }

    #[tokio::test]
    async fn test_execute_cell_daily_returns_daily_aggregates() {
        let provider = NativeUsageProvider::new();
        provider.preload().await;

        let cell = Cell { source: Source::Claude, report: ReportType::Daily };
        let result = provider.execute_cell(cell).await.unwrap();

        assert!(matches!(result, CellAggregates::Daily(_)));
    }

    #[tokio::test]
    async fn test_execute_cell_monthly_returns_monthly_aggregates() {
        let provider = NativeUsageProvider::new();
        provider.preload().await;

        let cell = Cell { source: Source::Codex, report: ReportType::Monthly };
        let result = provider.execute_cell(cell).await.unwrap();

        assert!(matches!(result, CellAggregates::Monthly(_)));
    }

    #[tokio::test]
    async fn test_execute_cell_session_returns_session_aggregates() {
        let provider = NativeUsageProvider::new();
        provider.preload().await;

        let cell = Cell { source: Source::Opencode, report: ReportType::Session };
        let result = provider.execute_cell(cell).await.unwrap();

        assert!(matches!(result, CellAggregates::Session(_)));
    }

    #[tokio::test]
    async fn test_daily_aggregates_have_expected_fields() {
        let provider = NativeUsageProvider::new();
        provider.preload().await;

        let cell = Cell { source: Source::Claude, report: ReportType::Daily };
        let result = provider.execute_cell(cell).await.unwrap();

        let CellAggregates::Daily(rows) = result else {
            panic!("expected daily aggregates");
        };

        if let Some(first) = rows.first() {
            assert!(!first.date.is_empty());
            assert!(first.total_tokens > 0);
            assert!(first.request_count > 0);
            assert!(!first.models_used.is_empty());
            assert!(!first.model_breakdown.is_empty());
        }
    }

    #[tokio::test]
    async fn test_session_aggregates_have_expected_fields() {
        let provider = NativeUsageProvider::new();
        provider.preload().await;

        let cell = Cell { source: Source::Opencode, report: ReportType::Session };
        let result = provider.execute_cell(cell).await.unwrap();

        let CellAggregates::Session(rows) = result else {
            panic!("expected session aggregates");
        };

        if let Some(first) = rows.first() {
            assert!(!first.session_id.is_empty());
            assert!(first.total_tokens > 0);
            assert!(first.request_count > 0);
            assert!(!first.model_breakdown.is_empty());
        }
    }
}
