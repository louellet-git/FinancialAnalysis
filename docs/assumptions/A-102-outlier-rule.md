# A-102: Outlier rule of the curve fit

Updated: 2026-11-21. Commit: not yet made.

## Choice
The fit runs 4 times (3 exclusion passes). After each of the first 3 fits, a security is left
out of the next fit if its residual differs from the median residual by more than
4 robust sigma. Robust sigma is `1.4826 x` the median absolute deviation, with a floor of 0.5 bp.
A security that was left out can return in a later pass. The fit needs 12 securities or more.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Treasury fit | `rust/src/fit.rs` lines 308-340 (`fit_robust`) | passes 0..4, `4.0 * sigma`, `.max(0.5)`, `< 12` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| No exclusion | One bad price moves the curve (`fit.rs` lines 21-22). |
| Other limit or number of passes | Not known. No test in the repo. |

## Why this choice
Comment `fit.rs` lines 21-22: one bad price must not move the curve. The values 4, 3 and 0.5
are an author choice, no study. Test `one_bad_price_is_left_out_and_does_not_move_the_curve`
(a price 1.2 points high is left out).

## Effect on the result
Not known. No run with another limit.
The number of left-out securities is printed as `outliers` (`Excluded`).

## Who decided and when
Not known. Ticket `T-20261120-arb-2-rates-recon` (2026-11-20).

## When to examine it again
When many securities of one group (for example the old 30-year bonds) are left out, because
the rule then removes a real effect from the curve and shows it as a residual.
