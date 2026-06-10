# Dashboard UI Improvements — Design Spec

**Date:** 2026-06-09

## Overview

Three UI improvements to the LLM Usage Dashboard before the Homebrew release:

1. Adaptive token units in top MetricCards
2. Cache Hit rate tooltip
3. Daily Report Table front-end pagination
4. MetricCard dual-line layout (Today / Historical total)

---

## 1. Adaptive Token Format

### Function: `formatTokens(n: number): string`

Location: `frontend/src/components/utils.ts` (new export alongside existing helpers)

Rules (based on raw token count `n`):

| Range | Unit | Divisor |
|---|---|---|
| n < 1,000,000 | — | `toLocaleString()` |
| 1,000,000 ≤ n < 100,000,000 | K | 1,000 |
| 100,000,000 ≤ n < 100,000,000,000 | M | 1,000,000 |
| n ≥ 100,000,000,000 | B | 1,000,000,000 |

Trailing zero trimming: divide by the unit divisor, call `.toFixed(3)`, wrap in `parseFloat()` to strip trailing zeros, then append the unit suffix.

```
Example: 1_500_000 → parseFloat((1500000/1000).toFixed(3)) + "K" → "1500K"
Example: 1_502_500 → parseFloat((1502500/1000).toFixed(3)) + "K" → "1502.5K"
Example: 500_000_000 → "500M"
```

**Scope:** Only used in MetricCard display values. Table cells (`fmt`) remain unchanged.

---

## 2. MetricCard Dual-Line Layout

### Data flow changes in `App.tsx`

Two new derived values computed from the Daily report (regardless of selected Tab):

- `todayRow`: find the entry in `dailyData?.days` where `row.date === today`
  - `today` = `new Date().toLocaleDateString('en-CA')` (yields `YYYY-MM-DD` in local timezone)
- `dailyTotals`: `dailyData?.totals` (unchanged, already computed)

Both respect the current source filter (`sourceKey`).

### MetricCard component changes

New props:

```ts
interface MetricCardProps {
  title: string;
  value: number | undefined;       // today's value (large)
  total: number | undefined;       // historical total (small gray)
  tooltip?: React.ReactNode;       // optional icon+tooltip in title area
}
```

Layout:

```
┌─────────────────────────────┐
│ Total Tokens                │   ← title (existing style)
│                             │
│ 1,234.5K                    │   ← text-2xl font-bold, formatTokens(value)
│ / 15,678.9K                 │   ← text-xs text-muted-foreground, "/ " + formatTokens(total)
└─────────────────────────────┘
```

- If `todayRow` is absent (no data for today), `value` falls back to `0`; the second line still shows the historical total.
- Est. Cost card follows the same layout: today's cost vs. total cost from all-days aggregated breakdown.

---

## 3. Cache Hit Rate Tooltip

### New component: `frontend/src/components/ui/tooltip.tsx`

Built on `@base-ui/react/tooltip`, following the same pattern as `popover.tsx`:

- `Tooltip` (Root)
- `TooltipTrigger` (Trigger)
- `TooltipContent` (Positioner + Popup with matching style: `rounded-lg bg-popover text-popover-foreground shadow-md ring-1 ring-foreground/10`)

The trigger has `delay={{ open: 200, close: 0 }}` for a slight hover delay.

### Cache Hit card

The `tooltip` prop of `MetricCard` renders inline with the title:

```
Cache Hit  ⓘ     ← Info icon (lucide-react), 14px, muted-foreground
```

Tooltip content: **`Cache hit rate: 73.2%`**

Calculation: `cacheReadTokens / totalTokens * 100`, using `todayRow` values. If `totalTokens === 0`, show `0%`. Formatted to one decimal place.

---

## 4. Daily Report Table Pagination

### Scope

Only `DailyTable` in `usage-table.tsx`. Monthly / Session / Blocks tables are unchanged.

### State & logic

```ts
const PAGE_SIZE = 20;
const [currentPage, setCurrentPage] = useState(1);
const totalPages = Math.ceil(data.length / PAGE_SIZE);
const pageData = data.slice((currentPage - 1) * PAGE_SIZE, currentPage * PAGE_SIZE);
```

The `currentPage` resets to `1` when the `data` prop changes (different source selected).

### Controls

Rendered below the table, centered:

```
[← Prev]   Page 2 / 7   [Next →]
```

- Uses existing `Button` component (`variant="outline"`, `size="sm"`)
- Prev disabled on page 1; Next disabled on last page
- Hidden entirely when `totalPages <= 1`

---

## File Change Summary

| File | Change |
|---|---|
| `frontend/src/components/utils.ts` | Add `formatTokens()` |
| `frontend/src/components/ui/tooltip.tsx` | New file, `@base-ui/react/tooltip` wrapper |
| `frontend/src/App.tsx` | Compute `todayRow`; update `MetricCard` calls with `value`+`total`+`tooltip`; update `MetricCard` component definition |
| `frontend/src/components/usage-table.tsx` | Add pagination state + controls to `DailyTable` |

---

## Non-goals

- No changes to the backend or API types
- No changes to table cell number formatting
- No pagination for Monthly / Session / Blocks tables
