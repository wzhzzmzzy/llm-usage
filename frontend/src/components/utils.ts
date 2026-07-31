/**
 * Format a token count with an adaptive SI unit.
 *
 * Ranges:
 *   n < 1_000_000          raw toLocaleString
 *   1M  ≤ n < 10M          divide by 1_000, append "K"
 *   10M ≤ n < 100B         divide by 1_000_000, append "M"
 *   n ≥ 100_000_000_000    divide by 1_000_000_000, append "B"
 *
 * Trailing zeros are trimmed (up to 2 decimal places).
 */
export function formatTokens(n: number | undefined): string {
  const v = n ?? 0;
  if (v < 1_000_000) return v.toLocaleString();
  if (v < 10_000_000) return `${parseFloat((v / 1_000).toFixed(2))}K`;
  if (v < 100_000_000_000) return `${parseFloat((v / 1_000_000).toFixed(2))}M`;
  return `${parseFloat((v / 1_000_000_000).toFixed(2))}B`;
}

/**
 * Sum backend-computed per-model costs. Returns null when the breakdown
 * predates backend costing (any item missing the field), so callers can
 * fall back to their own pricing-table estimate.
 */
export function sumBackendCost(breakdown: Array<{ model: string; cost?: number }> | undefined): number | null {
  if (!breakdown || breakdown.length === 0) return null;
  if (!breakdown.every((item) => typeof item.cost === 'number')) return null;
  return breakdown.reduce((total, item) => total + (item.cost ?? 0), 0);
}
