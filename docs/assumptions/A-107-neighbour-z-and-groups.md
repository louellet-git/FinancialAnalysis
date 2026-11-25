# A-107: Neighbours of the z-score and the group rules

Updated: 2026-11-21. Commit: not yet made.

## Choice
- `z` = (residual - median residual of the 20 securities nearest in maturity) / robust sigma
  of those 20, with a floor of 0.3 bp. Bills are compared with bills only.
- Group: the first rule that fits, in this order:
  1. Under 3 months.
  2. Recent issue: remaining life from the term minus 0.35 year to the term plus 0.05 year, for the terms 2, 3, 5, 7, 10, 20, 30 years, and a coupon within 0.5 percentage point of the yield.
  3. Old 30-year bond: a bond that matures before 2040-01-01.
  4. High coupon: the coupon is 1.25 points or more above the median coupon of the neighbours.
  5. Low coupon: the coupon is 1.25 points or more below that median.
  6. Near coupon date: the next coupon is in 10 days or less.
  7. Other.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Rich and cheap (`docs/models/treasury-fit-rich-cheap.md`) | `rust/src/richcheap.rs` | `neighbours(.., 20)`, `mad.max(0.3)`, `0.0125`, `0.35`, `0.005`, 2040 |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Other neighbour count or limits | Not known. |
| Issue date from a file | The file has no issue date (comment in `richcheap.rs`). |

## Why this choice
Author choice, no study. The rule for the old 30-year bond: the Treasury issued 20-year
bonds again from 2020, which mature from 2040 (comment ). The group
names the likely cause only. The command does not test the cause (`richcheap.rs` `limits`).

## Effect on the result
Not known. The group changes a label, not a residual. The 10 largest lasting residuals are
9 "old 30-year" and 1 "other" on 2026-11-18 (`rates_recon_step2_2026-11-20.md`, section 3.4).

## Who decided and when
Not known. Ticket `T-20261120-arb-2-rates-recon` (2026-11-20).

## When to examine it again
When `fa recon explain` gives a cause for the group effect (not run, see `HIDDEN-ASSUMPTIONS.md` #112).
