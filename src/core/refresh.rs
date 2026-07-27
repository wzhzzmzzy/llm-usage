use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{timeout, Duration};

use serde::{Deserialize, Serialize};

use crate::config::AppConfig;
use crate::core::error::RefreshError;
use crate::core::model::*;
use crate::core::normalize::Normalizer;

pub const MAX_CONCURRENT_COMMANDS: usize = 8;
pub const COMMAND_TIMEOUT_SECS: u64 = 15;
pub const REFRESH_TIMEOUT_SECS: u64 = 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshStatus {
    pub is_refreshing: bool,
    pub last_refresh: Option<chrono::DateTime<chrono::Utc>>,
    pub last_success: Option<chrono::DateTime<chrono::Utc>>,
    pub last_error: Option<String>,
    pub snapshot_status: SnapshotStatus,
}

pub struct RefreshManager {
    provider: Arc<NativeUsageProvider>,
    snapshot: Arc<Mutex<Snapshot>>,
    is_refreshing: Arc<RwLock<bool>>,
    last_error: Arc<Mutex<Option<String>>>,
}

use crate::core::provider::{CellAggregates, NativeUsageProvider};

impl RefreshManager {
    pub fn new(config: AppConfig) -> Self {
        Self {
            provider: Arc::new(NativeUsageProvider::with_block_duration(config.block_duration_hours)),
            snapshot: Arc::new(Mutex::new(Snapshot::default())),
            is_refreshing: Arc::new(RwLock::new(false)),
            last_error: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn get_snapshot(&self) -> Snapshot {
        self.snapshot.lock().await.clone()
    }

    pub async fn get_refresh_status(&self) -> RefreshStatus {
        let is_refreshing = *self.is_refreshing.read().await;
        let snapshot = self.snapshot.lock().await.clone();
        let last_error = self.last_error.lock().await.clone();

        RefreshStatus {
            is_refreshing,
            last_refresh: snapshot.last_refresh,
            last_success: snapshot.last_success,
            last_error,
            snapshot_status: snapshot.status,
        }
    }

    /// Start a background refresh. Returns immediately.
    pub async fn start_refresh(&self) -> Result<RefreshStatus, RefreshError> {
        {
            let refreshing = self.is_refreshing.read().await;
            if *refreshing {
                return Ok(self.get_refresh_status().await);
            }
        }

        {
            let mut refreshing = self.is_refreshing.write().await;
            *refreshing = true;
        }

        let provider = self.provider.clone();
        let snapshot = self.snapshot.clone();
        let is_refreshing = self.is_refreshing.clone();
        let last_error = self.last_error.clone();

        tokio::spawn(async move {
            let result = Self::execute_refresh_inner(provider.clone(), snapshot.clone()).await;

            let mut error_guard = last_error.lock().await;
            match result {
                Ok(_) => {
                    *error_guard = None;
                }
                Err(e) => {
                    *error_guard = Some(e.to_string());
                }
            }

            let mut refreshing = is_refreshing.write().await;
            *refreshing = false;
        });

        Ok(self.get_refresh_status().await)
    }

    /// Synchronous refresh (used by CLI)
    pub async fn refresh(&self) -> Result<RefreshResponse, RefreshError> {
        let result = Self::execute_refresh_inner(self.provider.clone(), self.snapshot.clone()).await?;

        let mut error_guard = self.last_error.lock().await;
        *error_guard = None;

        Ok(result)
    }

    async fn execute_refresh_inner(
        provider: Arc<NativeUsageProvider>,
        snapshot: Arc<Mutex<Snapshot>>,
    ) -> Result<RefreshResponse, RefreshError> {
        provider.preload().await;

        let cells = Self::plan_cells(&provider);
        if cells.is_empty() {
            let mut snapshot_guard = snapshot.lock().await;
            snapshot_guard.status = SnapshotStatus::NoData;
            snapshot_guard.last_refresh = Some(chrono::Utc::now());
            let response = RefreshResponse {
                status: SnapshotStatus::NoData,
                snapshot: snapshot_guard.clone(),
            };
            return Ok(response);
        }

        let semaphore = Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_COMMANDS));
        let mut handles = Vec::new();

        for cell in cells {
            let provider = provider.clone();
            let sem = semaphore.clone();

            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();

                let result = timeout(
                    Duration::from_secs(COMMAND_TIMEOUT_SECS),
                    provider.execute_cell(cell),
                )
                .await;

                match result {
                    Ok(Ok(data)) => (cell, Ok(data)),
                    Ok(Err(e)) => (cell, Err(e)),
                    Err(_) => (cell, Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        format!("Timeout after {} seconds", COMMAND_TIMEOUT_SECS),
                    )) as Box<dyn std::error::Error + Send + Sync>)),
                }
            }));
        }

        let overall_timeout = timeout(
            Duration::from_secs(REFRESH_TIMEOUT_SECS),
            futures::future::join_all(handles),
        )
        .await;

        let results = match overall_timeout {
            Ok(r) => r,
            Err(_) => return Err(RefreshError::Timeout(REFRESH_TIMEOUT_SECS)),
        };

        let mut snapshot_guard = snapshot.lock().await;
        let mut success_count = 0;
        let mut error_count = 0;

        for result in results {
            match result {
                Ok((cell, Ok(data))) => match Self::process_cell(&mut snapshot_guard, cell, data) {
                    Ok(()) => success_count += 1,
                    Err(e) => {
                        Self::mark_cell_error(&mut snapshot_guard, cell, &e.to_string());
                        error_count += 1;
                    }
                },
                Ok((cell, Err(e))) => {
                    Self::mark_cell_error(&mut snapshot_guard, cell, &e.to_string());
                    error_count += 1;
                }
                Err(e) => {
                    tracing::error!("Task join error: {}", e);
                    error_count += 1;
                }
            }
        }

        let now = chrono::Utc::now();
        snapshot_guard.last_refresh = Some(now);

        if success_count > 0 && error_count == 0 {
            snapshot_guard.status = SnapshotStatus::Success;
            snapshot_guard.last_success = Some(now);
        } else if success_count > 0 {
            snapshot_guard.status = SnapshotStatus::Partial;
            snapshot_guard.last_success = Some(now);
        } else if snapshot_guard.daily.is_empty() {
            snapshot_guard.status = SnapshotStatus::NoData;
            snapshot_guard.last_error = Some(now);
        } else {
            snapshot_guard.status = SnapshotStatus::Partial;
            snapshot_guard.last_error = Some(now);
        }

        let status = snapshot_guard.status;
        let response = RefreshResponse {
            status,
            snapshot: snapshot_guard.clone(),
        };

        Ok(response)
    }

    fn plan_cells(provider: &NativeUsageProvider) -> Vec<Cell> {
        let mut cells = Vec::new();
        let mut has_any_source = false;

        for source in [Source::Claude, Source::Codex, Source::Gemini, Source::Opencode] {
            if provider.source_has_data(source) {
                has_any_source = true;
                for report in ReportType::all_variants() {
                    cells.push(Cell {
                        source,
                        report: *report,
                    });
                }
            }
        }

        if has_any_source {
            for report in ReportType::all_variants() {
                cells.push(Cell {
                    source: Source::All,
                    report: *report,
                });
            }
        }

        cells
    }

    fn process_cell(
        snapshot: &mut Snapshot,
        cell: Cell,
        data: CellAggregates,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let key = Snapshot::cell_key(cell.source, cell.report);

        let cell_result = CellResult {
            cell,
            status: CellStatus::Success,
            error: None,
            stderr: None,
            exit_code: None,
            duration_ms: 0,
        };
        snapshot.cells.insert(key.clone(), cell_result);

        match (cell.report, data) {
            (ReportType::Daily, CellAggregates::Daily(rows)) => {
                let report = Normalizer::normalize_daily(&rows)?;
                snapshot.daily.insert(key, report);
            }
            (ReportType::Monthly, CellAggregates::Monthly(rows)) => {
                let report = Normalizer::normalize_monthly(&rows)?;
                snapshot.monthly.insert(key, report);
            }
            (ReportType::Session, CellAggregates::Session(rows)) => {
                let report = Normalizer::normalize_session(&rows)?;
                snapshot.session.insert(key, report);
            }
            (ReportType::Blocks, CellAggregates::Blocks(rows)) => {
                let report = Normalizer::normalize_blocks(&rows)?;
                snapshot.blocks.insert(key, report);
            }
            (report, _) => {
                return Err(format!("cell {:?} returned mismatched aggregates", report).into());
            }
        }

        Ok(())
    }

    fn mark_cell_error(snapshot: &mut Snapshot, cell: Cell, error: &str) {
        let key = Snapshot::cell_key(cell.source, cell.report);

        let prev_status = snapshot
            .cells
            .get(&key)
            .map(|c| c.status)
            .unwrap_or(CellStatus::Error);

        let status = if prev_status == CellStatus::Success {
            CellStatus::Stale
        } else {
            CellStatus::Error
        };

        snapshot.cells.insert(
            key,
            CellResult {
                cell,
                status,
                error: Some(error.to_string()),
                stderr: None,
                exit_code: None,
                duration_ms: 0,
            },
        );
    }

    pub async fn health_check(&self) -> HealthResponse {
        let provider_health = self.provider.health_check().await;

        let status = if provider_health.runner_found && provider_health.ccusage_available {
            HealthStatus::Healthy
        } else if provider_health.runner_found {
            HealthStatus::Degraded
        } else {
            HealthStatus::Unhealthy
        };

        HealthResponse {
            status,
            runner: RunnerStatus {
                found: provider_health.runner_found,
                path: provider_health.runner_path,
                version: provider_health.runner_version,
            },
            ccusage: CcUsageStatus {
                available: provider_health.ccusage_available,
                version: provider_health.ccusage_version,
            },
            config_path: AppConfig::config_path_display(),
        }
    }
}
