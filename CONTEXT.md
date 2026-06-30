# Domain Glossary

## Active Period

The time window currently reflected in the metric cards (Total Tokens, Input, Cache Hit, Output, Est. Cost).

Derived from the current **Date Selection**:
- No selection → today's date
- Pending selection → the single day of the first click
- Confirmed range → all days between start and end (inclusive), aggregated

## Date Selection

The user's interaction state with the mosaic. Modelled as a discriminated union:

- `{ type: 'none' }` — default; **Active Period** is today
- `{ type: 'pending'; start: string }` — one mosaic cell clicked; awaiting a second click
- `{ type: 'range'; start: string; end: string }` — two cells clicked; start ≤ end (auto-sorted regardless of click order)

Cleared by the × button next to the date label above the metric cards. Any new click on the mosaic resets the selection and begins a new pending state.

## Range Selection

The two-click interaction on the mosaic that produces a `range` **Date Selection**. The first click anchors the start; the second click confirms the end. Dates are always sorted so start ≤ end. Hovering during the pending state previews the candidate range on the mosaic.

## Mosaic

The contribution-calendar heatmap that displays per-day token usage intensity over the past 53 weeks. Each cell represents one calendar day. Clicking cells drives **Date Selection**.
