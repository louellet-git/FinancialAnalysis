# A-110: Mean-reversion t threshold 2.86

Updated: 2026-11-21. Commit: not yet made.

## Choice
A calibration reports mean reversion as "significant" if the t-statistic is 2.86 or more.
Below 2.86 it adds a warning that kappa and theta are weakly identified. 2.86 is the 5%
Dickey-Fuller critical value (constant, no trend). The normal value 1.96 is not used.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Calibration | `rust/src/ratemodels/calib.rs` lines 30-32, 377-383, 856 | `MR_T_THRESHOLD` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| 1.96 (normal) | Too small near a unit root (comment, `calib.rs` lines 30-31). |
| A formal unit root test with a p-value | Not in the code. Not known. |

## Why this choice
The comment in the code. Statistical convention. The t of least squares is `(1 - beta)/se(beta)`
and has the Dickey-Fuller form. For maximum likelihood the code uses `kappa/se(kappa)` with the same
value. That is an approximation (the code does not show that the distribution is the same).

## Effect on the result
Only the flag and the warning. The parameters do not change. No sensitivity number is known.

## Who decided and when
Not known. Ticket `T-20261119-stochastic-dcf-drivers` (2026-11-19).

## When to examine it again
When the flag is used to accept or refuse a calibrated parameter in a DCF.
