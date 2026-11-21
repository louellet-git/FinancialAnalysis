# A-113: Bounds and limits of the maximum likelihood search

Updated: 2026-11-21. Commit: not yet made.

## Choice
- CIR maximum likelihood (Nelder-Mead on logs): kappa 1e-4 to 50 per year, sigma 5e-3 to 5.
  The result is refused if kappa is above 49.5, sigma is below 5.05e-3 or above 4.95, or kappa
  is below 1.5e-4 (no mean reversion).
- CKLS quasi-likelihood: kappa 1e-4 to 50, gamma -1 to 3. A result with kappa below 1.5e-4 is refused.
- CKLS moment fit (GMM): gamma searched on [-1, 3] in steps of 0.02. With more than one root the
  root nearest 0.5 is taken, with a warning. With no root the fit gives an error.
- Nelder-Mead: tolerance 1e-9, 1500 iterations (CIR) and 2000 (CKLS). If it does not meet the tolerance, a warning is given.
- A fitted CKLS with gamma below 0 gives a warning: the library simulates gamma in [0, 3] only.

## Where used
| Model | File | Parameter name |
|---|---|---|
| CIR (`docs/models/short-rate-models.md`) | `rust/src/ratemodels/calib.rs` | bounds in `nll`, edge checks |
| CKLS ML | `rust/src/ratemodels/calib.rs` | `x[2] < -1.0 || x[2] > 3.0` |
| CKLS GMM | `rust/src/ratemodels/calib.rs` | `(lo, hi, step) = (-1.0, 3.0, 0.02)` |
| Simulation domain | `rust/src/ratemodels/models.rs` | gamma in [0, 3] |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Other bounds | Not known. |

## Why this choice
Author choice, no study. The code has no source for the numbers. The errors say that a result
at the edge means that the data do not fit the model.

## Effect on the result
Not known. A fit near an edge is refused for CIR and for the kappa edge of CKLS. A CKLS gamma
at -1 or 3 gives no refusal in the code (not known if it can happen).

## Who decided and when
Not known. Ticket `T-20261119-stochastic-dcf-drivers` (2026-11-19).

## When to examine it again
When a real series gives a refused fit, or when gamma is at a bound.
