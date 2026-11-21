# A-005: risk-free rate of the automatic model

Updated: 2026-11-21. Commit: 90ed737 (base of the branch).

## Choice
The risk-free rate is the GSW 10-year zero yield (`rf` default: GSW 10-year zero yield) of the model date.
The model makes it a Hull-White process on the GSW curve of that date: mean reversion `a` = 0.14,
volatility `sigma` = 0.0075, tenor 10 years, mode `long_flat`. With no curve, the model uses a fixed 4.2%
(`DEFAULT_RF`) and prints a warning. The curve is the last row on or before the model date.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Automatic path model | `rust/src/dcf/auto.rs` | `Rules::rf_fixed` (None), `hw_a` 0.14, `hw_sigma` 0.0075, `DEFAULT_RF` |
| Model file that the builder writes | `[rates]` of the TOML text | `model = "hullwhite"`, `mode = "long_flat"` |
| Path engine | `rust/src/dcf/pathrate.rs` | the Hull-White path |
| Curve file | `data/raw/rates/gsw_feds200628.csv` | Federal Reserve Board, GSW |
| ANT first pass | `analysis/ANT/driver_model_paths.toml` | `[rates]` (same `a`, `sigma`) |

## Alternatives considered
| Alternative | Command | Why not chosen |
|---|---|---|
| Fixed 4.2% | `--rules rf=0.042` | Not known. Used as a test of the effect |
| Terminal mean equals the yield | `--rules terminal_rf=1` | See A-001 |

## Why this choice
Source of `a` and `sigma`: `docs/HIDDEN-ASSUMPTIONS.md`, #160. They come from `fa models calibrate
--model vasicek --method ml` on the GSW 1-year zero yield, monthly, 2000 to 2026-11. The mean reversion
is not significant (t = 1.78 against 2.86): `a` is weakly known (#81). The 1-year yield is a proxy of
the short rate. The model gives no term-premium drift (#160). Mode `long_flat` keeps the expected
rate at the level of time zero (`rust/src/dcf/paths_tests.rs`). Why the 10-year yield and not another
tenor: not known (the code names it as the convention). The yield enters the discount of the whole cash flow.

## Effect on the result
`--rules rf=0.042`, 438 tickers, 2026-11-19: median value / price +0.097 (stable set +0.102), median
P50 +17% (`analysis/priors/dcf_investigation_2026-11-20.md`, section 2). The yield was 5.13% on
2026-11-13. The GSW curve has large `beta2`, `beta3` terms (a fit with near-collinear terms): `docs/HIDDEN-ASSUMPTIONS.md` (rates).

## Who decided and when
Not known for the choice of the 10-year zero yield. `a` and `sigma`: set on 2026-11-19 (commits 4ef60f9,
path engine, and 176aa8e, automatic model) from the calibration above. No review.

## When to examine it again
When the par curve work on its own branch is merged. When a person wants a different tenor.
When `a` is calibrated again on a longer or a daily series.
