# A-104: Start grid and bounds of tau in the curve fit

Updated: 2026-11-21. Commit: not yet made.

## Choice
- Start grid: tau1 in 0.4, 0.8, 1.5, 2.5, 4.0 years; tau2 in 3, 6, 10, 16, 25 years; a pair
  is skipped if tau2 is below 1.5 x tau1. Betas of each start come from least squares.
- The 3 starts with the lowest sum of squares go to Levenberg-Marquardt (80 iterations).
- tau is fitted as ln(tau) and is clamped: tau1 from exp(-2.5) to exp(3.7), about 0.08 to
  40.4 years; tau2 from exp(-1) to exp(3.7), about 0.37 to 40.4 years.
- The betas have no bound.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Treasury fit (`docs/models/treasury-fit-rich-cheap.md`) | `rust/src/fit.rs` | `clamp(-2.5, 3.7)`, `clamp(-1.0, 3.7)` |
| Start grid | `rust/src/fit.rs` (`fit_curve`) | `tau1`, `tau2`, `take(3)`, 80 |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Other grid or bounds | Not known. |

## Why this choice
Author choice, no study. The code has no comment for the numbers.

## Effect on the result
Not known. The code does not report if a tau reaches its clamp. A tau at a clamp would
change the curve shape without a message.

## Who decided and when
Not known.

## When to examine it again
When a fit has a large RMSE, or when a tau is at 0.08, 0.37 or 40.4 years.
