use chrono::NaiveDate;

use crate::core::adapter::{BlockAggregate, DailyAggregate, MonthlyAggregate, SessionAggregate};
use crate::core::error::NormalizeError;
use crate::core::model::*;

pub struct Normalizer;

impl Normalizer {
    pub fn normalize_daily(rows: &[DailyAggregate]) -> Result<DailyReport, NormalizeError> {
        let mut days = Vec::new();
        let mut total_tokens = 0u64;
        let mut total_input = 0u64;
        let mut total_cache_read = 0u64;
        let mut total_output = 0u64;
        let mut total_reasoning = 0u64;
        let mut total_requests = 0u64;

        for row in rows {
            let date = NaiveDate::parse_from_str(&row.date, "%Y-%m-%d")
                .map_err(|e| NormalizeError::ParseError(format!("Invalid date '{}': {}", row.date, e)))?;

            total_tokens += row.total_tokens;
            total_input += row.input_tokens;
            total_cache_read += row.cache_read_tokens;
            total_output += row.output_tokens;
            total_reasoning += row.reasoning_tokens;
            total_requests += row.request_count;

            days.push(DailyRow {
                date,
                total_tokens: row.total_tokens,
                input_tokens: row.input_tokens,
                cache_read_tokens: row.cache_read_tokens,
                output_tokens: row.output_tokens,
                reasoning_tokens: row.reasoning_tokens,
                request_count: Some(row.request_count),
                models_used: Some(row.models_used.clone()),
                model_breakdown: Some(row.model_breakdown.clone()),
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
                reasoning_tokens: total_reasoning,
                request_count: Some(total_requests),
                model_breakdown: None,
            },
        })
    }

    pub fn normalize_monthly(rows: &[MonthlyAggregate]) -> Result<MonthlyReport, NormalizeError> {
        let mut months = Vec::new();
        let mut total_tokens = 0u64;
        let mut total_input = 0u64;
        let mut total_cache_read = 0u64;
        let mut total_output = 0u64;
        let mut total_reasoning = 0u64;
        let mut total_requests = 0u64;

        for row in rows {
            total_tokens += row.total_tokens;
            total_input += row.input_tokens;
            total_cache_read += row.cache_read_tokens;
            total_output += row.output_tokens;
            total_reasoning += row.reasoning_tokens;
            total_requests += row.request_count;

            months.push(MonthlyRow {
                month: row.month.clone(),
                total_tokens: row.total_tokens,
                input_tokens: row.input_tokens,
                cache_read_tokens: row.cache_read_tokens,
                output_tokens: row.output_tokens,
                reasoning_tokens: row.reasoning_tokens,
                request_count: Some(row.request_count),
                models_used: Some(row.models_used.clone()),
                model_breakdown: Some(row.model_breakdown.clone()),
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
                reasoning_tokens: total_reasoning,
                request_count: Some(total_requests),
                model_breakdown: None,
            },
        })
    }

    pub fn normalize_session(rows: &[SessionAggregate]) -> Result<SessionReport, NormalizeError> {
        let mut sessions = Vec::new();
        let mut total_tokens = 0u64;
        let mut total_input = 0u64;
        let mut total_cache_read = 0u64;
        let mut total_output = 0u64;
        let mut total_reasoning = 0u64;
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
            total_reasoning += row.reasoning_tokens;
            total_requests += row.request_count;

            sessions.push(SessionRow {
                session_id: row.session_id.clone(),
                project_path: row.project_path.clone(),
                total_tokens: row.total_tokens,
                input_tokens: row.input_tokens,
                cache_read_tokens: row.cache_read_tokens,
                output_tokens: row.output_tokens,
                reasoning_tokens: row.reasoning_tokens,
                request_count: row.request_count,
                last_activity,
                models_used: Some(row.models_used.clone()),
                model_breakdown: Some(row.model_breakdown.clone()),
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
                reasoning_tokens: total_reasoning,
                request_count: Some(total_requests),
                model_breakdown: None,
            },
        })
    }

    pub fn normalize_blocks(rows: &[BlockAggregate]) -> Result<BlocksReport, NormalizeError> {
        let mut blocks = Vec::new();
        let mut total_tokens = 0u64;
        let mut total_input = 0u64;
        let mut total_cache_read = 0u64;
        let mut total_output = 0u64;
        let mut total_reasoning = 0u64;
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
            total_reasoning += row.reasoning_tokens;
            total_requests += row.request_count;

            blocks.push(BlockRow {
                block_id: row.block_id.clone(),
                start_time,
                end_time,
                total_tokens: row.total_tokens,
                input_tokens: row.input_tokens,
                cache_read_tokens: row.cache_read_tokens,
                output_tokens: row.output_tokens,
                reasoning_tokens: row.reasoning_tokens,
                is_active: row.is_active,
                models_used: Some(row.models_used.clone()),
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
                reasoning_tokens: total_reasoning,
                request_count: Some(total_requests),
                model_breakdown: None,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::adapter::ModelBreakdown;

    #[test]
    fn test_normalize_daily() {
        let rows = vec![DailyAggregate {
            date: "2026-05-29".to_string(),
            total_tokens: 1500,
            input_tokens: 1000,
            cache_read_tokens: 200,
            output_tokens: 300,
            reasoning_tokens: 50,
            request_count: 5,
            models_used: vec!["gpt-5.5".to_string()],
            model_breakdown: vec![ModelBreakdown {
                model: "gpt-5.5".to_string(),
                input_tokens: 1000,
                cache_read_tokens: 200,
                output_tokens: 300,
                reasoning_tokens: 50,
                total_tokens: 1500,
                request_count: 5,
            }],
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
        let rows = vec![MonthlyAggregate {
            month: "2026-05".to_string(),
            total_tokens: 5000,
            input_tokens: 3000,
            cache_read_tokens: 500,
            output_tokens: 1500,
            reasoning_tokens: 200,
            request_count: 20,
            models_used: vec!["gpt-5.5".to_string(), "claude-sonnet-4".to_string()],
            model_breakdown: vec![
                ModelBreakdown {
                    model: "gpt-5.5".to_string(),
                    input_tokens: 2000,
                    cache_read_tokens: 300,
                    output_tokens: 1000,
                    reasoning_tokens: 120,
                    total_tokens: 3300,
                    request_count: 12,
                },
                ModelBreakdown {
                    model: "claude-sonnet-4".to_string(),
                    input_tokens: 1000,
                    cache_read_tokens: 200,
                    output_tokens: 500,
                    reasoning_tokens: 80,
                    total_tokens: 1700,
                    request_count: 8,
                },
            ],
        }];

        let report = Normalizer::normalize_monthly(&rows).unwrap();
        assert_eq!(report.months.len(), 1);
        assert_eq!(report.months[0].total_tokens, 5000);
        assert_eq!(report.months[0].model_breakdown.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_normalize_session() {
        let rows = vec![SessionAggregate {
            session_id: "test-session".to_string(),
            project_path: Some("/home/user/project".to_string()),
            total_tokens: 1500,
            input_tokens: 1000,
            cache_read_tokens: 200,
            output_tokens: 300,
            reasoning_tokens: 50,
            request_count: 5,
            last_activity: Some("2026-05-29T10:00:00Z".to_string()),
            models_used: vec!["gpt-5.5".to_string()],
            model_breakdown: vec![ModelBreakdown {
                model: "gpt-5.5".to_string(),
                input_tokens: 1000,
                cache_read_tokens: 200,
                output_tokens: 300,
                reasoning_tokens: 50,
                total_tokens: 1500,
                request_count: 5,
            }],
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
        let rows = vec![DailyAggregate {
            date: "2026-05-29".to_string(),
            total_tokens: 1500,
            input_tokens: 1000,
            cache_read_tokens: 200,
            output_tokens: 300,
            reasoning_tokens: 50,
            request_count: 5,
            models_used: vec!["gpt-5.5".to_string()],
            model_breakdown: vec![ModelBreakdown {
                model: "gpt-5.5".to_string(),
                input_tokens: 1000,
                cache_read_tokens: 200,
                output_tokens: 300,
                reasoning_tokens: 50,
                total_tokens: 1500,
                request_count: 5,
            }],
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
        assert!(day.get("reasoningTokens").is_some(), "Missing day.reasoningTokens");
        assert!(day.get("requestCount").is_some(), "Missing day.requestCount");
        assert!(day.get("modelsUsed").is_some(), "Missing day.modelsUsed");
        assert!(day.get("modelBreakdown").is_some(), "Missing day.modelBreakdown");

        let totals = &json["totals"];
        assert!(totals.get("totalTokens").is_some(), "Missing totals.totalTokens");
        assert!(totals.get("inputTokens").is_some(), "Missing totals.inputTokens");
        assert!(totals.get("cacheReadTokens").is_some(), "Missing totals.cacheReadTokens");
        assert!(totals.get("outputTokens").is_some(), "Missing totals.outputTokens");
        assert!(totals.get("reasoningTokens").is_some(), "Missing totals.reasoningTokens");
    }

    #[test]
    fn test_session_report_json_matches_frontend_types() {
        let rows = vec![SessionAggregate {
            session_id: "test-session".to_string(),
            project_path: Some("/home/user/project".to_string()),
            total_tokens: 1500,
            input_tokens: 1000,
            cache_read_tokens: 200,
            output_tokens: 300,
            reasoning_tokens: 50,
            request_count: 5,
            last_activity: Some("2026-05-29T10:00:00Z".to_string()),
            models_used: vec!["gpt-5.5".to_string()],
            model_breakdown: vec![ModelBreakdown {
                model: "gpt-5.5".to_string(),
                input_tokens: 1000,
                cache_read_tokens: 200,
                output_tokens: 300,
                reasoning_tokens: 50,
                total_tokens: 1500,
                request_count: 5,
            }],
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
        assert!(session.get("reasoningTokens").is_some(), "Missing session.reasoningTokens");
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
