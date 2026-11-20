# Model: Heston and Bates

Updated: 2026-11-20. Commit: not yet made.

## Purpose
The model prices European options from a characteristic function and fits a smile
surface to the bid and ask of one stored option snapshot. It is the baseline for the later
steps of `analysis/priors/volatility_study_2026-11-20.md`. A model price is not a market
price. The fit gives parameters, and the error of each expiry in volatility points.

## Inputs
| Input | Source | Note |
|---|---|---|
| Quotes | `data/raw/<T>/options/<D>.json` (`OptionChainSnapshot`) | Real bid and ask only. Closes give no spread. |
| Forward of a series | Put-call parity of the mids (`forward_from_parity_5` in `chain.rs`) | Median of the 5 strikes nearest to the parity strike |
| Rate | `--rate R`, else the Treasury par curve of the date | It sets the discount factor only |
| Time | Calendar days from the snapshot date, divided by 365 | The snapshot has no clock time |

## Method (short)
1. A series is one root and one expiry (`occ_root`). The forward comes from put-call parity.
2. Keep out-of-the-money quotes with bid above 0, ask above bid, and a spread of at most
   1 vol point (5 for VIX). Each dropped quote has one reason, and the command prints the counts.
3. The model gives the characteristic function of `ln(F_T / F_0)` under the forward measure.
   Heston uses the stable form of Albrecher, Mayer, Schoutens and Tistaert (2007), rewritten
   with no difference of near-equal numbers (so `xi = 0` works). Bates multiplies it by the jump part.
4. The price is the Lewis integral with adaptive Gauss-Legendre (16 nodes per panel).
   All strikes of one expiry share the values of the characteristic function.
   The fit uses a panel tolerance of 1e-9 and a tail tolerance of 1e-12 (`Accuracy::FIT`). Prices, the reported errors and the tests use 1e-13 and 1e-16 (`Accuracy::TIGHT`).
   `C / df = F - sqrt(F K) J` and `P / df = K - sqrt(F K) J`.
5. The Carr-Madan FFT is an independent check, used in the tests only.
6. The loss is the sum of squares of `(model price - mid) / (vega * spread)`. Vega is the
   Black-76 vega at the mid volatility. The spread has a floor of 0.05 vol points.
7. Levenberg-Marquardt with a numerical Jacobian, bounds by clamping, 20 random starts
   (seed 7). LM refines the start with the lowest loss.
8. The reported error is the model implied volatility minus the market mid volatility.

## Outputs
Parameters, the Feller ratio `2 kappa theta / xi^2` (under 1: `v` can reach 0), the error
by expiry in vol points (root mean square and largest), and the run time.

## Assumptions
No record in `docs/assumptions/` has a form for these choices. They are listed here.

| Name | Value | One line |
|---|---|---|
| Bounds | kappa 0.05 to 20, xi 0.05 to 3, rho -0.99 to 0 | From the ticket. `rho` is not above 0 (fits stocks and SPX, not VIX). |
| Bounds, other | v0 and theta 1e-4 to 4, lambda 0 to 10, mu_j -1 to 0.5, delta_j 0.01 to 1 | Set by the author. |
| Spread filter | 1 vol point (5 for VIX) | From the ticket. `--max-spread` changes it. |
| Spread floor | 0.05 vol points | Stops one tight quote from taking all the weight. |
| Forward | 5 strikes nearest to the parity strike | From the ticket. Own code in `chain.rs`: the shared helper `forward_from_parity_blind` uses 3. |
| Refined starts | 1 of 20 | The ticket does not say. Set for speed. |
| Time | Calendar days / 365 | No clock time in the snapshot. |

## Checks of the price
- Published values (Chan 2017, equation 105 and appendix B): T = 10 gives 22.318945791 (error 1.5e-10).
  T = 1 gives 5.785155434, and the published value is 5.785155450 (difference 1.6e-8).
  The converged integral does not move by more than 1e-11 with a finer grid. The characteristic
  function equals the solution of the Riccati equations (RK4) to 1e-12, and `xi = 1e-12` gives
  Black-76 to 1e-10. So the published T = 1 value is likely less exact than its digits. The test
  uses 2e-8 for T = 1 and 1e-8 for T = 10. The owner decides if this is accepted.
- Put-call parity holds by construction of the formula. The Gil-Pelaez put is a separate check (1e-9).

## Limits
- American options (stocks, ETF): the model is European. The fit is an approximation.
- The snapshot has no clock time. Short expiries (1 to 2 days) have a large error in `t`.
- A model with one volatility factor fits one expiry well and the term structure badly.
- The filter of 1 vol point leaves few quotes on a stock chain (IBM, 2026-11-19: 40 of 1700).
- Alpaca closes have no bid and ask: do not fit them with this loss.
- Speed (4 cores, synthetic surface of 40 expiries and 11,000 quotes): the Heston fit takes 2.6 s and the Bates fit takes 3.7 s. One expiry of 101 strikes takes 0.8 to 1.2 ms on one thread.

## Code and command
| What | Path or command |
|---|---|
| Code | `rust/src/volmodels/` (`base`, `fourier`, `models`, `calib`, `chain`, `cli`, `cx`) |
| Command | `fa vol fit heston\|bates --symbol T --date D [--rate R] [--expiry-max DAYS] [--max-spread V]` |
| Tests | `rust/src/volmodels/tests.rs` (`cargo test --manifest-path rust/Cargo.toml --lib volmodels`) |

## Review status
Not reviewed.
