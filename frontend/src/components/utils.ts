/**
 * Format a token count with an adaptive SI unit.
 *
 * Ranges (per design spec):
 *   n < 1_000_000            raw toLocaleString
 *   1M  ≤ n < 100M           divide by 1_000, append "K"
 *   100M ≤ n < 100_000M      divide by 1_000_000, append "M"
 *   n ≥ 100_000_000_000      divide by 1_000_000_000, append "B"
 *
 * Trailing zeros are trimmed (up to 3 decimal places).
 */
export function formatTokens(n: number | undefined): string {
  const v = n ?? 0;
  if (v < 1_000_000) return v.toLocaleString();
  if (v < 100_000_000) return `${parseFloat((v / 1_000).toFixed(3))}K`;
  if (v < 100_000_000_000) return `${parseFloat((v / 1_000_000).toFixed(3))}M`;
  return `${parseFloat((v / 1_000_000_000).toFixed(3))}B`;
}
