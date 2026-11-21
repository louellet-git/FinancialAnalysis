# A-131: parameter bounds of Heston and Bates

Updated: 2026-11-21. Commit: not yet made.

## Choice
| Parameter | Lower | Upper | Model |
|---|---|---|---|
| v0 (start variance) | 1e-4 | 4 | Heston, Bates |
| theta (long variance) | 1e-4 | 4 | Heston, Bates |
| kappa (speed) | 0.05 | 20 | Heston, Bates |
| xi (vol of variance) | 0.05 | 3 | Heston, Bates |
| rho | -0.99 | 0 | Heston, Bates |
| lambda (jumps per year) | 0 | 10 | Bates |
| mu_j (mean log jump) | -1 | 0.5 | Bates |
| delta_j (jump size sd) | 0.01 | 1 | Bates |
The fit clamps each step into the bounds. A parameter within 0.1% of its range from a bound is marked "at bound" in the output.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Heston | `rust/src/volmodels/models.rs:49-51` | `Heston::bounds` |
| Bates | `rust/src/volmodels/models.rs:89-93` | `Bates::bounds` |
| Mark "at bound" | `rust/src/volmodels/calib.rs:66-68` | `at_bound` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| rho up to +1 | Not known. The bound 0 fits stocks and SPX, not VIX (`heston-bates.md`). |
| Wider kappa and xi | Not known. |

## Why this choice
kappa, xi and rho: the ticket `T-20261120-vol-1-heston-bates` states "kappa in 0.05 to 20, xi in 0.05 to 3, rho in -0.99 to 0".
The other bounds (v0, theta, lambda, mu_j, delta_j): author choice, no study. The ticket gives no source for the numbers.

## Effect on the result
A parameter at a bound means that the quotes do not identify it: the parameters are not to be read (`docs/models/heston-bates.md`, Outputs).
No run of the same chain with other bounds is recorded.

## Who decided and when
kappa, xi, rho: owner ticket of 2026-11-20 (vol-1). Others: agent `volatility-derivatives-quant`, date not known.

## When to examine it again
When a fit reports "at bound" on a good chain; when the fit is used for VIX options (rho above 0).
