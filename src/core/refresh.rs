use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration};

use crate::config::AppConfig;
use crate::core::error::RefreshError;
use crate::core::model::*;
use crate::core::provider::{NativeUsageProvider, RawBlockRow, RawDailyRow, RawMonthlyRow, RawSessionRow};

pub const MAX_CONCURRENT_COMMANDS: usize = 8;
pub const COMMAND_TIMEOUT_SECS: u64 = 15;
pub const REFRESH_TIMEOUT_SECS: u64 = 60;

pub struct RefreshManager {
    provider: Arc<NativeUsageProvider>,
    snapshot: Arc<Mutex<Snapshot>>,
}

impl RefreshManager {
    pub fn new(_config: AppConfig) -> Self {
        Self {
            provider: Arc::new(NativeUsageProvider::new()),
            snapshot: Arc::new(Mutex::new(Snapshot::default())),
        }
    }

    pub async fn get_snapshot(&self) -> Snapshot {
        self.snapshot.lock().await.clone()
    }

    pub async fn refresh(&self) -> Result<RefreshResponse, RefreshError> {
        self.execute_refresh().await
    }

    async fn execute_refresh(&self) -> Result<RefreshResponse, RefreshError> {
        self.provider.preload().await;

        let cells = self.plan_cells();
        if cells.is_empty() {
            let mut snapshot = self.snapshot.lock().await.clone();
            snapshot.status = SnapshotStatus::NoData;
            snapshot.last_refresh = Some(chrono::Utc::now());
            *self.snapshot.lock().await = snapshot.clone();
            return Ok(RefreshResponse {
                status: SnapshotStatus::NoData,
                snapshot,
            });
        }

        let semaphore = Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_COMMANDS));
        let mut handles = Vec::new();

        for cell in cells {
            let provider = self.provider.clone();
            let sem = semaphore.clone();

            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();

                let result = timeout(
                    Duration::from_secs(COMMAND_TIMEOUT_SECS),
                    provider.execute_cell(cell),
                )
                .await;

                match result {
                    Ok(Ok(json)) => (cell, Ok(json)),
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

        let mut snapshot = self.snapshot.lock().await.clone();
        let mut success_count = 0;
        let mut error_count = 0;

        for result in results {
            match result {
                Ok((cell, Ok(json))) => match self.process_cell(&mut snapshot, cell, &json) {
                    Ok(()) => success_count += 1,
                    Err(e) => {
                        self.mark_cell_error(&mut snapshot, cell, &e.to_string());
                        error_count += 1;
                    }
                },
                Ok((cell, Err(e))) => {
                    self.mark_cell_error(&mut snapshot, cell, &e.to_string());
                    error_count += 1;
                }
                Err(e) => {
                    tracing::error!("Task join error: {}", e);
                    error_count += 1;
                }
            }
        }

        let now = chrono::Utc::now();
        snapshot.last_refresh = Some(now);

        if success_count > 0 && error_count == 0 {
            snapshot.status = SnapshotStatus::Success;
            snapshot.last_success = Some(now);
        } else if success_count > 0 {
            snapshot.status = SnapshotStatus::Partial;
            snapshot.last_success = Some(now);
        } else if snapshot.daily.is_empty() {
            snapshot.status = SnapshotStatus::NoData;
            snapshot.last_error = Some(now);
        } else {
            snapshot.status = SnapshotStatus::Partial;
            snapshot.last_error = Some(now);
        }

        let status = snapshot.status;
        *self.snapshot.lock().await = snapshot.clone();

        Ok(RefreshResponse {
            status,
            snapshot,
        })
    }

    /// Plan which cells to execute. Skip sources with no data directories.
    fn plan_cells(&self) -> Vec<Cell> {
        let mut cells = Vec::new();
        let mut has_any_source = false;

        for source in [Source::Claude, Source::Codex, Source::Opencode] {
            if self.provider.source_has_data(source) {
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
        &self,
        snapshot: &mut Snapshot,
        cell: Cell,
        json: &str,
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

        #[derive(serde::Deserialize)]
        struct DailyWrapper {
            daily: Vec<RawDailyRow>,
        }

        #[derive(serde::Deserialize)]
        struct MonthlyWrapper {
            monthly: Vec<RawMonthlyRow>,
        }

        #[derive(serde::Deserialize)]
        struct SessionWrapper {
            #[serde(alias = "sessions")]
            session: Vec<RawSessionRow>,
        }

        #[derive(serde::Deserialize)]
        struct BlocksWrapper {
            blocks: Vec<RawBlockRow>,
        }

        match cell.report {
            ReportType::Daily => {
                let wrapper: DailyWrapper = serde_json::from_str(json)?;
                let report = crate::core::normalize::Normalizer::normalize_daily(&wrapper.daily)?;
                snapshot
                    .daily
                    .insert(Snapshot::cell_key(cell.source, cell.report), report);
            }
            ReportType::Monthly => {
                let wrapper: MonthlyWrapper = serde_json::from_str(json)?;
                let report = crate::core::normalize::Normalizer::normalize_monthly(&wrapper.monthly)?;
                snapshot
                    .monthly
                    .insert(Snapshot::cell_key(cell.source, cell.report), report);
            }
            ReportType::Session => {
                let wrapper: SessionWrapper = serde_json::from_str(json)?;
                let report = crate::core::normalize::Normalizer::normalize_session(&wrapper.session)?;
                snapshot
                    .session
                    .insert(Snapshot::cell_key(cell.source, cell.report), report);
            }
            ReportType::Blocks => {
                let wrapper: BlocksWrapper = serde_json::from_str(json)?;
                let report = crate::core::normalize::Normalizer::normalize_blocks(&wrapper.blocks)?;
                snapshot
                    .blocks
                    .insert(Snapshot::cell_key(cell.source, cell.report), report);
            }
        }

        Ok(())
    }

    fn mark_cell_error(&self, snapshot: &mut Snapshot, cell: Cell, error: &str) {
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
