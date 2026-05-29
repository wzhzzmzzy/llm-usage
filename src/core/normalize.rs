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

        let last_activity_str = row
            .last_activity
            .as_ref()
            .or_else(|| row.metadata.as_ref().and_then(|m| m.last_activity.as_ref()));

        let last_activity = last_activity_str
            .and_then(|s| {
                chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                    .ok()
                    .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc())
            });

        let project_path = row
            .project_path
            .clone()
            .or_else(|| row.metadata.as_ref().and_then(|m| m.project_path.clone()));

        Ok(SessionRow {
            session_id: row.period.clone(),
            project_path,
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
        let token_counts = row.token_counts.as_ref();
        let input_tokens = token_counts.map(|t| t.input_tokens).unwrap_or(0);
        let output_tokens = token_counts.map(|t| t.output_tokens).unwrap_or(0);
        let total_tokens = row.total_tokens.unwrap_or(input_tokens + output_tokens);
        let cost = row.cost_usd.unwrap_or(0.0);
        let cost_decimal = Decimal::from_f64_retain(cost).unwrap_or_default();

        let start_time = chrono::DateTime::parse_from_rfc3339(&row.start_time)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());

        let end_time = row
            .actual_end_time
            .as_ref()
            .or(row.end_time.as_ref())
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc));

        Ok(BlockRow {
            block_id: row.id.clone(),
            start_time,
            end_time,
            cost_usd: cost_decimal.to_string(),
            cost_usd_number: cost,
            cost_formatted: format!("${:.2}", cost),
            total_tokens,
            input_tokens,
            output_tokens,
            is_active: row.is_active,
            models_used: row.models.clone(),
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
            models_used: Some(vec!["claude-3.5-sonnet".to_string()]),
            model_breakdowns: None,
        }];

        let report = Normalizer::normalize_daily(&rows).unwrap();
        assert_eq!(report.days.len(), 1);
        assert_eq!(report.days[0].total_tokens, 1500);
        assert_eq!(report.days[0].input_tokens, 1000);
        assert_eq!(report.days[0].output_tokens, 500);
        assert!(report.days[0].cost_usd_number > 0.0);
    }
}
