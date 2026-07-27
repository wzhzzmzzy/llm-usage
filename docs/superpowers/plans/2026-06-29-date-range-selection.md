# Date Range Selection on Mosaic Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Clicking two cells on the usage mosaic sets a date range; the metric cards (tokens + cost) then show the aggregated totals for every day in that range.

**Architecture:** A new `Selection` discriminated union (`none | pending | range`) replaces the existing `selectedDate: string | null` state in `App.tsx`. A new `frontend/src/lib/selection.ts` module owns the type and all pure helper functions. `ContributionCalendar` receives the `selection` prop and renders per-cell highlight strokes. `App.tsx` aggregates `DailyRow` entries for the active range to drive the metric cards.

**Tech Stack:** React 19, TypeScript, Vitest, Tailwind CSS / SVG attributes

## Global Constraints

- Zero new npm dependencies
- `Selection` type exported from `frontend/src/lib/selection.ts` — import path `@/lib/selection`
- `getCellHighlight` exported from the same file — used by both the calendar and tests
- `selectedDate` state is **removed** from `App.tsx`; `selection: Selection` replaces it
- TS: `pnpm tsc --noEmit` must produce zero errors after every task
- Tests: `pnpm test` must pass after every task (run from `frontend/`)
- ADR 0001 (`docs/adr/0001-date-selection-discriminated-union.md`) documents the state-modelling rationale; do not re-litigate it

---

## File Map

### New
- `frontend/src/lib/selection.ts` — `Selection` type, `CellHighlight` type, `makeRange`, `dayCount`, `getCellHighlight`
- `frontend/src/__tests__/selection.test.ts` — unit tests for all helpers

### Modified
- `frontend/src/i18n/locales/en.ts` — add `'mosaic.range'` key
- `frontend/src/i18n/locales/zh-CN.ts` — add `'mosaic.range'` translation
- `frontend/src/i18n/locales/zh-TW.ts` — add `'mosaic.range'` translation
- `frontend/src/i18n/locales/ja.ts` — add `'mosaic.range'` translation
- `frontend/src/components/contribution-calendar.tsx` — add `selection` prop; per-cell highlight strokes
- `frontend/src/App.tsx` — replace `selectedDate` with `selection`; range aggregation for metric cards

---

## Task 1: Selection helpers + i18n key

**Files:**
- Create: `frontend/src/lib/selection.ts`
- Create: `frontend/src/__tests__/selection.test.ts`
- Modify: `frontend/src/i18n/locales/en.ts`
- Modify: `frontend/src/i18n/locales/zh-CN.ts`
- Modify: `frontend/src/i18n/locales/zh-TW.ts`
- Modify: `frontend/src/i18n/locales/ja.ts`

**Interfaces:**
- Produces:
  - `export type Selection = { type: 'none' } | { type: 'pending'; start: string } | { type: 'range'; start: string; end: string }`
  - `export type CellHighlight = 'start' | 'end' | 'in-range' | 'preview' | 'none'`
  - `export function makeRange(a: string, b: string): Extract<Selection, { type: 'range' }>` — auto-sorts so start ≤ end
  - `export function dayCount(start: string, end: string): number` — inclusive day count
  - `export function getCellHighlight(date: string, selection: Selection, hoverDate: string | null): CellHighlight`
  - New i18n key `'mosaic.range'` available in all four locales

---

- [ ] **Step 1: Write the failing tests**

```ts
// frontend/src/__tests__/selection.test.ts
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
```

- [ ] **Step 2: Run tests, confirm they fail**

```bash
cd frontend && pnpm test -- --reporter=verbose 2>&1 | grep -E "FAIL|Cannot find"
```

Expected: FAIL — `Cannot find module '../lib/selection'`

- [ ] **Step 3: Create `selection.ts`**

```ts
// frontend/src/lib/selection.ts

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
```

- [ ] **Step 4: Add `'mosaic.range'` to all four locale files**

In `frontend/src/i18n/locales/en.ts`, add after `'mosaic.viewing'`:
```ts
  'mosaic.range': '{start} → {end} ({days} days)',
```

In `frontend/src/i18n/locales/zh-CN.ts`, add after `'mosaic.viewing'`:
```ts
  'mosaic.range': '{start} → {end}（{days} 天）',
```

In `frontend/src/i18n/locales/zh-TW.ts`, add after `'mosaic.viewing'`:
```ts
  'mosaic.range': '{start} → {end}（{days} 天）',
```

In `frontend/src/i18n/locales/ja.ts`, add after `'mosaic.viewing'`:
```ts
  'mosaic.range': '{start} → {end}（{days} 日間）',
```

- [ ] **Step 5: Run tests, confirm all pass**

```bash
cd frontend && pnpm test -- --reporter=verbose
```

Expected:
```
✓ frontend/src/__tests__/selection.test.ts (18 tests)
✓ frontend/src/__tests__/i18n.test.ts (16 tests)
✓ frontend/src/__tests__/...
Test Files  4 passed (4)
```

The `i18n.test.ts` key-coverage tests will also verify the new `mosaic.range` key is present in all four locales.

- [ ] **Step 6: TS check**

```bash
cd frontend && pnpm tsc --noEmit
```

Expected: zero errors.

- [ ] **Step 7: Commit**

```bash
git add frontend/src/lib/selection.ts \
        frontend/src/__tests__/selection.test.ts \
        frontend/src/i18n/locales/en.ts \
        frontend/src/i18n/locales/zh-CN.ts \
        frontend/src/i18n/locales/zh-TW.ts \
        frontend/src/i18n/locales/ja.ts
git commit -m "feat(range): add Selection helpers and mosaic.range i18n key"
```

---

## Task 2: ContributionCalendar visual range highlight

**Files:**
- Modify: `frontend/src/components/contribution-calendar.tsx`

**Interfaces:**
- Consumes: `Selection`, `getCellHighlight` from `@/lib/selection` (Task 1)
- Produces: `ContributionCalendar` now accepts a required `selection: Selection` prop; renders per-cell highlight strokes using SVG attributes

---

- [ ] **Step 1: Read the current file**

```bash
cat -n frontend/src/components/contribution-calendar.tsx | head -10
```

Confirm the file starts with `import { useMemo, useState, useCallback, useRef, useEffect } from 'react';`

- [ ] **Step 2: Add imports to `contribution-calendar.tsx`**

After the last existing import line (the one importing from `'../api/types'`), add:

```ts
import { getCellHighlight } from '@/lib/selection';
import type { Selection } from '@/lib/selection';
```

- [ ] **Step 3: Add `selection` to `ContributionCalendarProps` and the component signature**

Change the interface and component signature together. The prop is optional with a default of `{ type: 'none' }` so the component is backwards-compatible and TS stays clean before App.tsx is updated in Task 3.

Change the interface from:
```ts
interface ContributionCalendarProps {
  data: Record<string, DailyReport>;
  onDayClick?: (date: string) => void;
  pricing?: PricingMap | null;
}
```

To:
```ts
interface ContributionCalendarProps {
  data: Record<string, DailyReport>;
  selection?: Selection;
  onDayClick?: (date: string) => void;
  pricing?: PricingMap | null;
}
```

Change the function signature from:
```ts
export function ContributionCalendar({ data, onDayClick, pricing }: ContributionCalendarProps) {
```

To:
```ts
export function ContributionCalendar({ data, selection = { type: 'none' }, onDayClick, pricing }: ContributionCalendarProps) {
```

- [ ] **Step 5: Replace the `<rect>` element with the highlighted version**

Find the current `<rect>` element (inside the nested `grid.map` → `week.map`):

```tsx
              <rect
                key={day.date}
                x={weekIndex * (CELL_SIZE + CELL_GAP) + 40}
                y={dayIndex * (CELL_SIZE + CELL_GAP) + 18}
                width={CELL_SIZE}
                height={CELL_SIZE}
                rx={2}
                fill={COLORS[intensity]}
                className="cursor-pointer hover:stroke-2 hover:stroke-foreground"
                onClick={() => onDayClick?.(day.date)}
                onMouseEnter={(e) => handleMouseEnter(day, e)}
                onMouseLeave={handleMouseLeave}
              />
```

Replace it with:

```tsx
              {(() => {
                const highlight = getCellHighlight(
                  day.date,
                  selection,
                  tooltip?.day.date ?? null,
                );
                return (
                  <rect
                    key={day.date}
                    x={weekIndex * (CELL_SIZE + CELL_GAP) + 40}
                    y={dayIndex * (CELL_SIZE + CELL_GAP) + 18}
                    width={CELL_SIZE}
                    height={CELL_SIZE}
                    rx={2}
                    fill={COLORS[intensity]}
                    stroke={
                      highlight === 'start' || highlight === 'end'
                        ? 'white'
                        : highlight === 'in-range' || highlight === 'preview'
                          ? 'currentColor'
                          : undefined
                    }
                    strokeWidth={
                      highlight === 'start' || highlight === 'end'
                        ? 2
                        : highlight === 'in-range'
                          ? 1
                          : highlight === 'preview'
                            ? 1
                            : undefined
                    }
                    opacity={highlight === 'preview' ? 0.7 : undefined}
                    className={`cursor-pointer${highlight === 'none' ? ' hover:stroke-2 hover:stroke-foreground' : ''}`}
                    onClick={() => onDayClick?.(day.date)}
                    onMouseEnter={(e) => handleMouseEnter(day, e)}
                    onMouseLeave={handleMouseLeave}
                  />
                );
              })()}
```

Note: the IIFE `{(() => { ... })()}` avoids introducing a new component just for the highlight variable. The `tooltip?.day.date` is the currently hovered date, which drives the pending-state preview.

- [ ] **Step 6: TS check**

```bash
cd frontend && pnpm tsc --noEmit
```

Expected: zero errors. Because `selection` is optional with a default value, `App.tsx` does not need to be updated yet — the existing call site compiles cleanly.

- [ ] **Step 7: Run tests**

```bash
cd frontend && pnpm test -- --reporter=verbose
```

Expected: all tests pass (the calendar has no unit tests to break, and the selection tests from Task 1 are unaffected).

- [ ] **Step 8: Commit**

```bash
git add frontend/src/components/contribution-calendar.tsx
git commit -m "feat(range): add visual range highlight to ContributionCalendar"
```

---

## Task 3: App.tsx — Selection state + range aggregation

**Files:**
- Modify: `frontend/src/App.tsx`

**Interfaces:**
- Consumes:
  - `Selection`, `makeRange`, `dayCount` from `@/lib/selection`
  - `useMemo` from `react`
  - `ContributionCalendar` now requires `selection: Selection` prop (Task 2)
- Produces: complete working feature end-to-end

---

- [ ] **Step 1: Add `useMemo` to the React import**

Change line 1 from:
```ts
import { useState, useEffect, useCallback, useRef } from 'react';
```
To:
```ts
import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
```

- [ ] **Step 2: Add the selection imports**

After the existing `import { useTranslation }` line, add:
```ts
import { makeRange, dayCount } from '@/lib/selection';
import type { Selection } from '@/lib/selection';
```

- [ ] **Step 3: Replace `selectedDate` state with `selection`**

Find (inside the `App` function body, near the other `useState` calls):
```ts
  const [selectedDate, setSelectedDate] = useState<string | null>(null);
```

Replace with:
```ts
  const [selection, setSelection] = useState<Selection>({ type: 'none' });
```

- [ ] **Step 4: Add `handleDayClick` callback**

After the `setView` state declaration, add:
```ts
  const handleDayClick = useCallback((date: string) => {
    setSelection((prev) => {
      if (prev.type === 'none' || prev.type === 'range') {
        return { type: 'pending', start: date };
      }
      // pending → confirm range (auto-sorted)
      return makeRange(prev.start, date);
    });
  }, []);
```

- [ ] **Step 5: Replace `activeRow` + `activeTotals` with a `useMemo`**

Find and remove these lines (currently after `const dailyTotals`):
```ts
  const activeRow = allDailyData?.days.find(
    (d) => d.date === (selectedDate ?? today)
  );

  const activeTotals = {
    totalTokens: activeRow?.totalTokens ?? 0,
    inputTokens: activeRow?.inputTokens ?? 0,
    cacheReadTokens: activeRow?.cacheReadTokens ?? 0,
    outputTokens: activeRow?.outputTokens ?? 0,
  };
```

Replace with:
```ts
  const { activeTotals, activeModelBreakdown } = useMemo(() => {
    if (selection.type === 'range') {
      const inRange = allDailyData?.days.filter(
        (d) => d.date >= selection.start && d.date <= selection.end,
      ) ?? [];

      // Aggregate tokens
      const totals = {
        totalTokens: inRange.reduce((s, r) => s + (r.totalTokens ?? 0), 0),
        inputTokens: inRange.reduce((s, r) => s + (r.inputTokens ?? 0), 0),
        cacheReadTokens: inRange.reduce((s, r) => s + (r.cacheReadTokens ?? 0), 0),
        outputTokens: inRange.reduce((s, r) => s + (r.outputTokens ?? 0), 0),
      };

      // Aggregate model breakdowns for cost estimate
      const breakdown: Array<{
        model: string;
        inputTokens: number;
        outputTokens: number;
        cacheReadTokens: number;
      }> = [];
      for (const row of inRange) {
        for (const item of row.modelBreakdown ?? []) {
          const existing = breakdown.find((a) => a.model === item.model);
          if (existing) {
            existing.inputTokens += item.inputTokens;
            existing.outputTokens += item.outputTokens;
            existing.cacheReadTokens += item.cacheReadTokens;
          } else {
            breakdown.push({ ...item });
          }
        }
      }

      return { activeTotals: totals, activeModelBreakdown: breakdown };
    }

    // pending or none: use single day
    const activeDate =
      selection.type === 'pending' ? selection.start : today;
    const row = allDailyData?.days.find((d) => d.date === activeDate);
    return {
      activeTotals: {
        totalTokens: row?.totalTokens ?? 0,
        inputTokens: row?.inputTokens ?? 0,
        cacheReadTokens: row?.cacheReadTokens ?? 0,
        outputTokens: row?.outputTokens ?? 0,
      },
      activeModelBreakdown: row?.modelBreakdown ?? [],
    };
  }, [selection, allDailyData, today]);
```

- [ ] **Step 6: Update the date label above the metric cards**

Find:
```tsx
            {selectedDate && (
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <span>{t('mosaic.viewing', { date: selectedDate })}</span>
                <button
                  onClick={() => setSelectedDate(null)}
                  className="rounded-sm opacity-70 hover:opacity-100"
                  aria-label={t('btn.backToday')}
                >
                  ×
                </button>
              </div>
            )}
```

Replace with:
```tsx
            {selection.type !== 'none' && (
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <span>
                  {selection.type === 'pending'
                    ? t('mosaic.viewing', { date: selection.start })
                    : t('mosaic.range', {
                        start: selection.start,
                        end: selection.end,
                        days: dayCount(selection.start, selection.end),
                      })}
                </span>
                <button
                  onClick={() => setSelection({ type: 'none' })}
                  className="rounded-sm opacity-70 hover:opacity-100"
                  aria-label={t('btn.backToday')}
                >
                  ×
                </button>
              </div>
            )}
```

- [ ] **Step 7: Update `activeRow?.modelBreakdown` in the cost card**

Find inside the cost `<Card>`:
```tsx
                    const activeCost = estimateCost(
                      activeTotals.inputTokens,
                      activeTotals.outputTokens,
                      activeTotals.cacheReadTokens,
                      activeRow?.modelBreakdown ?? [],
                      pricing
                    );
```

Replace `activeRow?.modelBreakdown ?? []` with `activeModelBreakdown`:
```tsx
                    const activeCost = estimateCost(
                      activeTotals.inputTokens,
                      activeTotals.outputTokens,
                      activeTotals.cacheReadTokens,
                      activeModelBreakdown,
                      pricing
                    );
```

- [ ] **Step 8: Update `ContributionCalendar` usage**

Find:
```tsx
            <ContributionCalendar
              data={calendarData as any}
              onDayClick={setSelectedDate}
              pricing={pricing}
            />
```

Replace with:
```tsx
            <ContributionCalendar
              data={calendarData as any}
              selection={selection}
              onDayClick={handleDayClick}
              pricing={pricing}
            />
```

- [ ] **Step 9: TS check**

```bash
cd frontend && pnpm tsc --noEmit
```

Expected: zero errors. Common issues to watch for:
- If TS complains about `activeRow` being referenced elsewhere — search the file for any remaining `activeRow` reference and remove it (the variable no longer exists after Step 5)
- `dayCount` expects two `string` arguments — `selection.start` and `selection.end` are strings ✓

- [ ] **Step 10: Run tests**

```bash
cd frontend && pnpm test -- --reporter=verbose
```

Expected: all tests pass.

- [ ] **Step 11: Commit**

```bash
git add frontend/src/App.tsx
git commit -m "feat(range): integrate Selection state and range aggregation in App"
```

---

## Acceptance Criteria

Manual verification after all tasks (`cargo tauri dev`):

- [ ] Click one mosaic cell → it gets a white stroke; metric cards show that day's data
- [ ] Hover other cells while first is selected → candidate range cells get a faint stroke preview
- [ ] Click a second cell → range confirmed; all cells in range show `currentColor` stroke; label shows `{start} → {end}（N 天）`; metric cards show aggregated totals for the range
- [ ] Click second cell **before** first (reverse order) → auto-sorted correctly
- [ ] Click the `×` → selection cleared, metric cards return to today
- [ ] Click any new cell after a confirmed range → starts a new pending selection
- [ ] TS: `cd frontend && pnpm tsc --noEmit` → zero errors
- [ ] Tests: `cd frontend && pnpm test` → all pass
