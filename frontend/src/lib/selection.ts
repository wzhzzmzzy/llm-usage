export type Selection =
  | { type: 'none' }
  | { type: 'pending'; start: string }
  | { type: 'range'; start: string; end: string };

export type CellHighlight = 'start' | 'end' | 'in-range' | 'preview' | 'none';

/**
 * Build a confirmed range from two clicked dates.
 * Auto-sorts so start <= end regardless of click order.
 */
export function makeRange(
  a: string,
  b: string,
): Extract<Selection, { type: 'range' }> {
  const [start, end] = [a, b].sort();
  return { type: 'range', start, end };
}

/**
 * Inclusive day count between two ISO date strings (YYYY-MM-DD).
 * dayCount('2026-01-01', '2026-01-01') === 1
 * dayCount('2026-01-01', '2026-01-10') === 10
 */
export function dayCount(start: string, end: string): number {
  const ms = new Date(end).getTime() - new Date(start).getTime();
  return Math.round(ms / (1000 * 60 * 60 * 24)) + 1;
}

/**
 * Determine the visual highlight state for a single mosaic cell.
 *
 * pending + no hoverDate  → only the anchor ('start') is highlighted
 * pending + hoverDate     → the anchor–hover span shows 'preview'; anchor is 'start'
 * range                   → start/end cells are 'start'/'end'; interior is 'in-range'
 * none                    → always 'none'
 */
export function getCellHighlight(
  date: string,
  selection: Selection,
  hoverDate: string | null,
): CellHighlight {
  if (selection.type === 'range') {
    if (date === selection.start) return 'start';
    if (date === selection.end) return 'end';
    if (date > selection.start && date < selection.end) return 'in-range';
    return 'none';
  }

  if (selection.type === 'pending') {
    if (date === selection.start) return 'start';
    if (hoverDate) {
      const [lo, hi] = [selection.start, hoverDate].sort();
      if (date >= lo && date <= hi) return 'preview';
    }
    return 'none';
  }

  return 'none';
}
