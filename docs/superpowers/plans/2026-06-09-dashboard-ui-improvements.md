# Dashboard UI Improvements Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add adaptive token units, a dual-line today/total MetricCard layout, a Cache Hit rate tooltip, and Daily table pagination to the LLM Usage Dashboard frontend.

**Architecture:** Pure frontend changes across four files. A new `formatTokens` utility drives the adaptive units. `MetricCard` grows two props (`value` = today, `total` = historical) and an optional `tooltip` slot. A new `tooltip.tsx` UI component mirrors the existing `popover.tsx` pattern using `@base-ui/react/tooltip`. `DailyTable` adds local pagination state.

**Tech Stack:** React 19, TypeScript, Tailwind CSS v4, `@base-ui/react`, lucide-react, Vitest

---

## File Map

| Action | Path | Responsibility |
|---|---|---|
| **Create** | `frontend/src/components/utils.ts` | `formatTokens()` pure utility |
| **Create** | `frontend/src/components/ui/tooltip.tsx` | Base UI tooltip wrapper |
| **Create** | `frontend/src/__tests__/formatTokens.test.ts` | Unit tests for `formatTokens` |
| **Modify** | `frontend/src/App.tsx` | MetricCard props, today/total data, cache-hit tooltip |
| **Modify** | `frontend/src/components/usage-table.tsx` | DailyTable pagination |

---

## Task 1: `formatTokens` utility + tests (TDD)

**Files:**
- Create: `frontend/src/components/utils.ts`
- Create: `frontend/src/__tests__/formatTokens.test.ts`

### Adaptive unit rules

| Range | Unit | Divisor |
|---|---|---|
| n < 1 000 000 | — | `toLocaleString()` |
| 1 000 000 ≤ n < 100 000 000 | K | 1 000 |
| 100 000 000 ≤ n < 100 000 000 000 | M | 1 000 000 |
| n ≥ 100 000 000 000 | B | 1 000 000 000 |

Trailing zero trimming: `parseFloat((v / divisor).toFixed(3))` — `parseFloat` drops trailing zeros automatically.

---

- [ ] **Step 1.1: Write failing tests**

Create `frontend/src/__tests__/formatTokens.test.ts`:

```ts
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
```

- [ ] **Step 1.2: Run tests to confirm they fail**

```bash
cd frontend && pnpm test --reporter=verbose 2>&1 | grep -A 3 "formatTokens"
```

Expected: `formatTokens` tests all fail with `Cannot find module '../components/utils'`.

- [ ] **Step 1.3: Implement `formatTokens`**

Create `frontend/src/components/utils.ts`:

```ts
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
```

- [ ] **Step 1.4: Run tests to confirm they pass**

```bash
cd frontend && pnpm test --reporter=verbose 2>&1 | grep -A 3 "formatTokens"
```

Expected: all `formatTokens` tests PASS.

- [ ] **Step 1.5: Commit**

```bash
git add frontend/src/components/utils.ts frontend/src/__tests__/formatTokens.test.ts
git commit -m "feat: add formatTokens utility with adaptive SI units"
```

---

## Task 2: Tooltip UI component

**Files:**
- Create: `frontend/src/components/ui/tooltip.tsx`

This follows the identical pattern as `frontend/src/components/ui/popover.tsx` which wraps `@base-ui/react/popover`. The tooltip module exports `Tooltip.Provider`, `Tooltip.Root`, `Tooltip.Trigger`, `Tooltip.Portal`, `Tooltip.Positioner`, and `Tooltip.Popup`.

- [ ] **Step 2.1: Create `tooltip.tsx`**

Create `frontend/src/components/ui/tooltip.tsx`:

```tsx
import * as React from "react"
import { Tooltip as TooltipPrimitive } from "@base-ui/react/tooltip"

import { cn } from "@/lib/utils"

function TooltipProvider({
  delay = 200,
  ...props
}: TooltipPrimitive.Provider.Props) {
  return <TooltipPrimitive.Provider delay={delay} {...props} />
}

function Tooltip({ ...props }: TooltipPrimitive.Root.Props) {
  return <TooltipPrimitive.Root data-slot="tooltip" {...props} />
}

function TooltipTrigger({ ...props }: TooltipPrimitive.Trigger.Props) {
  return <TooltipPrimitive.Trigger data-slot="tooltip-trigger" {...props} />
}

function TooltipContent({
  className,
  side = "top",
  sideOffset = 6,
  ...props
}: TooltipPrimitive.Popup.Props &
  Pick<TooltipPrimitive.Positioner.Props, "side" | "sideOffset">) {
  return (
    <TooltipPrimitive.Portal>
      <TooltipPrimitive.Positioner
        side={side}
        sideOffset={sideOffset}
        className="isolate z-50"
      >
        <TooltipPrimitive.Popup
          data-slot="tooltip-content"
          className={cn(
            "z-50 origin-(--transform-origin) rounded-md bg-popover px-3 py-1.5 text-xs text-popover-foreground shadow-md ring-1 ring-foreground/10 outline-hidden duration-100 data-[side=bottom]:slide-in-from-top-2 data-[side=top]:slide-in-from-bottom-2 data-[side=left]:slide-in-from-right-2 data-[side=right]:slide-in-from-left-2 data-open:animate-in data-open:fade-in-0 data-open:zoom-in-95 data-closed:animate-out data-closed:fade-out-0 data-closed:zoom-out-95",
            className
          )}
          {...props}
        />
      </TooltipPrimitive.Positioner>
    </TooltipPrimitive.Portal>
  )
}

export { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger }
```

- [ ] **Step 2.2: Type-check**

```bash
cd frontend && pnpm exec tsc --noEmit 2>&1 | grep -i "tooltip" || echo "No tooltip errors"
```

Expected: no errors related to `tooltip.tsx`.

- [ ] **Step 2.3: Commit**

```bash
git add frontend/src/components/ui/tooltip.tsx
git commit -m "feat: add Tooltip UI component (base-ui/react)"
```

---

## Task 3: MetricCard dual-line layout + Cache Hit tooltip

**Files:**
- Modify: `frontend/src/App.tsx`

### Changes overview

1. Import `formatTokens` and new tooltip components + `Info` icon.
2. Update `MetricCard` props: `value` (today, large) + `total` (historical, small gray) + optional `tooltip`.
3. Compute `todayRow` and `allDailyAggregatedBreakdown` from `allDailyData`.
4. Update all four token `MetricCard` calls to pass today + total.
5. Update Est. Cost card inline with the same dual-line layout.
6. Add Cache Hit tooltip node.

---

- [ ] **Step 3.1: Update imports in `App.tsx`**

Find the current imports block at the top of `frontend/src/App.tsx` and make these changes:

Add `Info` to the lucide-react import (line 25):
```tsx
import { Check, ChevronsUpDown, RefreshCw, BarChart3, Table, Layers, Users, Info } from 'lucide-react';
```

Add after the existing UI component imports:
```tsx
import { formatTokens } from '@/components/utils';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
```

- [ ] **Step 3.2: Update `MetricCard` component**

Replace the existing `MetricCard` function (lines 193–212) with:

```tsx
function MetricCard({
  title,
  value,
  total,
  tooltip,
}: {
  title: string;
  value: number | undefined;
  total: number | undefined;
  tooltip?: React.ReactNode;
}) {
  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="text-sm font-medium text-muted-foreground flex items-center gap-1">
          {title}
          {tooltip}
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="text-2xl font-bold">{formatTokens(value)}</div>
        <div className="text-xs text-muted-foreground mt-0.5">
          / {formatTokens(total)}
        </div>
      </CardContent>
    </Card>
  );
}
```

- [ ] **Step 3.3: Add today/total derivation in the `App` component**

Add these derived values inside the `App` function, right after the existing `sourceKey` / `dailyData` / … lines (after line 313 where `blocksData` is defined):

```tsx
// Today's date in YYYY-MM-DD (local timezone, e.g. "2026-06-09")
const today = new Date().toLocaleDateString('en-CA');

// Always derived from the Daily report regardless of selected tab
const allDailyData = snapshot?.daily?.[`${sourceKey}_daily`];
const todayRow = allDailyData?.days.find((d) => d.date === today);
const dailyTotals = allDailyData?.totals;

const todayTotals = {
  totalTokens: todayRow?.totalTokens ?? 0,
  inputTokens: todayRow?.inputTokens ?? 0,
  cacheReadTokens: todayRow?.cacheReadTokens ?? 0,
  outputTokens: todayRow?.outputTokens ?? 0,
};

// Aggregate all-days model breakdown for the historical Est. Cost
const allDailyAggregatedBreakdown: Array<{
  model: string;
  inputTokens: number;
  outputTokens: number;
  cacheReadTokens: number;
}> = [];
for (const row of allDailyData?.days ?? []) {
  if (!row.modelBreakdown) continue;
  for (const item of row.modelBreakdown) {
    const existing = allDailyAggregatedBreakdown.find((a) => a.model === item.model);
    if (existing) {
      existing.inputTokens += item.inputTokens;
      existing.outputTokens += item.outputTokens;
      existing.cacheReadTokens += item.cacheReadTokens;
    } else {
      allDailyAggregatedBreakdown.push({
        model: item.model,
        inputTokens: item.inputTokens,
        outputTokens: item.outputTokens,
        cacheReadTokens: item.cacheReadTokens,
      });
    }
  }
}

// Cache hit rate for today (shown in the Cache Hit tooltip)
const cacheHitRate =
  todayTotals.totalTokens > 0
    ? ((todayTotals.cacheReadTokens / todayTotals.totalTokens) * 100).toFixed(1)
    : '0.0';

const cacheHitTooltip = (
  <TooltipProvider>
    <Tooltip>
      <TooltipTrigger className="text-muted-foreground/60 hover:text-muted-foreground cursor-default">
        <Info className="h-3.5 w-3.5" />
      </TooltipTrigger>
      <TooltipContent>Cache hit rate: {cacheHitRate}%</TooltipContent>
    </Tooltip>
  </TooltipProvider>
);
```

- [ ] **Step 3.4: Replace the MetricCards grid in the JSX**

Find the existing MetricCards grid (the `{totals && (` block, lines 433–468) and replace it entirely with:

```tsx
{(todayTotals || dailyTotals) && (
  <div className="grid grid-cols-2 sm:grid-cols-5 gap-4">
    <MetricCard
      title="Total Tokens"
      value={todayTotals.totalTokens}
      total={dailyTotals?.totalTokens}
    />
    <MetricCard
      title="Input"
      value={todayTotals.inputTokens}
      total={dailyTotals?.inputTokens}
    />
    <MetricCard
      title="Cache Hit"
      value={todayTotals.cacheReadTokens}
      total={dailyTotals?.cacheReadTokens}
      tooltip={cacheHitTooltip}
    />
    <MetricCard
      title="Output"
      value={todayTotals.outputTokens}
      total={dailyTotals?.outputTokens}
    />
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="text-sm font-medium text-muted-foreground">
          Est. Cost
        </CardTitle>
      </CardHeader>
      <CardContent>
        {(() => {
          const todayCost = estimateCost(
            todayTotals.inputTokens,
            todayTotals.outputTokens,
            todayTotals.cacheReadTokens,
            todayRow?.modelBreakdown ?? [],
            pricing
          );
          const totalCost = estimateCost(
            dailyTotals?.inputTokens ?? 0,
            dailyTotals?.outputTokens ?? 0,
            dailyTotals?.cacheReadTokens ?? 0,
            allDailyAggregatedBreakdown,
            pricing
          );
          return (
            <>
              <div className="text-2xl font-bold">
                {formatCost(todayCost.cost)}
                {!todayCost.matched && pricing && (
                  <span className="text-xs text-muted-foreground ml-2">(default)</span>
                )}
              </div>
              <div className="text-xs text-muted-foreground mt-0.5">
                / {formatCost(totalCost.cost)}
              </div>
            </>
          );
        })()}
      </CardContent>
    </Card>
  </div>
)}
```

- [ ] **Step 3.5: Type-check**

```bash
cd frontend && pnpm exec tsc --noEmit 2>&1 | grep -v "^$" || echo "No errors"
```

Expected: no TypeScript errors.

- [ ] **Step 3.6: Commit**

```bash
git add frontend/src/App.tsx
git commit -m "feat: MetricCard shows today vs historical total with cache-hit rate tooltip"
```

---

## Task 4: Daily Report Table pagination

**Files:**
- Modify: `frontend/src/components/usage-table.tsx`

Pagination lives entirely in `DailyTable`. No changes to Monthly, Session, or Blocks tables.

- [ ] **Step 4.1: Add pagination state and sliced data to `DailyTable`**

In `frontend/src/components/usage-table.tsx`, find the `DailyTable` function definition. Add these lines right after the existing `const [expanded, setExpanded] = useState<ExpandedState>({});` line:

```tsx
const PAGE_SIZE = 20;
const [currentPage, setCurrentPage] = useState(1);
const totalPages = Math.max(1, Math.ceil(data.length / PAGE_SIZE));
const pageData = data.slice((currentPage - 1) * PAGE_SIZE, currentPage * PAGE_SIZE);
```

- [ ] **Step 4.2: Render `pageData` instead of `data`**

In `DailyTable`, find the `data.map((row) => {` call inside `<TableBody>`. Change it to:

```tsx
pageData.map((row) => {
```

(There is only one `.map` on `data` in `DailyTable`, so this is unambiguous.)

- [ ] **Step 4.3: Add pagination controls below the table**

In `DailyTable`, find the closing `</Table>` tag and add pagination controls after it:

```tsx
    </Table>
    {totalPages > 1 && (
      <div className="flex items-center justify-center gap-4 pt-4">
        <Button
          variant="outline"
          size="sm"
          onClick={() => setCurrentPage((p) => Math.max(1, p - 1))}
          disabled={currentPage === 1}
        >
          ← Prev
        </Button>
        <span className="text-sm text-muted-foreground">
          Page {currentPage} / {totalPages}
        </span>
        <Button
          variant="outline"
          size="sm"
          onClick={() => setCurrentPage((p) => Math.min(totalPages, p + 1))}
          disabled={currentPage === totalPages}
        >
          Next →
        </Button>
      </div>
    )}
```

- [ ] **Step 4.4: Add `Button` import to `usage-table.tsx`**

At the top of `frontend/src/components/usage-table.tsx`, add `Button` to the existing UI imports:

```tsx
import { Button } from '@/components/ui/button';
```

- [ ] **Step 4.5: Reset page when source changes**

In `frontend/src/components/usage-table.tsx`, inside the `UsageTable` switch, add a `key` prop to reset `DailyTable` state when `source` changes:

```tsx
case 'daily':
  return (
    <DailyTable
      key={source}
      data={data as DailyRow[]}
      snapshot={snapshot}
      source={source}
      pricing={pricing}
    />
  );
```

- [ ] **Step 4.6: Type-check and run all tests**

```bash
cd frontend && pnpm exec tsc --noEmit 2>&1 | grep -v "^$" || echo "No TS errors"
pnpm test 2>&1 | tail -5
```

Expected: no TS errors, all tests pass.

- [ ] **Step 4.7: Commit**

```bash
git add frontend/src/components/usage-table.tsx
git commit -m "feat: add front-end pagination to Daily Report table (20 rows/page)"
```

---

## Self-Review Checklist

### Spec coverage

| Spec requirement | Covered by |
|---|---|
| Adaptive units 1M–100M → K | Task 1 |
| Adaptive units 100M–100B → M | Task 1 |
| Adaptive units 100B+ → B | Task 1 |
| Trailing zeros trimmed | Task 1 |
| Cache Hit title: info icon + hover tooltip | Task 3 |
| Tooltip shows cache hit rate | Task 3 |
| MetricCard large = Today | Task 3 |
| MetricCard small gray = historical total with "/" prefix | Task 3 |
| Historical total always from Daily report totals | Task 3 (allDailyData) |
| Today always from Daily report regardless of tab | Task 3 (allDailyData) |
| Daily table 20 rows/page | Task 4 |
| Pagination resets on source change | Task 4 (key prop) |
| Tooltip built with @base-ui/react | Task 2 |

### No placeholders scan

All steps contain complete code. No TBDs.

### Type consistency

- `formatTokens(n: number | undefined): string` — used in Task 3 MetricCard
- `MetricCard` props `value: number | undefined`, `total: number | undefined`, `tooltip?: React.ReactNode` — consistent across Tasks 3.2–3.4
- `cacheHitTooltip` is `JSX.Element` which satisfies `React.ReactNode` — consistent
- `allDailyAggregatedBreakdown` shape matches what `estimateCost` `modelBreakdown` parameter expects — consistent
- `key={source}` on `DailyTable` — `source` is `string | undefined`, valid React key — consistent
