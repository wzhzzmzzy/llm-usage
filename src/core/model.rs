use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::core::adapter::ModelBreakdown;

/// Data source types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    All,
    Claude,
    Codex,
    Gemini,
    Opencode,
}

impl Source {
    pub fn as_str(&self) -> &'static str {
        match self {
            Source::All => "all",
            Source::Claude => "claude",
            Source::Codex => "codex",
            Source::Gemini => "gemini",
            Source::Opencode => "opencode",
        }
    }

    pub fn all_variants() -> &'static [Source] {
        &[Source::All, Source::Claude, Source::Codex, Source::Gemini, Source::Opencode]
    }
}

/// Report types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportType {
    Daily,
    Monthly,
    Session,
    Blocks,
}

impl ReportType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReportType::Daily => "daily",
            ReportType::Monthly => "monthly",
            ReportType::Session => "session",
            ReportType::Blocks => "blocks",
        }
    }

    pub fn all_variants() -> &'static [ReportType] {
        &[
            ReportType::Daily,
            ReportType::Monthly,
            ReportType::Session,
            ReportType::Blocks,
        ]
    }
}

/// Cell in the command matrix (source + report type combination)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Cell {
    pub source: Source,
    pub report: ReportType,
}

impl Cell {
    pub fn all_cells() -> Vec<Cell> {
        let mut cells = Vec::new();
        for source in Source::all_variants() {
            for report in ReportType::all_variants() {
                cells.push(Cell {
                    source: *source,
                    report: *report,
                });
            }
        }
        cells
    }
}

/// Usage metrics (common shape)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageMetric {
    pub total_tokens: u64,
    pub input_tokens: u64,
    pub cache_read_tokens: u64,
    pub output_tokens: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_breakdown: Option<Vec<ModelBreakdown>>,
}

/// Daily report row
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyRow {
    pub date: NaiveDate,
    pub total_tokens: u64,
    pub input_tokens: u64,
    pub cache_read_tokens: u64,
    pub output_tokens: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub models_used: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_breakdown: Option<Vec<ModelBreakdown>>,
}

/// Monthly report row
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthlyRow {
    pub month: String,
    pub total_tokens: u64,
    pub input_tokens: u64,
    pub cache_read_tokens: u64,
    pub output_tokens: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub models_used: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_breakdown: Option<Vec<ModelBreakdown>>,
}

/// Session report row
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionRow {
    pub session_id: String,
    pub project_path: Option<String>,
    pub total_tokens: u64,
    pub input_tokens: u64,
    pub cache_read_tokens: u64,
    pub output_tokens: u64,
    pub request_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_activity: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub models_used: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_breakdown: Option<Vec<ModelBreakdown>>,
}

/// Block report row
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockRow {
    pub block_id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub total_tokens: u64,
    pub input_tokens: u64,
    pub cache_read_tokens: u64,
    pub output_tokens: u64,
    pub is_active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub models_used: Option<Vec<String>>,
}

/// Report data containers
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyReport {
    pub days: Vec<DailyRow>,
    pub totals: UsageMetric,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthlyReport {
    pub months: Vec<MonthlyRow>,
    pub totals: UsageMetric,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionReport {
    pub sessions: Vec<SessionRow>,
    pub totals: UsageMetric,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlocksReport {
    pub blocks: Vec<BlockRow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_block: Option<BlockRow>,
    pub totals: UsageMetric,
}

/// Cell status in refresh
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CellStatus {
    Success,
    Stale,
    Error,
}

/// Cell result after refresh
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CellResult {
    pub cell: Cell,
    pub status: CellStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
}

/// Overall snapshot status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SnapshotStatus {
    Success,
    Partial,
    Error,
    NoData,
}

/// The main snapshot response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub status: SnapshotStatus,
    pub last_refresh: Option<DateTime<Utc>>,
    pub last_success: Option<DateTime<Utc>>,
    pub last_error: Option<DateTime<Utc>>,
    pub cells: HashMap<String, CellResult>,
    pub daily: HashMap<String, DailyReport>,
    pub monthly: HashMap<String, MonthlyReport>,
    pub session: HashMap<String, SessionReport>,
    pub blocks: HashMap<String, BlocksReport>,
    pub timezone: String,
}

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            status: SnapshotStatus::NoData,
            last_refresh: None,
            last_success: None,
            last_error: None,
            cells: HashMap::new(),
            daily: HashMap::new(),
            monthly: HashMap::new(),
            session: HashMap::new(),
            blocks: HashMap::new(),
            timezone: "UTC".to_string(),
        }
    }
}

impl Snapshot {
    pub fn cell_key(source: Source, report: ReportType) -> String {
        format!("{}_{}", source.as_str(), report.as_str())
    }
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    pub status: HealthStatus,
    pub runner: RunnerStatus,
    pub ccusage: CcUsageStatus,
    pub config_path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunnerStatus {
    pub found: bool,
    pub path: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CcUsageStatus {
    pub available: bool,
    pub version: Option<String>,
}

/// Refresh response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshResponse {
    pub status: SnapshotStatus,
    pub snapshot: Snapshot,
}
