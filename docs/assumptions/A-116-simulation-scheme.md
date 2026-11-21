# A-116: Simulation scheme and run defaults

Updated: 2026-11-21. Commit: not yet made.

## Choice
- CIR: the Andersen quadratic-exponential (QE) scheme, switch psi_c = 1.5. Each step has the exact
  mean and variance. r stays at 0 or above.
- CKLS with gamma above 0 (gamma 0.5 too): Milstein with full truncation. Gamma 0 uses the exact Gaussian step. For gamma 0.5 with the QE scheme use the model `Cir`.
- Vasicek, Hull-White, Ho-Lee: exact Gaussian step.
- `fa models simulate` defaults: 10 years, 252 steps per year, 100000 paths, seed 1.
- The discount factor is `exp(-integral of r)`, with the trapezoid rule (error of order dt^2).

## Where used
| Model | File | Parameter name |
|---|---|---|
| CIR (`docs/models/short-rate-models.md`) | `rust/src/ratemodels/models.rs` | `QE_PSI_C` |
| CKLS | `rust/src/ratemodels/models.rs` | `rp = r.max(0.0)` |
| Defaults | `rust/src/ratemodels/cli.rs` | `years`, `steps_per_year`, `paths`, `seed` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Euler with truncation for CIR | Not known. The QE scheme keeps the exact mean and variance (comment `models.rs` ). |

## Why this choice
Andersen (2008) gives the QE scheme and the value 1.5 as the usual switch. The source is cited in the
code comment. The run defaults are an author choice.

## Effect on the result
The output compares the simulated mean and bond price with the closed form, in standard errors
(`mean_diff_in_se`, `bond_diff_in_se`). Not known as a sensitivity to psi_c or to the step.

## Who decided and when
Not known. Ticket `T-20261119-stochastic-dcf-drivers` (2026-11-19).

## When to examine it again
When the step is coarse (monthly) or the rates are near 0 and the Feller ratio is below 1.
