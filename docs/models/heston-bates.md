# Model: Heston and Bates

Updated: 2026-11-21. Commit: not yet made.

## Purpose
WARNING: the fitted parameters are calibrated values of option prices. They are not measurements of the company.
Do not use them as an input of a DCF or of a forecast.

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
   (seed 7). Stage 1: LM runs 3 iterations from each of the 3 starts with the lowest loss. Stage 2: the best one goes on
   to 60 iterations. LM ends as "converged" (the last gain is 1e-10 of the loss or less, or the loss is under 1e-12),
   "iteration cap" (not converged) or "damping cap" (no step lowers the loss).
8. The reported error is the model implied volatility minus the market mid volatility.

## Outputs
Parameters, the status of the fit, a mark "at bound" for each parameter within 0.1% of its range from a bound, the losses of
the 3 refined starts after stage 1 (not final losses: only the best start goes on to the end; the status of the best start is final), the Feller ratio `2 kappa theta / xi^2` (under 1: `v` can reach 0),
the error by expiry in vol points (root mean square and largest), and the run time.

A bound hit means that the quotes do not identify the parameter: do not read the parameters. A fit that is not "converged"
is not a result. Bates on a thin chain is not unique, and only a comparison of runs with other seeds shows it (one run does not
check it; only the best start goes to the end). A new seed gives other parameters, and the final loss changes by about 11% (IBM, 40 quotes, final losses for seeds 1 to 5: 2.238, 2.141, 2.141, 2.141, 2.141; seed 7: 1.996; all "NOT converged: iteration cap", 2026-11-21, `--rate 0.04`). Heston and Bates are one-factor models and
the jump parameters of Bates are weak on a short or thin chain.

## Assumptions
Each choice has one record in `docs/assumptions/`.

| Record | Name | Value | One line |
|---|---|---|---|
| A-131 | Bounds | kappa 0.05 to 20, xi 0.05 to 3, rho -0.99 to 0; others in the record | A bound hit means the quotes do not identify the parameter. |
| A-132 | Spread filter | 1 vol point (5 for VIX) | From the ticket. `--max-spread` changes it. |
| A-133 | Spread floor | 0.05 vol points | Author choice, no study. |
| A-134 | Forward strikes | 5 nearest the parity strike | The chain check uses 3 (A-126). |
| A-135 | Starts and iterations | 20 starts, 3 refined, 3 iterations each, seed 7 | Refined starts set for speed. |
| A-136 | Time | Calendar days / 365 | The ticket says minutes; the code uses days. |

## Checks of the price
- Published values (Chan 2017, equation 105 and appendix B): T = 10 gives 22.318945791 (error 1.5e-10).
  T = 1 gives 5.785155434, and the published value is 5.785155450 (difference 1.6e-8).
  The converged integral does not move by more than 1e-11 with a finer grid. The characteristic
  function equals the solution of the Riccati equations (RK4) to 1e-12, and `xi = 1e-12` gives
  Black-76 to 1e-10. An independent Gil-Pelaez price of the validator gives 5.785155434376, the same as this code. So the
  published T = 1 value is 1.56e-8 high. The tests use 2e-8 against the published T = 1 value, 1e-10 against 5.785155434376,
  and 1e-8 for T = 10. The owner decides if this is accepted.
- Put-call parity holds by construction of the formula. The Gil-Pelaez put is a separate check (1e-9).

## Limits
- Limits of use: the fitted parameters are calibrated values, not measurements, and are not an input of a DCF or a forecast (help text, `rust/src/volmodels/cli.rs`); a fit that is "NOT converged" or has a parameter "at bound" is not a result (`calib.rs` `at_bound`; test `lm_reports_a_run_that_did_not_converge`).
- Limits of use: on Alpaca indicative quotes (`quote_kind: indicative`) a difference between the model and a quote is a measurement, not a tradable price (Alpaca documentation, https://docs.alpaca.markets/docs/historical-option-data, read 2026-11-21: the indicative feed is not OPRA quotes).
- American options (stocks, ETF): the model is European. The fit is an approximation.
- Backtest: none applies, because the fit calibrates a surface of one snapshot. It moves no value with history and it does not forecast.
- The snapshot has no clock time. Short expiries (1 to 2 days) have a large error in `t`.
- A model with one volatility factor fits one expiry well and the term structure badly.
- The filter of 1 vol point leaves few quotes on a stock chain (IBM, 2026-11-19: 40 of 1700).
- Alpaca closes have no bid and ask: do not fit them with this loss.
- Speed. The acceptance of 5 s is for Heston only. Machine: 4 cores, load average 1.4 to 2.9 (`uptime`) during the runs below,
  a machine that others also use. One expiry of 101 strikes takes 0.8 to 1.3 ms on one thread. Each refined start costs time:
  3 refined starts cost 3 warm-ups. SPX, 2026-11-20 (8141 quotes, 53 series, `--rate 0.04`):
  Heston with a warm-up of 5 iterations: 20 LM iterations, 3.9 s wall, 13.7 s CPU. With a warm-up of 3 iterations: 16 iterations,
  3.3 s wall, 12.2 s CPU, the same final loss 145232.630. Bates (3 iterations): 66 iterations, 18.1 s wall, 70 s CPU, loss 114998.
  The synthetic surface of 40 expiries and 11,000 quotes took 7.7 to 8.6 s for Heston under a load of 6 to 8 (before the warm-up
  change; not measured again).
- IBM, 2026-11-19: the 1 vol point filter keeps LEAPS only (4 series, 393 to 757 days). The short end is absent. The fit says
  nothing about it. The bias of the European model for early exercise is not sized.

## Code and command
| What | Path or command |
|---|---|
| Code | `rust/src/volmodels/` (`base`, `fourier`, `models`, `calib`, `chain`, `cli`, `cx`) |
| Command | `fa vol fit heston\|bates --symbol T --date D [--time HHMM] [--rate R] [--expiry-max DAYS] [--max-spread V] [--starts N] [--seed S]` |
| Tests | `rust/src/volmodels/tests.rs` (`cargo test --manifest-path rust/Cargo.toml --lib volmodels`) |

## Review status
Not reviewed.
