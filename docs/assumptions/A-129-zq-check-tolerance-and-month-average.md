# A-129: ZQ check (5 bp tolerance, month average with the last rate carried)

Updated: 2026-11-21. Commit: not yet made.

## Choice
- The implied rate is `100 - price` of the Yahoo bar of the 30-day fed funds contract of the month of the date (decimal).
- The expected rate is the mean of the daily EFFR over all calendar days of that month. A day with no published rate (weekend, holiday, future) has the last published rate.
- The check fails when the two differ by more than 0.0005 (5 basis points).
- The EFFR window starts 7 days before the month.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Futures check (`docs/models/futures-check.md`) | `rust/src/futures/check.rs` | `ZQ_TOLERANCE` |
| Futures check | `rust/src/futures/check.rs` | `expected_average` |
| Futures check | `rust/src/futures/check.rs` | the start (first day minus 7) |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| A forward EFFR path from the market | Not known. No second source is in the code. |
| A smaller tolerance | Not known. The test cases are 4.9 bp (pass) and 6 bp (fail). |

## Why this choice
`docs/HIDDEN-ASSUMPTIONS.md` #76 names the tolerance (5 bp) and the weak point: the check can fail when the FOMC changes the target range inside the month.
The code comment gives the same warning (`check.rs`). No study of the width 5 bp: author choice. The 7 days of start: not known.

## Effect on the result
Example: the target moves by 25 bp on day d after the date of the check. The price already includes it, but the expected rate carries the old rate.
The difference is 25 x (days from d to the month end) / (days in the month) bp. In a 30-day month it is above 5 bp when the change is more than 6 days before the month end.
So a correct price can fail. This is arithmetic, not a measurement. No run is recorded.

## Who decided and when
Not known. The check is from the commit `c6adfa7`; no person is named. The entry #76 is the record of the limit.

## When to examine it again
When the check fails in a month with an FOMC meeting; when a price of a forward EFFR path is available.
