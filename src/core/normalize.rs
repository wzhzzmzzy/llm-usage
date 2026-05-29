use chrono::NaiveDate;

use crate::core::adapter::ModelBreakdown;
use crate::core::error::NormalizeError;
use crate::core::model::*;

/// Raw daily aggregate from provider JSON
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawDailyAggregate {
    pub date: String,
    #[serde(default)]
    pub total_tokens: u64,
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub cache_read_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub request_count: Option<u64>,
    #[serde(default)]
    pub models_used: Option<Vec<String>>,
    #[serde(default)]
    pub model_breakdown: Option<Vec<ModelBreakdown>>,
}

/// Raw monthly aggregate from provider JSON
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawMonthlyAggregate {
    pub month: String,
    #[serde(default)]
    pub total_tokens: u64,
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub cache_read_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub request_count: Option<u64>,
    #[serde(default)]
    pub models_used: Option<Vec<String>>,
    #[serde(default)]
    pub model_breakdown: Option<Vec<ModelBreakdown>>,
}

/// Raw session aggregate from provider JSON
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawSessionAggregate {
    pub session_id: String,
    #[serde(default)]
    pub project_path: Option<String>,
    #[serde(default)]
    pub total_tokens: u64,
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub cache_read_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub request_count: Option<u64>,
    #[serde(default)]
    pub last_activity: Option<String>,
    #[serde(default)]
    pub models_used: Option<Vec<String>>,
    #[serde(default)]
    pub model_breakdown: Option<Vec<ModelBreakdown>>,
}

/// Raw block aggregate from provider JSON
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawBlockAggregate {
    pub block_id: String,
    #[serde(default)]
    pub start_time: String,
    #[serde(default)]
    pub end_time: String,
    #[serde(default)]
    pub actual_end_time: Option<String>,
    #[serde(default)]
    pub is_active: bool,
    #[serde(default)]
    pub total_tokens: u64,
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub cache_read_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub request_count: Option<u64>,
    #[serde(default)]
    pub models_used: Option<Vec<String>>,
    #[serde(default)]
    pub model_breakdown: Option<Vec<ModelBreakdown>>,
}

pub struct Normalizer;

impl Normalizer {
    pub fn normalize_daily(rows: &[RawDailyAggregate]) -> Result<DailyReport, NormalizeError> {
        let mut days = Vec::new();
        let mut total_tokens = 0u64;
        let mut total_input = 0u64;
        let mut total_cache_read = 0u64;
        let mut total_output = 0u64;
        let mut total_requests = 0u64;

        for row in rows {
            let date = NaiveDate::parse_from_str(&row.date, "%Y-%m-%d")
                .map_err(|e| NormalizeError::ParseError(format!("Invalid date '{}': {}", row.date, e)))?;

            total_tokens += row.total_tokens;
            total_input += row.input_tokens;
            total_cache_read += row.cache_read_tokens;
            total_output += row.output_tokens;
            total_requests += row.request_count.unwrap_or(0);

            days.push(DailyRow {
                date,
                total_tokens: row.total_tokens,
                input_tokens: row.input_tokens,
                cache_read_tokens: row.cache_read_tokens,
                output_tokens: row.output_tokens,
                request_count: row.request_count,
                models_used: row.models_used.clone(),
                model_breakdown: row.model_breakdown.clone(),
            });
        }

        days.sort_by(|a, b| b.date.cmp(&a.date));

        Ok(DailyReport {
            days,
            totals: UsageMetric {
                total_tokens,
                input_tokens: total_input,
                cache_read_tokens: total_cache_read,
                output_tokens: total_output,
                request_count: Some(total_requests),
                model_breakdown: None,
            },
        })
    }

    pub fn normalize_monthly(rows: &[RawMonthlyAggregate]) -> Result<MonthlyReport, NormalizeError> {
        let mut months = Vec::new();
        let mut total_tokens = 0u64;
        let mut total_input = 0u64;
        let mut total_cache_read = 0u64;
        let mut total_output = 0u64;
        let mut total_requests = 0u64;

        for row in rows {
            total_tokens += row.total_tokens;
            total_input += row.input_tokens;
            total_cache_read += row.cache_read_tokens;
            total_output += row.output_tokens;
            total_requests += row.request_count.unwrap_or(0);

            months.push(MonthlyRow {
                month: row.month.clone(),
                total_tokens: row.total_tokens,
                input_tokens: row.input_tokens,
                cache_read_tokens: row.cache_read_tokens,
                output_tokens: row.output_tokens,
                request_count: row.request_count,
                models_used: row.models_used.clone(),
                model_breakdown: row.model_breakdown.clone(),
            });
        }

        months.sort_by(|a, b| b.month.cmp(&a.month));

        Ok(MonthlyReport {
            months,
            totals: UsageMetric {
                total_tokens,
                input_tokens: total_input,
                cache_read_tokens: total_cache_read,
                output_tokens: total_output,
                request_count: Some(total_requests),
                model_breakdown: None,
            },
        })
    }

    pub fn normalize_session(rows: &[RawSessionAggregate]) -> Result<SessionReport, NormalizeError> {
        let mut sessions = Vec::new();
        let mut total_tokens = 0u64;
        let mut total_input = 0u64;
        let mut total_cache_read = 0u64;
        let mut total_output = 0u64;
        let mut total_requests = 0u64;

        for row in rows {
            let last_activity = row
                .last_activity
                .as_ref()
                .and_then(|s| {
                    chrono::DateTime::parse_from_rfc3339(s)
                        .ok()
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                });

            total_tokens += row.total_tokens;
            total_input += row.input_tokens;
            total_cache_read += row.cache_read_tokens;
            total_output += row.output_tokens;
            total_requests += row.request_count.unwrap_or(0);

            sessions.push(SessionRow {
                session_id: row.session_id.clone(),
                project_path: row.project_path.clone(),
                total_tokens: row.total_tokens,
                input_tokens: row.input_tokens,
                cache_read_tokens: row.cache_read_tokens,
                output_tokens: row.output_tokens,
                request_count: row.request_count.unwrap_or(0),
                last_activity,
                models_used: row.models_used.clone(),
                model_breakdown: row.model_breakdown.clone(),
            });
        }

        sessions.sort_by(|a, b| {
            b.last_activity
                .unwrap_or_default()
                .cmp(&a.last_activity.unwrap_or_default())
        });

        Ok(SessionReport {
            sessions,
            totals: UsageMetric {
                total_tokens,
                input_tokens: total_input,
                cache_read_tokens: total_cache_read,
                output_tokens: total_output,
                request_count: Some(total_requests),
                model_breakdown: None,
            },
        })
    }

    pub fn normalize_blocks(rows: &[RawBlockAggregate]) -> Result<BlocksReport, NormalizeError> {
        let mut blocks = Vec::new();
        let mut total_tokens = 0u64;
        let mut total_input = 0u64;
        let mut total_cache_read = 0u64;
        let mut total_output = 0u64;
        let mut total_requests = 0u64;

        for row in rows {
            let start_time = chrono::DateTime::parse_from_rfc3339(&row.start_time)
                .ok()
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_default();

            let end_time = chrono::DateTime::parse_from_rfc3339(&row.end_time)
                .ok()
                .map(|dt| dt.with_timezone(&chrono::Utc));

            total_tokens += row.total_tokens;
            total_input += row.input_tokens;
            total_cache_read += row.cache_read_tokens;
            total_output += row.output_tokens;
            total_requests += row.request_count.unwrap_or(0);

            blocks.push(BlockRow {
                block_id: row.block_id.clone(),
                start_time,
                end_time,
                total_tokens: row.total_tokens,
                input_tokens: row.input_tokens,
                cache_read_tokens: row.cache_read_tokens,
                output_tokens: row.output_tokens,
                is_active: row.is_active,
                models_used: row.models_used.clone(),
            });
        }

        blocks.sort_by(|a, b| b.start_time.cmp(&a.start_time));

        let active_block = blocks.iter().find(|b| b.is_active).cloned();

        Ok(BlocksReport {
            blocks,
            active_block,
            totals: UsageMetric {
                total_tokens,
                input_tokens: total_input,
                cache_read_tokens: total_cache_read,
                output_tokens: total_output,
                request_count: Some(total_requests),
                model_breakdown: None,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_daily() {
        let rows = vec![RawDailyAggregate {
            date: "2026-05-29".to_string(),
            total_tokens: 1500,
            input_tokens: 1000,
            cache_read_tokens: 200,
            output_tokens: 300,
            request_count: Some(5),
            models_used: Some(vec!["gpt-5.5".to_string()]),
            model_breakdown: Some(vec![ModelBreakdown {
                model: "gpt-5.5".to_string(),
                input_tokens: 1000,
                cache_read_tokens: 200,
                output_tokens: 300,
                total_tokens: 1500,
                request_count: 5,
            }]),
        }];

        let report = Normalizer::normalize_daily(&rows).unwrap();
        assert_eq!(report.days.len(), 1);
        assert_eq!(report.days[0].total_tokens, 1500);
        assert_eq!(report.days[0].input_tokens, 1000);
        assert_eq!(report.days[0].cache_read_tokens, 200);
        assert_eq!(report.days[0].output_tokens, 300);
        assert_eq!(report.days[0].request_count, Some(5));
        assert_eq!(report.days[0].model_breakdown.as_ref().unwrap().len(), 1);

        assert_eq!(report.totals.total_tokens, 1500);
        assert_eq!(report.totals.input_tokens, 1000);
        assert_eq!(report.totals.cache_read_tokens, 200);
        assert_eq!(report.totals.output_tokens, 300);
        assert_eq!(report.totals.request_count, Some(5));
    }

    #[test]
    fn test_normalize_monthly() {
        let rows = vec![RawMonthlyAggregate {
            month: "2026-05".to_string(),
            total_tokens: 5000,
            input_tokens: 3000,
            cache_read_tokens: 500,
            output_tokens: 1500,
            request_count: Some(20),
            models_used: Some(vec!["gpt-5.5".to_string(), "claude-sonnet-4".to_string()]),
            model_breakdown: Some(vec![
                ModelBreakdown {
                    model: "gpt-5.5".to_string(),
                    input_tokens: 2000,
                    cache_read_tokens: 300,
                    output_tokens: 1000,
                    total_tokens: 3300,
                    request_count: 12,
                },
                ModelBreakdown {
                    model: "claude-sonnet-4".to_string(),
                    input_tokens: 1000,
                    cache_read_tokens: 200,
                    output_tokens: 500,
                    total_tokens: 1700,
                    request_count: 8,
                },
            ]),
        }];

        let report = Normalizer::normalize_monthly(&rows).unwrap();
        assert_eq!(report.months.len(), 1);
        assert_eq!(report.months[0].total_tokens, 5000);
        assert_eq!(report.months[0].model_breakdown.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_normalize_session() {
        let rows = vec![RawSessionAggregate {
            session_id: "test-session".to_string(),
            project_path: Some("/home/user/project".to_string()),
            total_tokens: 1500,
            input_tokens: 1000,
            cache_read_tokens: 200,
            output_tokens: 300,
            request_count: Some(5),
            last_activity: Some("2026-05-29T10:00:00Z".to_string()),
            models_used: Some(vec!["gpt-5.5".to_string()]),
            model_breakdown: Some(vec![ModelBreakdown {
                model: "gpt-5.5".to_string(),
                input_tokens: 1000,
                cache_read_tokens: 200,
                output_tokens: 300,
                total_tokens: 1500,
                request_count: 5,
            }]),
        }];

        let report = Normalizer::normalize_session(&rows).unwrap();
        assert_eq!(report.sessions.len(), 1);
        assert_eq!(report.sessions[0].session_id, "test-session");
        assert_eq!(report.sessions[0].total_tokens, 1500);
        assert_eq!(report.sessions[0].request_count, 5);
        assert!(report.sessions[0].last_activity.is_some());
    }

    #[test]
    fn test_daily_report_json_matches_frontend_types() {
        let rows = vec![RawDailyAggregate {
            date: "2026-05-29".to_string(),
            total_tokens: 1500,
            input_tokens: 1000,
            cache_read_tokens: 200,
            output_tokens: 300,
            request_count: Some(5),
            models_used: Some(vec!["gpt-5.5".to_string()]),
            model_breakdown: Some(vec![ModelBreakdown {
                model: "gpt-5.5".to_string(),
                input_tokens: 1000,
                cache_read_tokens: 200,
                output_tokens: 300,
                total_tokens: 1500,
                request_count: 5,
            }]),
        }];

        let report = Normalizer::normalize_daily(&rows).unwrap();
        let json = serde_json::to_value(&report).unwrap();

        assert!(json.get("days").is_some(), "Missing 'days' field");
        assert!(json.get("totals").is_some(), "Missing 'totals' field");

        let day = &json["days"][0];
        assert!(day.get("date").is_some(), "Missing day.date");
        assert!(day.get("totalTokens").is_some(), "Missing day.totalTokens");
        assert!(day.get("inputTokens").is_some(), "Missing day.inputTokens");
        assert!(day.get("cacheReadTokens").is_some(), "Missing day.cacheReadTokens");
        assert!(day.get("outputTokens").is_some(), "Missing day.outputTokens");
        assert!(day.get("requestCount").is_some(), "Missing day.requestCount");
        assert!(day.get("modelsUsed").is_some(), "Missing day.modelsUsed");
        assert!(day.get("modelBreakdown").is_some(), "Missing day.modelBreakdown");

        let totals = &json["totals"];
        assert!(totals.get("totalTokens").is_some(), "Missing totals.totalTokens");
        assert!(totals.get("inputTokens").is_some(), "Missing totals.inputTokens");
        assert!(totals.get("cacheReadTokens").is_some(), "Missing totals.cacheReadTokens");
        assert!(totals.get("outputTokens").is_some(), "Missing totals.outputTokens");
    }

    #[test]
    fn test_session_report_json_matches_frontend_types() {
        let rows = vec![RawSessionAggregate {
            session_id: "test-session".to_string(),
            project_path: Some("/home/user/project".to_string()),
            total_tokens: 1500,
            input_tokens: 1000,
            cache_read_tokens: 200,
            output_tokens: 300,
            request_count: Some(5),
            last_activity: Some("2026-05-29T10:00:00Z".to_string()),
            models_used: Some(vec!["gpt-5.5".to_string()]),
            model_breakdown: Some(vec![ModelBreakdown {
                model: "gpt-5.5".to_string(),
                input_tokens: 1000,
                cache_read_tokens: 200,
                output_tokens: 300,
                total_tokens: 1500,
                request_count: 5,
            }]),
        }];

        let report = Normalizer::normalize_session(&rows).unwrap();
        let json = serde_json::to_value(&report).unwrap();

        assert!(json.get("sessions").is_some(), "Missing 'sessions' field");

        let session = &json["sessions"][0];
        assert!(session.get("sessionId").is_some(), "Missing session.sessionId");
        assert!(session.get("projectPath").is_some(), "Missing session.projectPath");
        assert!(session.get("totalTokens").is_some(), "Missing session.totalTokens");
        assert!(session.get("inputTokens").is_some(), "Missing session.inputTokens");
        assert!(session.get("cacheReadTokens").is_some(), "Missing session.cacheReadTokens");
        assert!(session.get("outputTokens").is_some(), "Missing session.outputTokens");
        assert!(session.get("requestCount").is_some(), "Missing session.requestCount");
        assert!(session.get("lastActivity").is_some(), "Missing session.lastActivity");
        assert!(session.get("modelsUsed").is_some(), "Missing session.modelsUsed");
        assert!(session.get("modelBreakdown").is_some(), "Missing session.modelBreakdown");
    }

    #[test]
    fn test_snapshot_json_matches_frontend_types() {
        let snapshot = Snapshot::default();
        let json = serde_json::to_value(&snapshot).unwrap();

        assert!(json.get("status").is_some());
        assert!(json.get("lastRefresh").is_some());
        assert!(json.get("lastSuccess").is_some());
        assert!(json.get("lastError").is_some());
        assert!(json.get("cells").is_some());
        assert!(json.get("daily").is_some());
        assert!(json.get("monthly").is_some());
        assert!(json.get("session").is_some());
        assert!(json.get("blocks").is_some());
        assert!(json.get("timezone").is_some());
    }
}
