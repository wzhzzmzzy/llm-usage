import { describe, it, expect } from 'vitest';
import { makeRange, dayCount, getCellHighlight } from '../lib/selection';
import type { Selection } from '../lib/selection';

describe('makeRange', () => {
  it('sorts so start <= end regardless of click order', () => {
    expect(makeRange('2026-01-10', '2026-01-05')).toEqual({
      type: 'range',
      start: '2026-01-05',
      end: '2026-01-10',
    });
  });
  it('preserves order when already sorted', () => {
    expect(makeRange('2026-01-01', '2026-01-31')).toEqual({
      type: 'range',
      start: '2026-01-01',
      end: '2026-01-31',
    });
  });
  it('handles same date (single-day range)', () => {
    expect(makeRange('2026-06-15', '2026-06-15')).toEqual({
      type: 'range',
      start: '2026-06-15',
      end: '2026-06-15',
    });
  });
});

describe('dayCount', () => {
  it('returns 1 for same start and end', () => {
    expect(dayCount('2026-01-05', '2026-01-05')).toBe(1);
  });
  it('returns 10 for a 10-day span', () => {
    expect(dayCount('2026-01-01', '2026-01-10')).toBe(10);
  });
  it('returns 31 for a full month', () => {
    expect(dayCount('2026-01-01', '2026-01-31')).toBe(31);
  });
});

describe('getCellHighlight — selection type none', () => {
  it('always returns none', () => {
    expect(getCellHighlight('2026-01-01', { type: 'none' }, null)).toBe('none');
    expect(getCellHighlight('2026-01-01', { type: 'none' }, '2026-01-01')).toBe('none');
  });
});

describe('getCellHighlight — selection type pending', () => {
  const pending: Selection = { type: 'pending', start: '2026-01-05' };

  it('returns start for the anchor cell', () => {
    expect(getCellHighlight('2026-01-05', pending, null)).toBe('start');
    expect(getCellHighlight('2026-01-05', pending, '2026-01-10')).toBe('start');
  });
  it('returns none when no hoverDate', () => {
    expect(getCellHighlight('2026-01-06', pending, null)).toBe('none');
  });
  it('returns preview for cells between anchor and hover (forward)', () => {
    expect(getCellHighlight('2026-01-07', pending, '2026-01-10')).toBe('preview');
    expect(getCellHighlight('2026-01-10', pending, '2026-01-10')).toBe('preview');
  });
  it('returns preview for cells between hover and anchor (backward — auto-sort)', () => {
    expect(getCellHighlight('2026-01-03', pending, '2026-01-01')).toBe('preview');
    expect(getCellHighlight('2026-01-01', pending, '2026-01-01')).toBe('preview');
  });
  it('returns none for cells outside the anchor–hover span', () => {
    expect(getCellHighlight('2026-01-11', pending, '2026-01-10')).toBe('none');
    expect(getCellHighlight('2026-01-01', pending, '2026-01-10')).toBe('none');
  });
});

describe('getCellHighlight — selection type range', () => {
  const range: Selection = { type: 'range', start: '2026-01-05', end: '2026-01-10' };

  it('returns start for the start cell', () => {
    expect(getCellHighlight('2026-01-05', range, null)).toBe('start');
  });
  it('returns end for the end cell', () => {
    expect(getCellHighlight('2026-01-10', range, null)).toBe('end');
  });
  it('returns in-range for cells inside the range', () => {
    expect(getCellHighlight('2026-01-06', range, null)).toBe('in-range');
    expect(getCellHighlight('2026-01-09', range, null)).toBe('in-range');
  });
  it('returns none for cells outside the range', () => {
    expect(getCellHighlight('2026-01-04', range, null)).toBe('none');
    expect(getCellHighlight('2026-01-11', range, null)).toBe('none');
  });
  it('returns start for a single-day range (start === end)', () => {
    const single: Selection = { type: 'range', start: '2026-01-05', end: '2026-01-05' };
    expect(getCellHighlight('2026-01-05', single, null)).toBe('start');
  });
});
