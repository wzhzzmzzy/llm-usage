import { describe, it, expect } from 'vitest';
import { sumBackendCost } from '../components/utils';

describe('sumBackendCost', () => {
  it('sums per-model backend costs when every item carries one', () => {
    expect(
      sumBackendCost([
        { model: 'claude-opus-4-6-fast', cost: 1.5 },
        { model: 'claude-sonnet-4', cost: 0.25 },
      ])
    ).toBe(1.75);
  });

  it('treats a legitimately zero computed cost as present', () => {
    expect(sumBackendCost([{ model: 'unknown-model', cost: 0 }])).toBe(0);
  });

  it('returns null when any item lacks a backend cost', () => {
    expect(
      sumBackendCost([
        { model: 'a', cost: 0.5 },
        { model: 'b' },
      ])
    ).toBeNull();
  });

  it('returns null for empty or missing breakdowns', () => {
    expect(sumBackendCost([])).toBeNull();
    expect(sumBackendCost(undefined)).toBeNull();
  });
});
