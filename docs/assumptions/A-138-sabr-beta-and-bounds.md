# A-138: SABR beta fixed at 1, and the bounds of alpha, rho, nu

Updated: 2026-11-21. Commit: not yet made.

## Choice
Beta is fixed, not fitted: default 1 (index), `--beta B` changes it (0 to 1).
The bounds of the fit, for a series with forward F: alpha from 1e-4 x F^(1-beta) to 10 x F^(1-beta); rho from -0.999 to 0.999; nu from 1e-4 to 10.

## Where used
| Model | File | Parameter name |
|---|---|---|
| SABR (`docs/models/sabr.md`) | `rust/src/volmodels/cli.rs` | `--beta`, default 1.0 |
| SABR (`docs/models/sabr.md`) | `rust/src/volmodels/sabr.rs` | `bounds` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Fit beta | Beta and rho are not separate in one smile. The ticket fixes beta. |
| Beta 0.5 | It stays available with `--beta 0.5`. The ticket names 1 or 0.5. |
| Unscaled alpha bounds | Alpha is about the volatility times F^(1-beta). For beta below 1 and F = 7000 a bound of 10 is too low (test `sabr_6_alpha_bounds_scale_with_the_forward`). |

## Why this choice
Beta fixed, default 1 for an index: the ticket (`PLANNING/board/T-20261120-vol-2-sabr-localvol.md`, "Model").
The bound values: author choice, no study. A parameter within 0.1% of its range from a bound is marked "at bound" (`calib.rs`, `at_bound`).
The scaling of alpha with F^(1-beta): from the form of the formula (Obloj 2008, arXiv:0708.0998, Table 1).

## Effect on the result
SPX, 2026-11-20, `--rate 0.04`, 28 days: beta 1 gives rms 0.382 vol points, beta 0.5 gives 0.316. No other comparison is recorded.
Rho and nu are comparable only for the same beta.

## Who decided and when
Beta: the ticket, 2026-11-20. Bounds: agent `volatility-derivatives-quant`, 2026-11-21.

## When to examine it again
When a fit has a parameter at a bound; when a chain has a forward or a volatility far from the SPX range.
