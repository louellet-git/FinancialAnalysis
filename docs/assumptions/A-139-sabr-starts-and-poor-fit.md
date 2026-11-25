# A-139: SABR starts and the poor-fit rule

Updated: 2026-11-21. Commit: not yet made.

## Choice
Six fixed starts of Levenberg-Marquardt for each series: rho in {-0.7, -0.3, 0} times nu in {0.3, 1.0}; alpha from the ATM mid volatility times F^(1-beta). The best final loss wins. Up to 100 iterations for each start.
A fit that converged is marked "converged, poor fit" when its largest error (vol points) is more than 5 times the median spread (vol points) of the series.

## Where used
| Model | File | Parameter name |
|---|---|---|
| SABR (`docs/models/sabr.md`) | `rust/src/volmodels/sabr.rs` | `fit_series` (starts), `POOR_FACTOR` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Random starts as in Heston (A-135) | The SABR fit has 3 parameters and takes microseconds. A fixed grid gives the same result in each run. |
| A rule on the RMS error | The largest error shows a bad strike range that the RMS hides. |

## Why this choice
Author choice, no study, for the starts and for the factor 5. The factor 5 has no source. It is a flag, not a test.

## Effect on the result
SPX, 2026-11-20, `--rate 0.04`: the rule marks 7 of 53 series (SPXW 3 to 10 days, SPX 91 days, SPXW 131 days).
No run with another factor or other starts is recorded. The starts have no effect on the 53 fits in this run: all converged with no parameter at a bound (no check with other starts).

## Who decided and when
Agent `volatility-derivatives-quant`, ticket review of 2026-11-21.

## When to examine it again
When a fit that is not marked has a visible error; when a fit with a good loss is marked; when a second date is fitted.
