# A-103: Which securities enter the curve fit

Updated: 2026-11-21. Commit: not yet made.

## Choice
In the fit: notes and bonds (`MARKET BASED NOTE`, `MARKET BASED BOND`) with a coupon above 0,
a positive end-of-day price, no call date, and a life of 0.25 year (3 months) or more from
settlement. Out of the fit: bills, TIPS and floating rate notes, callable bonds, zero coupon
securities, securities with no yield solution. The panel of `fa recon history` uses a life of
0.5 year (6 months) or more and no bills.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Treasury fit | `rust/src/fit.rs` lines 253-295 (`select`) | `min_years` (0.25) |
| Rich and cheap | `rust/src/richcheap.rs` line 145, lines 155-170 | `select(settle, quotes, 0.25)` |
| History panel | `rust/src/history.rs` lines 199 and 341 | `r.years >= 0.5` |
| Bond listing | `rust/src/recon.rs` lines 84-95 | under 7 days skipped |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Add bills to the fit | Not known. Bills are a different market (money market demand). |
| Other minimum life | Not known. No study. |

## Why this choice
Convention for a coupon curve: the curve is for notes and bonds (`fit.rs` lines 3-5).
The minimum life of 3 months and 6 months is an author choice, no study.

## Effect on the result
Bills of 6 months and more price 4 to 12 bp rich against the note curve, so the exclusion
changes any curve that has bills in it (`rates_recon_step2_2026-11-20.md`, sections 3.2 and 4;
3 dates, 1 to 6 bills: hypothesis, not proved). No other sensitivity is known.

## Who decided and when
Not known. Ticket `T-20261120-arb-2-rates-recon` (2026-11-20).

## When to examine it again
When bills are needed in the curve, or when the 6-month panel limit hides a short-end effect.
