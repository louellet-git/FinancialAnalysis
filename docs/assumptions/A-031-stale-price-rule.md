# A-031: stale-price rule (a close older than the SPY close)

Updated: 2026-11-21. Commit: 2e70201 (branch `claude/premium-series`).
Model document: `docs/models/implied-and-attribute.md`.

## Choice
For a date, the last close of a ticker on or before the date is stale when it is older than the last SPY close on or
before the same date. A stale close at D1 or D2 takes the ticker out of every sum of the bridge. The row and the CSV
keep the ticker with a flag. In `fa dcf premium-series`, the same rule takes the ticker out of ret_w. The list of the
tickers left out is printed (rule of the code, being added by another change).

## Where used
| Model | File | Parameter name |
|---|---|---|
| Attribution | `rust/src/dcf/auto_attribute.rs` | `stale_note`, `mark_stale`, `finish_weights` |
| Premium series | `rust/src/dcf/auto_premium.rs` | `week_return` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Take the last close up to 10 days older (A-012) | It uses a stale price. IBM and JNJ had no close after 2026-11-18 |
| Fixed number of days | SPY is the trading calendar, so no day count is needed |

## Why this choice
Data defect 3 of `PLANNING/board/T-20261121-sp100-week-attribution.md` (2026-11-21): a stale close gives a false
price change. The owner asked for a fix of data problems. SPY is the calendar of the series.

## Effect on the result
Not known as a number. IBM and JNJ left the sums of the run to 2026-11-20.

## Who decided and when
Head of Fundamental Research, 2026-11-21.

## When to examine it again
When the store has a close for each ticker on each trading day, or when a halted ticker is in a list.
