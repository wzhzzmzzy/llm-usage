import { describe, it, expect } from 'vitest';
import { formatTokens } from '../components/utils';

describe('formatTokens', () => {
  it('shows raw value below 1M', () => {
    expect(formatTokens(0)).toBe('0');
    expect(formatTokens(1_000)).toBe('1,000');
    expect(formatTokens(999_999)).toBe('999,999');
  });

  it('uses K for 1M–10M range', () => {
    expect(formatTokens(1_000_000)).toBe('1000K');
    expect(formatTokens(1_500_000)).toBe('1500K');
    expect(formatTokens(1_502_500)).toBe('1502.5K');
    expect(formatTokens(9_999_000)).toBe('9999K');
  });

  it('uses M for 10M–100B range', () => {
    expect(formatTokens(10_000_000)).toBe('10M');
    expect(formatTokens(10_500_000)).toBe('10.5M');
    expect(formatTokens(100_000_000)).toBe('100M');
    expect(formatTokens(500_500_000)).toBe('500.5M');
  });

  it('uses B for 100B+ range', () => {
    expect(formatTokens(100_000_000_000)).toBe('100B');
    expect(formatTokens(250_000_000_000)).toBe('250B');
    expect(formatTokens(250_500_000_000)).toBe('250.5B');
  });

  it('caps decimal places at 2', () => {
    expect(formatTokens(1_502_536)).toBe('1502.54K');   // 1502.536 → 1502.54
    expect(formatTokens(10_125_678)).toBe('10.13M');    // 10.125678 → 10.13
  });

  it('trims trailing zeros', () => {
    expect(formatTokens(1_000_000)).toBe('1000K');      // not "1000.00K"
    expect(formatTokens(10_000_000)).toBe('10M');       // not "10.00M"
    expect(formatTokens(100_000_000_000)).toBe('100B'); // not "100.00B"
  });

  it('handles undefined', () => {
    expect(formatTokens(undefined)).toBe('0');
  });
});
