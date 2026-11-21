# A-203: HQM month for a date, and the use of the Treasury 10Y of the date

Updated: 2026-11-21. Commit: not yet made.

## Choice
For a date D the cost of debt uses the HQM month (dated the 1st) that is the latest on or
before D, at most 70 days old. The Treasury 10Y is the value of D (or the last day before D).
So `rf + kd_spread` equals the cost of debt (HQM + step) exactly.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Cost of debt | `rust/src/bonds.rs` | `MONTHLY_MAX_AGE`, `spread_over_10y()` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Treasury 10Y on the HQM month date | The date is the 1st of a monthly average, not a day. It would give a spread that mixes a month and a day |
| No age limit | A gap in the data would give an old number without a word |

## Why this choice
The cost of debt is a level; the spread is only the form that the engine needs. The level
is then the HQM level and the step. Convention.

## Effect on the result
Inside a month the HQM level does not move, so the cost of debt does not follow the daily
change of the Treasury. The spread moves with `rf`: `kd` stays the same.

## Who decided and when
The Technology head, 2026-11-21.

## When to examine it again
If a daily corporate source with open terms exists.
