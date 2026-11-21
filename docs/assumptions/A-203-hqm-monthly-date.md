# A-203: HQM month for a date, publication lag and the basis

Updated: 2026-11-21. Commit: a66a70d (step 3), fix commit to follow.
## Choice
1. A HQM month is used from its publication day: the first weekday after the 5th of the next
   month (`hqm_available`). Before that day the month does not exist for a person. On 2026-10-15
   the month is September, not October.
2. Evidence for the lag: the file `hqm_24_28.xls` with October 2026 as its last month has
   `Last-Modified: Wed, 04 Nov 2026 21:38:48 GMT` (one HEAD request, 2026-11-21). The Treasury
   page gives no schedule. The rule day (2026-11-06) is 2 days after that one observation. A bank
   holiday is not counted. One observation is a thin base: check again when the next month appears.
3. The 70 days (`MONTHLY_MAX_AGE`) is a staleness cap, not the lag. The newest published month
   must end at most 70 days before the date; an older month gives an error that names
   `fa bonds hqm get`.
4. The cost of debt is `UST 10Y (date) + basis + step`. The basis is the HQM 10Y of the month minus
   the mean UST 10Y of the same month (monthly average against monthly average). The HQM level
   is an average and the UST of the date is a close: the old form (HQM level plus a step, minus
   a close) was biased high by 0.2 to 0.3 points when the UST fell inside the month.
5. `rf + kd_spread` is the cost of debt again, with `kd_spread = basis + step`.
Model document: `docs/models/cost-of-debt.md`.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Cost of debt | `rust/src/bonds.rs` | `hqm_available()`, `pick_month()`, `MONTHLY_MAX_AGE`, `hqm_basis()` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Use the month from its 1st day | Look-ahead: the average does not exist yet |
| HQM level minus the UST of the date | Mixes an average with a close; bias of 0.2 to 0.3 points |
| End-of-month HQM files (the page lists them) | Not read; a later step if a daily-like level is needed |

## Why this choice
No look-ahead, and the same kind of number on both sides of the spread. Convention for the lag.

## Effect on the result
At 2026-11-20 for A-: basis 0.89 points, kd 5.92% (the old form gave 6.02%). The check of the
whole kd against a second source is in the model document.

## Who decided and when
The Technology head, 2026-11-21, after the QA gate and the Quant review of ticket `T-20261121-fred-local`.

## When to examine it again
When a second observation of the publication day exists, or if a daily corporate source with open terms exists.
