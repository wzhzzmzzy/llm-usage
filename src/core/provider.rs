use std::collections::HashMap;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::core::adapter::{claude::ClaudeAdapter, codex::CodexAdapter, gemini::GeminiAdapter, opencode::OpenCodeAdapter, UsageAdapter, UsageEntry};
use crate::core::model::*;

/// Cached entries for a single source (claude/codex/opencode)
struct SourceCache {
    entries: Vec<UsageEntry>,
}

/// Native usage provider with per-source entry caching
pub struct NativeUsageProvider {
    adapters: Vec<Box<dyn UsageAdapter>>,
    source_cache: RwLock<HashMap<Source, SourceCache>>,
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
            block_duration_hours,
        }
    }

    /// Pre-load all source data. Call once before executing cells.
    pub async fn preload(&self) {
        let start = std::time::Instant::now();

        let mut handles = Vec::new();
        for source in [Source::Claude, Source::Codex, Source::Gemini, Source::Opencode] {
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
                (source, entries, adapter_start.elapsed())
            }));
        }

        let mut cache = self.source_cache.write().await;
        for handle in handles {
            if let Ok((source, entries, elapsed)) = handle.await {
                tracing::info!("{:?} adapter: {} entries loaded, {:?}", source, entries.len(), elapsed);
                cache.insert(source, SourceCache { entries });
            }
        }

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

    pub async fn execute_cell(&self, cell: Cell) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let entries = if cell.source == Source::All {
            let cache = self.source_cache.read().await;
            let mut all = Vec::new();
            for cached in cache.values() {
                all.extend(cached.entries.clone());
            }
            all
        } else {
            let cache = self.source_cache.read().await;
            match cache.get(&cell.source) {
                Some(cached) => cached.entries.clone(),
                None => Vec::new(),
            }
        };

        let any_adapter = self.adapters.first()
            .ok_or("No adapters available")?;

        let json = match cell.report {
            ReportType::Daily => {
                let aggregates = any_adapter.aggregate_daily(&entries);
                serde_json::to_string(&aggregates)?
            }
            ReportType::Monthly => {
                let aggregates = any_adapter.aggregate_monthly(&entries);
                serde_json::to_string(&aggregates)?
            }
            ReportType::Session => {
                let aggregates = any_adapter.aggregate_session(&entries);
                serde_json::to_string(&aggregates)?
            }
            ReportType::Blocks => {
                let aggregates = any_adapter.aggregate_blocks(&entries, self.block_duration_hours);
                serde_json::to_string(&aggregates)?
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
    async fn test_execute_cell_daily_returns_json_array() {
        let provider = NativeUsageProvider::new();
        provider.preload().await;

        let cell = Cell { source: Source::Claude, report: ReportType::Daily };
        let json = provider.execute_cell(cell).await.unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert!(value.is_array());
    }

    #[tokio::test]
    async fn test_execute_cell_monthly_returns_json_array() {
        let provider = NativeUsageProvider::new();
        provider.preload().await;

        let cell = Cell { source: Source::Codex, report: ReportType::Monthly };
        let json = provider.execute_cell(cell).await.unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert!(value.is_array());
    }

    #[tokio::test]
    async fn test_execute_cell_session_returns_json_array() {
        let provider = NativeUsageProvider::new();
        provider.preload().await;

        let cell = Cell { source: Source::Opencode, report: ReportType::Session };
        let json = provider.execute_cell(cell).await.unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert!(value.is_array());
    }

    #[tokio::test]
    async fn test_daily_json_has_expected_fields() {
        let provider = NativeUsageProvider::new();
        provider.preload().await;

        let cell = Cell { source: Source::Claude, report: ReportType::Daily };
        let json = provider.execute_cell(cell).await.unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Should be an array
        assert!(value.is_array());

        // If there's data, check the first element has expected fields
        if let Some(first) = value.as_array().and_then(|a| a.first()) {
            assert!(first.get("date").is_some());
            assert!(first.get("totalTokens").is_some());
            assert!(first.get("inputTokens").is_some());
            assert!(first.get("cacheReadTokens").is_some());
            assert!(first.get("outputTokens").is_some());
            assert!(first.get("requestCount").is_some());
            assert!(first.get("modelsUsed").is_some());
            assert!(first.get("modelBreakdown").is_some());
        }
    }

    #[tokio::test]
    async fn test_session_json_has_expected_fields() {
        let provider = NativeUsageProvider::new();
        provider.preload().await;

        let cell = Cell { source: Source::Opencode, report: ReportType::Session };
        let json = provider.execute_cell(cell).await.unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert!(value.is_array());

        if let Some(first) = value.as_array().and_then(|a| a.first()) {
            assert!(first.get("sessionId").is_some());
            assert!(first.get("totalTokens").is_some());
            assert!(first.get("inputTokens").is_some());
            assert!(first.get("cacheReadTokens").is_some());
            assert!(first.get("outputTokens").is_some());
            assert!(first.get("requestCount").is_some());
            assert!(first.get("modelBreakdown").is_some());
        }
    }
}
