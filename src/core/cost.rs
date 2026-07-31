use crate::core::adapter::UsageEntry;
use crate::core::pricing::PricingMap;

/// Compute and store the cost of every entry. Fast-tier entries carry a
/// "-fast" model suffix for grouping; the suffix is stripped before the
/// pricing lookup while the entry's is_fast flag applies the multiplier.
pub fn apply_costs(entries: &mut [UsageEntry], pricing: &PricingMap) {
    for entry in entries {
        let lookup = entry
            .model
            .as_deref()
            .map(|m| m.strip_suffix("-fast").unwrap_or(m));
        entry.cost = calculate_entry_cost(
            lookup,
            entry.input_tokens,
            entry.output_tokens,
            entry.reasoning_tokens,
            entry.cache_creation_tokens,
            entry.cache_creation_1h_tokens,
            entry.cache_read_tokens,
            entry.is_fast,
            entry.cost_usd,
            Some(pricing),
        );
    }
}

/// 1h-ephemeral cache creation is billed at twice the input rate (ccusage parity)
pub const CACHE_CREATE_1H_INPUT_MULTIPLIER: f64 = 2.0;

/// Cost of a single usage entry, ccusage Auto-mode semantics: a present
/// costUSD always wins; otherwise calculate from per-model pricing.
/// `cache_creation_tokens` is the combined 5m+1h count; the 1h portion is
/// priced separately at 2x the input rate, and each cache bucket tiers on
/// its own 200k boundary. Reasoning tokens bill at the output rate.
/// speed=fast entries multiply the total by the model's fast multiplier.
#[allow(clippy::too_many_arguments)]
pub fn calculate_entry_cost(
    model: Option<&str>,
    input_tokens: u64,
    output_tokens: u64,
    reasoning_tokens: u64,
    cache_creation_tokens: u64,
    cache_creation_1h_tokens: u64,
    cache_read_tokens: u64,
    is_fast: bool,
    cost_usd: Option<f64>,
    pricing: Option<&PricingMap>,
) -> f64 {
    if let Some(cost) = cost_usd {
        return cost;
    }
    let (Some(model), Some(pricing)) = (model, pricing) else {
        return 0.0;
    };
    let Some(p) = pricing.find(model) else {
        return 0.0;
    };

    let cache_create_5m = cache_creation_tokens.saturating_sub(cache_creation_1h_tokens);
    let base = tiered_cost(input_tokens, p.input, p.input_above_200k)
        + tiered_cost(
            output_tokens + reasoning_tokens,
            p.output,
            p.output_above_200k,
        )
        + tiered_cost(
            cache_create_5m,
            p.cache_create,
            p.cache_create_above_200k,
        )
        + tiered_cost(
            cache_creation_1h_tokens,
            p.input * CACHE_CREATE_1H_INPUT_MULTIPLIER,
            p.input_above_200k
                .map(|above| above * CACHE_CREATE_1H_INPUT_MULTIPLIER),
        )
        + tiered_cost(cache_read_tokens, p.cache_read, p.cache_read_above_200k);

    if is_fast {
        base * p.fast_multiplier
    } else {
        base
    }
}

/// Marginal tiering: the first 200k tokens bill at the base rate, the
/// remainder at the above-200k rate when one is published.
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
    use crate::core::pricing::Pricing;

    fn pricing_map() -> PricingMap {
        let mut map = PricingMap::default();
        map.load_json(
            r#"{"data":[{"id":"test-model","pricing":{"prompt":"0.000001","completion":"0.000002","input_cache_write":"0.00000125","input_cache_read":"0.0000001"}}]}"#,
        );
        map
    }

    fn tiered_pricing_map() -> PricingMap {
        let mut map = pricing_map();
        map.entries.insert(
            "tiered-model".to_string(),
            Pricing {
                input: 1e-6,
                output: 2e-6,
                cache_create: 1.25e-6,
                cache_read: 0.1e-6,
                fast_multiplier: 1.0,
                input_above_200k: Some(2e-6),
                output_above_200k: Some(4e-6),
                cache_create_above_200k: Some(2.5e-6),
                cache_read_above_200k: Some(0.2e-6),
            },
        );
        map
    }

    #[test]
    fn test_cost_usd_preferred_over_calculation() {
        let map = pricing_map();
        let cost = calculate_entry_cost(
            Some("test-model"),
            1000,
            2000,
            500,
            100,
            0,
            50,
            false,
            Some(0.05),
            Some(&map),
        );
        assert_eq!(cost, 0.05);
    }

    #[test]
    fn test_cost_usd_preferred_even_without_pricing() {
        let cost = calculate_entry_cost(Some("anything"), 100, 200, 0, 0, 0, 0, false, Some(0.07), None);
        assert_eq!(cost, 0.07);
    }

    #[test]
    fn test_no_pricing_map_yields_zero() {
        let cost = calculate_entry_cost(Some("test-model"), 100, 200, 0, 0, 0, 0, false, None, None);
        assert_eq!(cost, 0.0);
    }

    #[test]
    fn test_unknown_model_yields_zero() {
        let map = pricing_map();
        let cost = calculate_entry_cost(
            Some("never-heard-of-it"),
            100,
            200,
            0,
            0,
            0,
            0,
            false,
            None,
            Some(&map),
        );
        assert_eq!(cost, 0.0);
    }

    #[test]
    fn test_missing_model_yields_zero() {
        let map = pricing_map();
        let cost = calculate_entry_cost(None, 100, 200, 0, 0, 0, 0, false, None, Some(&map));
        assert_eq!(cost, 0.0);
    }

    #[test]
    fn test_flat_rate_calculation() {
        let map = pricing_map();
        // input 1000 @1e-6, output+reasoning 2500 @2e-6, 5m 100 @1.25e-6, read 50 @0.1e-6
        let cost = calculate_entry_cost(
            Some("test-model"),
            1000,
            2000,
            500,
            100,
            0,
            50,
            false,
            None,
            Some(&map),
        );
        assert_eq!(cost, 0.001 + 0.005 + 0.000125 + 0.000005);
    }

    #[test]
    fn test_1h_cache_creation_billed_at_double_input_rate() {
        let map = pricing_map();
        // combined creation 300 of which 200 is 1h: 100 5m @1.25e-6 + 200 1h @(2 * 1e-6)
        let cost = calculate_entry_cost(
            Some("test-model"),
            0,
            0,
            0,
            300,
            200,
            0,
            false,
            None,
            Some(&map),
        );
        assert_eq!(cost, 100.0 * 1.25e-6 + 200.0 * 2e-6);
    }

    #[test]
    fn test_cache_buckets_tier_separately() {
        let map = tiered_pricing_map();
        // 150k 5m + 150k 1h: merged they'd cross 200k, separate buckets stay at base
        let cost = calculate_entry_cost(
            Some("tiered-model"),
            0,
            0,
            0,
            300_000,
            150_000,
            0,
            false,
            None,
            Some(&map),
        );
        let expected = 150_000.0 * 1.25e-6 + 150_000.0 * 2e-6;
        assert_eq!(cost, expected);
    }

    #[test]
    fn test_1h_above_200k_rate_is_double_input_above_rate() {
        let map = tiered_pricing_map();
        // 300k 1h tokens: 200k @(2*1e-6) + 100k @(2*2e-6)
        let cost = calculate_entry_cost(
            Some("tiered-model"),
            0,
            0,
            0,
            300_000,
            300_000,
            0,
            false,
            None,
            Some(&map),
        );
        let expected = 200_000.0 * 2e-6 + 100_000.0 * 4e-6;
        assert_eq!(cost, expected);
    }

    #[test]
    fn test_marginal_tiering_on_input() {
        let map = tiered_pricing_map();
        let cost = calculate_entry_cost(
            Some("tiered-model"),
            300_000,
            0,
            0,
            0,
            0,
            0,
            false,
            None,
            Some(&map),
        );
        assert_eq!(cost, 200_000.0 * 1e-6 + 100_000.0 * 2e-6);
    }

    #[test]
    fn test_fast_multiplier_applied() {
        let mut map = pricing_map();
        map.entries.insert(
            "fast-model".to_string(),
            Pricing {
                input: 1e-6,
                output: 2e-6,
                cache_create: 1.25e-6,
                cache_read: 0.1e-6,
                fast_multiplier: 6.0,
                input_above_200k: None,
                output_above_200k: None,
                cache_create_above_200k: None,
                cache_read_above_200k: None,
            },
        );
        let standard = calculate_entry_cost(
            Some("fast-model"),
            1000,
            1000,
            0,
            0,
            0,
            0,
            false,
            None,
            Some(&map),
        );
        let fast = calculate_entry_cost(
            Some("fast-model"),
            1000,
            1000,
            0,
            0,
            0,
            0,
            true,
            None,
            Some(&map),
        );
        assert_eq!(fast, standard * 6.0);
    }

    #[test]
    fn test_tiered_cost_no_tier() {
        assert_eq!(tiered_cost(1000, 1e-6, None), 0.001);
    }

    #[test]
    fn test_tiered_cost_with_tier() {
        assert_eq!(tiered_cost(300_000, 1e-6, Some(2e-6)), 200_000.0 * 1e-6 + 100_000.0 * 2e-6);
    }

    fn bare_entry(model: Option<&str>) -> crate::core::adapter::UsageEntry {
        crate::core::adapter::UsageEntry {
            session_id: "s".to_string(),
            timestamp: "2026-05-29T10:00:00.000Z".to_string(),
            model: model.map(|m| m.to_string()),
            input_tokens: 1000,
            output_tokens: 1000,
            reasoning_tokens: 0,
            cache_creation_tokens: 0,
            cache_creation_1h_tokens: 0,
            cache_read_tokens: 0,
            total_tokens: 2000,
            cost_usd: None,
            is_fast: false,
            cost: 0.0,
            project_path: None,
        }
    }

    #[test]
    fn test_apply_costs_strips_fast_suffix_for_pricing_lookup() {
        let mut map = PricingMap::default();
        map.put_builtin_pricing();
        let mut entries = vec![{
            let mut e = bare_entry(Some("claude-opus-4-6-fast"));
            e.is_fast = true;
            e
        }];
        apply_costs(&mut entries, &map);
        // priced as claude-opus-4-6 with the 6x fast multiplier
        let expected = (1000.0 * 15e-6 + 1000.0 * 75e-6) * 6.0;
        assert_eq!(entries[0].cost, expected);
    }

    #[test]
    fn test_apply_costs_prefers_cost_usd() {
        let mut map = PricingMap::default();
        map.put_builtin_pricing();
        let mut entries = vec![{
            let mut e = bare_entry(Some("claude-opus-4"));
            e.cost_usd = Some(0.42);
            e
        }];
        apply_costs(&mut entries, &map);
        assert_eq!(entries[0].cost, 0.42);
    }

    #[test]
    fn test_apply_costs_unknown_model_zero() {
        let mut map = PricingMap::default();
        map.put_builtin_pricing();
        let mut entries = vec![bare_entry(Some("mystery-model"))];
        apply_costs(&mut entries, &map);
        assert_eq!(entries[0].cost, 0.0);
    }
}
