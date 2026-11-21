# A-114: Settings of the volatility estimator

Updated: 2026-11-21. Commit: not yet made.

## Choice
- Default estimator for `realvol`: close-to-close (all changes, equal weight).
- Rolling: the last N changes, with 5 <= N <= number of changes. EWMA: weight lambda^age,
  0 < lambda < 1 (the person gives lambda; there is no default).
- CKLS `realvol`: gamma is fixed at 0.5 by default (`--gamma`).
- A warning is given if the effective sample is less than 30 changes.
- Sigma is set from the residuals of the fitted mean divided by the conditional variance of the model (not from the raw changes).

## Where used
| Model | File | Parameter name |
|---|---|---|
| Weights (`docs/models/short-rate-models.md`) | `rust/src/ratemodels/calib.rs` | `weights`, `n_eff` |
| Warning | `rust/src/ratemodels/calib.rs` | `ne < 30.0` |
| Default gamma | `rust/src/ratemodels/cli.rs` | `unwrap_or(0.5)` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Fixed lambda (for example 0.94) | Not in the code. Not known. |

## Why this choice
Author choice, no study. Gamma 0.5 is the CIR value. The value 30 and the minimum N of 5 have no source in the code.

## Effect on the result
Not known as a number. `fa models frequencies` and `fa models rolling` show how the parameters move.

## Who decided and when
Not known. Ticket `T-20261119-stochastic-dcf-drivers` (2026-11-19).

## When to examine it again
When realized volatility is used as the volatility of a simulation.
