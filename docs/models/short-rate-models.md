# Model: Short-rate models (`fa models`)

Updated: 2026-11-21. Commit: not yet made.

## Purpose
WARNING: the parameters are fitted to the history of a proxy series, under the real-world measure.
The models have no market price of risk. Do not use a bond price of a model as a market price.

The module fits Vasicek, CIR, CKLS, Ho-Lee and Hull-White models to a rate history, and simulates paths
with a fixed seed. It gives parameters, their errors, and a comparison of the simulation with the closed forms.
For a person who needs a rate path or a study of mean reversion. Its types (`Cir`, `Vasicek`, `ZeroCurve`) are also used by
`rust/src/dcf/pathrate.rs`; the choices of that use are not in this document.

## Inputs
| Input | Source | Note |
|---|---|---|
| Rate history | GSW zero yield `SVENY01` (`gsw1y`, default) or `csv:PATH` | A proxy of the short rate (A-111). |
| Zero curve | GSW parameters of one day (`--curve-date`) | Ho-Lee and Hull-White only. |
| Sample | `--start`, `--end`, `--freq` | Rules in A-112. |

## Method (short)
1. Resample the history to daily, weekly or monthly; `dt` is the mean spacing in years (A-112).
2. Fit the AR(1) form by least squares and map it exactly to `kappa` and `theta` (`calib.rs` lines 4-7).
3. Methods: `ml` (exact density for Vasicek and CIR, quasi-likelihood for CKLS), `ols`, `gmm` (CKLS only), `realvol` (A-114).
4. Search bounds and refusal rules for CIR and CKLS: A-113.
5. Report t of mean reversion and flag it against 2.86 (A-110). Report the Feller ratio for CIR (below 1: r can reach 0).
6. Hull-White takes `a` and `sigma` from the Vasicek fit; theta(t) comes from the curve (A-115).
7. Simulate paths in parallel with one random stream for each path (the result does not depend on the thread count) (A-116).
8. Compare the simulated mean and bond price with the closed form (`mean_diff_in_se`, `bond_diff_in_se`).

## Outputs
JSON on the screen or a CSV or JSON table (`rolling`). `calibrate`: parameters with standard errors, log-likelihood, AIC,
t of mean reversion, warnings. `rolling`: parameters for each window. `frequencies`: the same interval at three samplings.
`compare`: Vasicek, CIR, CKLS, Ho-Lee on one interval with AIC (the AIC of CKLS and Ho-Lee is only close, because of the quasi-likelihood).
`simulate`: quantiles p05 to p95 at the horizons, the check against the closed form, the run time.

## Assumptions
| Record | Name | Value | One line |
|---|---|---|---|
| A-110 | Mean-reversion threshold | t 2.86 | The Dickey-Fuller 5% value. |
| A-111 | Short-rate proxy | GSW 1Y zero | Not the overnight rate. |
| A-112 | Sampling rules | 30 observations, `dt` by 365.25 | Last observation of a period. |
| A-113 | Optimizer bounds | kappa 1e-4 to 50, gamma -1 to 3 | Edge results are refused for CIR. |
| A-114 | Volatility estimator | close-to-close, warning below 30 | CKLS `realvol` gamma 0.5. |
| A-115 | Hull-White parameters | `a`, `sigma` from history | Curve gives theta(t). |
| A-116 | Simulation scheme | QE psi_c 1.5, 252 steps per year | Run defaults. |

## Limits
- The series is a fitted yield (GSW), not an observed overnight rate. The 1-year yield has a term premium and an expected path.
  Parameters are for that series. Do not use them as the policy rate.
- The Gaussian models (Vasicek, Ho-Lee, Hull-White) allow negative rates. CIR and CKLS need positive history (`require_positive`).
- Mean reversion is often not different from zero in a sample (see the warning). Then `kappa` and `theta` are weak and the long-run mean is not reliable.
- The sample is one series. A model with one factor does not give the curve shape of a day.
- Hull-White mixes a real-world `a` and `sigma` with a market curve (A-115). A price from this is not a market price.
- CKLS `gamma` below 0 cannot be simulated. `gmm` can give no root or many roots.
- No sensitivity number is stored in the records. Results of runs are in `docs/HIDDEN-ASSUMPTIONS.md` (about lines 587 to 625 and 1214 to 1217). I did not copy them.
- Network: the commands read the GSW file with `forwards::load`, which downloads when the cache is older than 12 hours. Check before you run.

## Code and command
| What | Path or command |
|---|---|
| Code | `rust/src/ratemodels/` (`models`, `calib`, `data`, `curve`, `sim`, `rolling`, `rng`, `math`, `cli`) |
| Model or parameter file | none (the parameters are fitted or given on the command line) |
| Command | `fa models calibrate\|rolling\|frequencies\|compare\|simulate` (`fa models` prints the usage) |
| Tests | `rust/src/ratemodels/tests.rs` (`cargo test --manifest-path rust/Cargo.toml --lib ratemodels`) |
| Architecture note | `docs/ARCHITECTURE.md`, section "`rust/src/ratemodels/`" |

## Review status
Not reviewed. Ticket `T-20261119-stochastic-dcf-drivers` made the module (see the board).
