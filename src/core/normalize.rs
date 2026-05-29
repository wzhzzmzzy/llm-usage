use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::core::error::NormalizeError;
use crate::core::model::*;
use crate::core::provider::*;

pub struct Normalizer;

impl Normalizer {
    pub fn normalize_daily(rows: &[RawDailyRow]) -> Result<DailyReport, NormalizeError> {
        let mut days = Vec::new();
        let mut total_cost = 0.0;
        let mut total_input = 0u64;
        let mut total_output = 0u64;
        let mut total_cache_creation = 0u64;
        let mut total_cache_read = 0u64;

        for row in rows {
            days.push(Self::normalize_daily_row(row)?);
            total_cost += row.total_cost;
            total_input += row.input_tokens;
            total_output += row.output_tokens;
            total_cache_creation += row.cache_creation_tokens.unwrap_or(0);
            total_cache_read += row.cache_read_tokens.unwrap_or(0);
        }

        days.sort_by(|a, b| b.date.cmp(&a.date));

        let total_tokens = total_input + total_output;
        let cost = Decimal::from_f64_retain(total_cost).unwrap_or_default();

        Ok(DailyReport {
            days,
            totals: UsageMetric {
                total_cost_usd: cost.to_string(),
                total_cost_usd_number: total_cost,
                cost_formatted: format!("${:.2}", total_cost),
                total_tokens,
                input_tokens: total_input,
                output_tokens: total_output,
                cache_creation_tokens: Some(total_cache_creation),
                cache_read_tokens: Some(total_cache_read),
                request_count: None,
                model_breakdown: None,
            },
        })
    }

    pub fn normalize_monthly(rows: &[RawMonthlyRow]) -> Result<MonthlyReport, NormalizeError> {
        let mut months = Vec::new();
        let mut total_cost = 0.0;
        let mut total_input = 0u64;
        let mut total_output = 0u64;
        let mut total_cache_creation = 0u64;
        let mut total_cache_read = 0u64;

        for row in rows {
            months.push(Self::normalize_monthly_row(row)?);
            total_cost += row.total_cost;
            total_input += row.input_tokens;
            total_output += row.output_tokens;
            total_cache_creation += row.cache_creation_tokens.unwrap_or(0);
            total_cache_read += row.cache_read_tokens.unwrap_or(0);
        }

        months.sort_by(|a, b| b.month.cmp(&a.month));

        let total_tokens = total_input + total_output;
        let cost = Decimal::from_f64_retain(total_cost).unwrap_or_default();

        Ok(MonthlyReport {
            months,
            totals: UsageMetric {
                total_cost_usd: cost.to_string(),
                total_cost_usd_number: total_cost,
                cost_formatted: format!("${:.2}", total_cost),
                total_tokens,
                input_tokens: total_input,
                output_tokens: total_output,
                cache_creation_tokens: Some(total_cache_creation),
                cache_read_tokens: Some(total_cache_read),
                request_count: None,
                model_breakdown: None,
            },
        })
    }

    pub fn normalize_session(rows: &[RawSessionRow]) -> Result<SessionReport, NormalizeError> {
        let mut sessions = Vec::new();
        let mut total_cost = 0.0;
        let mut total_input = 0u64;
        let mut total_output = 0u64;

        for row in rows {
            sessions.push(Self::normalize_session_row(row)?);
            total_cost += row.total_cost;
            total_input += row.input_tokens;
            total_output += row.output_tokens;
        }

        sessions.sort_by(|a, b| {
            b.last_activity
                .unwrap_or_default()
                .cmp(&a.last_activity.unwrap_or_default())
        });

        let total_tokens = total_input + total_output;
        let cost = Decimal::from_f64_retain(total_cost).unwrap_or_default();

        Ok(SessionReport {
            sessions,
            totals: UsageMetric {
                total_cost_usd: cost.to_string(),
                total_cost_usd_number: total_cost,
                cost_formatted: format!("${:.2}", total_cost),
                total_tokens,
                input_tokens: total_input,
                output_tokens: total_output,
                cache_creation_tokens: None,
                cache_read_tokens: None,
                request_count: None,
                model_breakdown: None,
            },
        })
    }

    pub fn normalize_blocks(rows: &[RawBlockRow]) -> Result<BlocksReport, NormalizeError> {
        let mut blocks = Vec::new();
        let mut active_block = None;
        let mut total_cost = 0.0;
        let mut total_input = 0u64;
        let mut total_output = 0u64;

        for row in rows {
            if row.is_gap == Some(true) {
                continue;
            }
            let block = Self::normalize_block_row(row)?;
            if block.is_active {
                active_block = Some(block.clone());
            }
            total_cost += block.cost_usd_number;
            total_input += block.input_tokens;
            total_output += block.output_tokens;
            blocks.push(block);
        }

        blocks.sort_by(|a, b| b.start_time.cmp(&a.start_time));

        let total_tokens = total_input + total_output;
        let cost = Decimal::from_f64_retain(total_cost).unwrap_or_default();

        Ok(BlocksReport {
            blocks,
            active_block,
            totals: UsageMetric {
                total_cost_usd: cost.to_string(),
                total_cost_usd_number: total_cost,
                cost_formatted: format!("${:.2}", total_cost),
                total_tokens,
                input_tokens: total_input,
                output_tokens: total_output,
                cache_creation_tokens: None,
                cache_read_tokens: None,
                request_count: None,
                model_breakdown: None,
            },
        })
    }

    fn normalize_daily_row(row: &RawDailyRow) -> Result<DailyRow, NormalizeError> {
        let date = NaiveDate::parse_from_str(&row.period, "%Y-%m-%d")
            .map_err(|e| NormalizeError::ParseError(format!("Invalid date '{}': {}", row.period, e)))?;

        let total_tokens = row.total_tokens.unwrap_or(row.input_tokens + row.output_tokens);
        let cost = Decimal::from_f64_retain(row.total_cost).unwrap_or_default();

        Ok(DailyRow {
            date,
            cost_usd: cost.to_string(),
            cost_usd_number: row.total_cost,
            cost_formatted: format!("${:.2}", row.total_cost),
            total_tokens,
            input_tokens: row.input_tokens,
            output_tokens: row.output_tokens,
            cache_creation_tokens: row.cache_creation_tokens,
            cache_read_tokens: row.cache_read_tokens,
            request_count: None,
            models_used: row.models_used.clone(),
        })
    }

    fn normalize_monthly_row(row: &RawMonthlyRow) -> Result<MonthlyRow, NormalizeError> {
        let total_tokens = row.total_tokens.unwrap_or(row.input_tokens + row.output_tokens);
        let cost = Decimal::from_f64_retain(row.total_cost).unwrap_or_default();

        Ok(MonthlyRow {
            month: row.period.clone(),
            cost_usd: cost.to_string(),
            cost_usd_number: row.total_cost,
            cost_formatted: format!("${:.2}", row.total_cost),
            total_tokens,
            input_tokens: row.input_tokens,
            output_tokens: row.output_tokens,
            cache_creation_tokens: row.cache_creation_tokens,
            cache_read_tokens: row.cache_read_tokens,
            request_count: None,
            models_used: row.models_used.clone(),
        })
    }

    fn normalize_session_row(row: &RawSessionRow) -> Result<SessionRow, NormalizeError> {
        let total_tokens = row.total_tokens.unwrap_or(row.input_tokens + row.output_tokens);
        let cost = Decimal::from_f64_retain(row.total_cost).unwrap_or_default();

        let last_activity = row
            .last_activity
            .as_ref()
            .and_then(|s| {
                chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                    .ok()
                    .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc())
            });

        Ok(SessionRow {
            session_id: row.session_id.clone(),
            project_path: row.project_path.clone(),
            cost_usd: cost.to_string(),
            cost_usd_number: row.total_cost,
            cost_formatted: format!("${:.2}", row.total_cost),
            total_tokens,
            input_tokens: row.input_tokens,
            output_tokens: row.output_tokens,
            last_activity,
            models_used: row.models_used.clone(),
        })
    }

    fn normalize_block_row(row: &RawBlockRow) -> Result<BlockRow, NormalizeError> {
        let input_tokens = row.input_tokens;
        let output_tokens = row.output_tokens;
        let total_tokens = row.total_tokens.unwrap_or(input_tokens + output_tokens);
        let cost = row.cost_usd.unwrap_or(0.0);
        let cost_decimal = Decimal::from_f64_retain(cost).unwrap_or_default();

        let start_time = chrono::DateTime::parse_from_rfc3339(&row.start_time)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());

        let end_time = row
            .end_time
            .as_ref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc));

        Ok(BlockRow {
            block_id: row.block_id.clone(),
            start_time,
            end_time,
            cost_usd: cost_decimal.to_string(),
            cost_usd_number: cost,
            cost_formatted: format!("${:.2}", cost),
            total_tokens,
            input_tokens,
            output_tokens,
            is_active: row.is_active,
            models_used: row.models_used.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_daily() {
        let rows = vec![RawDailyRow {
            period: "2025-01-15".to_string(),
            input_tokens: 1000,
            output_tokens: 500,
            cache_creation_tokens: Some(100),
            cache_read_tokens: Some(200),
            total_cost: 0.05,
            total_tokens: Some(1500),
            request_count: None,
            models_used: Some(vec!["claude-3.5-sonnet".to_string()]),
        }];

        let report = Normalizer::normalize_daily(&rows).unwrap();
        assert_eq!(report.days.len(), 1);
        assert_eq!(report.days[0].total_tokens, 1500);
        assert_eq!(report.days[0].input_tokens, 1000);
        assert_eq!(report.days[0].output_tokens, 500);
        assert!(report.days[0].cost_usd_number > 0.0);
    }

    #[test]
    fn test_daily_report_json_matches_frontend_types() {
        let rows = vec![RawDailyRow {
            period: "2026-05-29".to_string(),
            input_tokens: 1000,
            output_tokens: 500,
            cache_creation_tokens: Some(100),
            cache_read_tokens: Some(200),
            total_cost: 0.05,
            total_tokens: Some(1500),
            request_count: None,
            models_used: Some(vec!["claude-sonnet-4".to_string()]),
        }];

        let report = Normalizer::normalize_daily(&rows).unwrap();
        let json = serde_json::to_value(&report).unwrap();

        assert!(json.get("days").is_some(), "Missing 'days' field");
        assert!(json.get("totals").is_some(), "Missing 'totals' field");

        let day = &json["days"][0];
        assert!(day.get("date").is_some(), "Missing day.date");
        assert!(day.get("costUsd").is_some(), "Missing day.costUsd");
        assert!(day.get("costUsdNumber").is_some(), "Missing day.costUsdNumber");
        assert!(day.get("costFormatted").is_some(), "Missing day.costFormatted");
        assert!(day.get("totalTokens").is_some(), "Missing day.totalTokens");
        assert!(day.get("inputTokens").is_some(), "Missing day.inputTokens");
        assert!(day.get("outputTokens").is_some(), "Missing day.outputTokens");
        assert!(day.get("cacheCreationTokens").is_some(), "Missing day.cacheCreationTokens");
        assert!(day.get("cacheReadTokens").is_some(), "Missing day.cacheReadTokens");

        let totals = &json["totals"];
        assert!(totals.get("totalCostUsd").is_some(), "Missing totals.totalCostUsd");
        assert!(totals.get("totalCostUsdNumber").is_some(), "Missing totals.totalCostUsdNumber");
        assert!(totals.get("costFormatted").is_some(), "Missing totals.costFormatted");
        assert!(totals.get("totalTokens").is_some(), "Missing totals.totalTokens");
        assert!(totals.get("inputTokens").is_some(), "Missing totals.inputTokens");
        assert!(totals.get("outputTokens").is_some(), "Missing totals.outputTokens");
    }

    #[test]
    fn test_monthly_report_json_matches_frontend_types() {
        let rows = vec![RawMonthlyRow {
            period: "2026-05".to_string(),
            input_tokens: 5000,
            output_tokens: 2500,
            cache_creation_tokens: Some(500),
            cache_read_tokens: Some(1000),
            total_cost: 0.25,
            total_tokens: Some(7500),
            request_count: None,
            models_used: Some(vec!["claude-sonnet-4".to_string()]),
        }];

        let report = Normalizer::normalize_monthly(&rows).unwrap();
        let json = serde_json::to_value(&report).unwrap();

        assert!(json.get("months").is_some(), "Missing 'months' field");
        assert!(json.get("totals").is_some(), "Missing 'totals' field");

        let month = &json["months"][0];
        assert!(month.get("month").is_some(), "Missing month.month");
        assert!(month.get("costUsd").is_some(), "Missing month.costUsd");
        assert!(month.get("totalTokens").is_some(), "Missing month.totalTokens");
    }

    #[test]
    fn test_session_report_json_matches_frontend_types() {
        let rows = vec![RawSessionRow {
            session_id: "abc123".to_string(),
            project_path: Some("/home/user/project".to_string()),
            input_tokens: 1000,
            output_tokens: 500,
            total_tokens: Some(1500),
            total_cost: 0.05,
            last_activity: Some("2026-05-29".to_string()),
            models_used: Some(vec!["claude-sonnet-4".to_string()]),
        }];

        let report = Normalizer::normalize_session(&rows).unwrap();
        let json = serde_json::to_value(&report).unwrap();

        assert!(json.get("sessions").is_some(), "Missing 'sessions' field");

        let session = &json["sessions"][0];
        assert!(session.get("sessionId").is_some(), "Missing session.sessionId");
        assert!(session.get("projectPath").is_some(), "Missing session.projectPath");
        assert!(session.get("costUsd").is_some(), "Missing session.costUsd");
        assert!(session.get("costUsdNumber").is_some(), "Missing session.costUsdNumber");
        assert!(session.get("totalTokens").is_some(), "Missing session.totalTokens");
        assert!(session.get("inputTokens").is_some(), "Missing session.inputTokens");
        assert!(session.get("outputTokens").is_some(), "Missing session.outputTokens");
        assert!(session.get("lastActivity").is_some(), "Missing session.lastActivity");
        assert!(session.get("modelsUsed").is_some(), "Missing session.modelsUsed");
    }

    #[test]
    fn test_blocks_report_json_matches_frontend_types() {
        let rows = vec![RawBlockRow {
            block_id: "block-1".to_string(),
            start_time: "2026-05-29T10:00:00Z".to_string(),
            end_time: Some("2026-05-29T11:00:00Z".to_string()),
            is_active: false,
            is_gap: None,
            input_tokens: 1000,
            output_tokens: 500,
            total_tokens: Some(1500),
            cost_usd: Some(0.05),
            models_used: Some(vec!["claude-sonnet-4".to_string()]),
        }];

        let report = Normalizer::normalize_blocks(&rows).unwrap();
        let json = serde_json::to_value(&report).unwrap();

        assert!(json.get("blocks").is_some(), "Missing 'blocks' field");

        let block = &json["blocks"][0];
        assert!(block.get("blockId").is_some(), "Missing block.blockId");
        assert!(block.get("startTime").is_some(), "Missing block.startTime");
        assert!(block.get("endTime").is_some(), "Missing block.endTime");
        assert!(block.get("costUsd").is_some(), "Missing block.costUsd");
        assert!(block.get("totalTokens").is_some(), "Missing block.totalTokens");
        assert!(block.get("isActive").is_some(), "Missing block.isActive");
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
