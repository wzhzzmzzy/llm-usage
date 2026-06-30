# ADR 0001: Model Date Selection as a Discriminated Union

**Date:** 2026-06-29  
**Status:** Accepted

## Context

The mosaic supports clicking two cells to define a date range. This requires tracking three distinct states: no selection, one click pending a second, and a confirmed range. The pre-existing `selectedDate: string | null` only covered a single date.

Two modelling options were considered:

**Option A — Discriminated union (chosen)**
```ts
type Selection =
  | { type: 'none' }
  | { type: 'pending'; start: string }
  | { type: 'range'; start: string; end: string }
```

**Option B — Two nullable strings**
```ts
selectedDate: string | null   // doubles as rangeStart
rangeEnd: string | null
```

## Decision

Use the discriminated union (Option A). Replace `selectedDate` with a `selection: Selection` state in `App.tsx`.

## Rationale

Option B reuses `selectedDate` as rangeStart, which makes `null` ambiguous (no selection vs. cleared) and requires callers to infer the current mode from a combination of two fields. The discriminated union makes the three modes explicit and exhaustively checkable — a `switch` on `selection.type` is a compile-time guarantee that all cases are handled. The metric-card aggregation logic, the mosaic highlight props, and the date label copy all branch on the same type, so a single exhaustive check is significantly cleaner than scattered `null` checks across three separate concerns.

## Consequences

- `selectedDate` is removed; components that consumed it receive the new `selection` prop or derived values
- The `ContributionCalendar` component gains `selection` and `onHover` props to drive the preview highlight during the pending state
- Aggregated metric calculation must handle the `range` case by summing all `DailyRow` entries where `start ≤ date ≤ end`
