# A-027: curve rule of the premium series (GSW at most 3 days older, else par)

Updated: 2026-11-21. Commit: 2e70201 (branch `claude/premium-series`).
Model document: `docs/models/premium-series.md`.

## Choice
The curve of an anchor is the GSW row when GSW has a row on or before the anchor that is at most 3 days older
(`CURVE_LAG_DAYS` = 3, unit: days). Else the curve is the NSS fit to the Treasury par yield curve (`par`) of the last
par row on or before the anchor. The first week with the par fit is the switch week. The code prints it.

`fa dcf attribute` gets the same rule as `--curve auto`, its default. GSW is used at
both dates when its row is at most 3 days older than each date. Else the par fit is used at both dates (one source
at both dates). The text names the source. With GSW at both dates the result equals `--curve gsw`.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Premium series | `rust/src/dcf/auto_premium.rs` | `CURVE_LAG_DAYS`, `curve_source` |
| Attribution flag (same value) | `rust/src/dcf/auto_attribute.rs` | `CURVE_LAG_DAYS` (A-012) |
| Attribution `--curve auto` | `rust/src/dcf/auto_attribute.rs` | `auto_curve_source`, `CURVE_LAG_DAYS` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| GSW only | GSW lags about one week; the last weeks have no row |
| Par fit only | A history from 2019 needs the same curve family as the main work; reason not known in a file |
| `fedinvest` fit | Not known. The command `attribute` has it as an option |

## Why this choice
The design of the Head of Economics (rule 3) wants GSW for the history and the par fit for the last weeks, and
the curve and its date for each Friday. The value 3 equals the flag limit of A-012. No source for the value 3 is
known.

## Effect on the result
Not known. No sensitivity number exists. The switch week can show a step in `zero10` and in S1.

## Who decided and when
Head of Fundamental Research, 2026-11-21, after the design of the Head of Economics (`analysis/macro/2026-11-21-premium-macro-design.md`, rule 3).

## When to examine it again
When GSW has no lag, or when the first run shows a step at the switch week.

## Limits of the automatic choice (added 2026-11-21, gate note)
- The choice depends on how old the GSW file of the cache is. A past week and a current week can thus have different
  sources only because the cache was made at a different time. The GSW row of a date is published about one business day
  later; the 3-day rule gives the same choice.
- A window whose second date is newer than the GSW file gets the par fit at both dates by default.
- Do not chain weeks across a change of the source. Read the line CURVE SOURCE of each run first.
