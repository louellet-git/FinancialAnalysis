# A-115: Hull-White and Ho-Lee parameters from history and from the curve

Updated: 2026-11-21. Commit: not yet made.

## Choice
- Hull-White: `a` and `sigma` come from the Vasicek fit to the history (the `a` is the kappa
  of that fit). The function theta(t) is not calibrated to history: the simulation fits it
  to the GSW zero curve of one day (`--curve-date`, default today).
- Ho-Lee: `sigma` is the realized volatility of the changes. The drift is the curve.
- The start rate `r0` is the instantaneous forward rate at 0 of that curve (b0 + b1), unless the
  person gives `--r0`.
- For a curve before 1980 the code sets b3 = 0.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Calibration | `rust/src/ratemodels/calib.rs` lines 446-480, 817-831 | `holee`, `HullWhite` branch |
| Curve | `rust/src/ratemodels/curve.rs` lines 38-52 | `ZeroCurve::from_forwards` |
| Start | `rust/src/ratemodels/models.rs` (Hull-White `default_r0`) | `curve.forward(0.0)` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| `a` and `sigma` from option prices (caps, swaptions) | The repo has no such data. |

## Why this choice
The code gives the warning "a and sigma come from the history; theta(t) is not calibrated to history".
The mix is a modelling choice: history parameters (real-world) are used with a curve that
is a market curve. Author choice, no study.

## Effect on the result
The curve fixes the mean of r(t) and the bond price P(0,t) (the model reprices the curve; the
GSW parameters match the published zero yields to 5e-7, `docs/ARCHITECTURE.md`, section
`ratemodels`). The history fixes the spread of the paths. No sensitivity number is known.

## Who decided and when
Not known. Ticket `T-20261119-stochastic-dcf-drivers` (2026-11-19).

## When to examine it again
When a path model with the curve is used for a value, and when option data for `a` exist.
