import { describe, it, expect } from 'vitest';
import { formatTokens } from '../components/utils';

describe('formatTokens', () => {
  it('shows raw value below 1M', () => {
    expect(formatTokens(0)).toBe('0');
    expect(formatTokens(1_000)).toBe('1,000');
    expect(formatTokens(999_999)).toBe('999,999');
  });

  it('uses K for 1M–100M range', () => {
    expect(formatTokens(1_000_000)).toBe('1000K');
    expect(formatTokens(1_500_000)).toBe('1500K');
    expect(formatTokens(1_502_500)).toBe('1502.5K');
    expect(formatTokens(99_999_000)).toBe('99999K');
  });

  it('uses M for 100M–100B range', () => {
    expect(formatTokens(100_000_000)).toBe('100M');
    expect(formatTokens(500_000_000)).toBe('500M');
    expect(formatTokens(500_500_000)).toBe('500.5M');
  });

  it('uses B for 100B+ range', () => {
    expect(formatTokens(100_000_000_000)).toBe('100B');
    expect(formatTokens(250_000_000_000)).toBe('250B');
    expect(formatTokens(250_500_000_000)).toBe('250.5B');
  });

  it('trims trailing zeros', () => {
    expect(formatTokens(1_000_000)).toBe('1000K');       // not "1000.000K"
    expect(formatTokens(100_000_000)).toBe('100M');      // not "100.000M"
    expect(formatTokens(100_000_000_000)).toBe('100B');  // not "100.000B"
  });

  it('handles undefined', () => {
    expect(formatTokens(undefined)).toBe('0');
  });
});
