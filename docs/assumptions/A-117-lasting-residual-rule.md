# A-117: Rule for a lasting residual

Updated: 2026-11-21. Commit: not yet made.

## Choice
A security has a lasting residual if all four hold, over the dates of the panel:
1. it has 38 dates or more (`min_dates`);
2. the residual has the sign of its mean on 90% of its days or more (`same_sign` 0.9; a zero day does not count);
3. `|mean residual|` is above its mean cost (A-106);
4. `|mean residual|` is above 1.9 bp (`min_abs_bp`, the mean fit RMSE of the 58 days).
Units: bp of yield. Minus is rich, plus is cheap (A-105). The tests use exact values, with no rounding.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Rich and cheap (`docs/models/treasury-fit-rich-cheap.md`) | `rust/src/history.rs` (`LastingRule`, `lasting`) | `min_dates`, `same_sign`, `min_abs_bp` |
| Command | `fa recon history --lasting` | `--min-dates`, `--same-sign`, `--min-abs-bp` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| A unit-root or mean-reversion test of each security | 58 days are too few. Not run. |
| A limit from the RMSE of each day | The mean RMSE is one number; the rule stays simple. |
| 2/3 of the dates as a function | 38 is 2/3 of 58 days, rounded down. A fixed number is easier to state. |

## Why this choice
Author choice, no study. The step 2 note (`analysis/priors/rates_recon_step2_2026-11-20.md`,
section 3.4) says the values were set before the run and were not tuned. The 38 dates are about two
thirds of the 58 days of the store. The 1.9 bp is the mean fit RMSE (range 1.46 to 2.50 bp). The
cost test is the rule of A-105 applied to the mean of the security. The 90% is a round number.
The values are set on the same 58 days that they score, so they are not an out-of-sample choice.

## Effect on the result
On the 58 stored days: the counts are in `docs/models/treasury-fit-rich-cheap.md` ("Lasting
residuals"). A change of a limit changes the counts. The effects of several limits do not add.

## Who decided and when
The author of the step 2 note (ticket `T-20261120-arb-2-rates-recon`, 2026-11-20). Moved to code in
ticket `T-20261121-recon-lasting` (2026-11-21).

## When to examine it again
When the store has more than one regime of data (for example 120 days or more), or when a lasting
residual is used for a decision.
