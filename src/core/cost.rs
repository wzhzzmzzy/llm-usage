use crate::core::pricing::Pricing;

/// Cost mode for calculations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CostMode {
    /// Use pre-calculated cost if available
    Display,
    /// Use pre-calculated cost, fall back to calculation
    Auto,
    /// Always calculate from tokens
    Calculate,
}

/// Calculate cost for a usage entry
pub fn calculate_cost(
    model: Option<&str>,
    input_tokens: u64,
    output_tokens: u64,
    cache_creation_tokens: u64,
    cache_read_tokens: u64,
    cost_usd: Option<f64>,
    mode: CostMode,
    pricing: Option<&Pricing>,
) -> f64 {
    match mode {
        CostMode::Display => cost_usd.unwrap_or(0.0),
        CostMode::Auto => {
            cost_usd.unwrap_or_else(|| {
                calculate_cost_from_tokens(
                    model,
                    input_tokens,
                    output_tokens,
                    cache_creation_tokens,
                    cache_read_tokens,
                    pricing,
                )
            })
        }
        CostMode::Calculate => calculate_cost_from_tokens(
            model,
            input_tokens,
            output_tokens,
            cache_creation_tokens,
            cache_read_tokens,
            pricing,
        ),
    }
}

/// Calculate cost from token counts using pricing
fn calculate_cost_from_tokens(
    _model: Option<&str>,
    input_tokens: u64,
    output_tokens: u64,
    cache_creation_tokens: u64,
    cache_read_tokens: u64,
    pricing: Option<&Pricing>,
) -> f64 {
    let Some(pricing) = pricing else {
        return 0.0;
    };
    
    tiered_cost(input_tokens, pricing.input, pricing.input_above_200k)
        + tiered_cost(output_tokens, pricing.output, pricing.output_above_200k)
        + tiered_cost(
            cache_creation_tokens,
            pricing.cache_create,
            pricing.cache_create_above_200k,
        )
        + tiered_cost(
            cache_read_tokens,
            pricing.cache_read,
            pricing.cache_read_above_200k,
        )
}

/// Calculate cost with tiered pricing for >200k tokens
fn tiered_cost(tokens: u64, base: f64, above: Option<f64>) -> f64 {
    const THRESHOLD: u64 = 200_000;
    if tokens == 0 {
        return 0.0;
    }
    if let Some(above) = above {
        if tokens > THRESHOLD {
            return (THRESHOLD as f64 * base) + ((tokens - THRESHOLD) as f64 * above);
        }
    }
    tokens as f64 * base
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_cost_display_mode() {
        let cost = calculate_cost(
            Some("test"),
            100,
            200,
            0,
            0,
            Some(0.05),
            CostMode::Display,
            None,
        );
        assert_eq!(cost, 0.05);
    }

    #[test]
    fn test_calculate_cost_display_mode_no_cost() {
        let cost = calculate_cost(
            Some("test"),
            100,
            200,
            0,
            0,
            None,
            CostMode::Display,
            None,
        );
        assert_eq!(cost, 0.0);
    }

    #[test]
    fn test_calculate_cost_calculate_mode() {
        let pricing = Pricing {
            input: 1e-6,
            output: 2e-6,
            cache_create: 1.25e-6,
            cache_read: 0.1e-6,
            input_above_200k: None,
            output_above_200k: None,
            cache_create_above_200k: None,
            cache_read_above_200k: None,
        };

        let cost = calculate_cost(
            Some("test"),
            1000,
            2000,
            100,
            50,
            Some(0.05), // Should be ignored in Calculate mode
            CostMode::Calculate,
            Some(&pricing),
        );
        
        // 1000 * 1e-6 + 2000 * 2e-6 + 100 * 1.25e-6 + 50 * 0.1e-6
        assert_eq!(cost, 0.001 + 0.004 + 0.000125 + 0.000005);
    }

    #[test]
    fn test_tiered_cost_no_tier() {
        let cost = tiered_cost(1000, 1e-6, None);
        assert_eq!(cost, 0.001);
    }

    #[test]
    fn test_tiered_cost_with_tier() {
        let cost = tiered_cost(300_000, 1e-6, Some(2e-6));
        assert_eq!(cost, 200_000.0 * 1e-6 + 100_000.0 * 2e-6);
    }
}
