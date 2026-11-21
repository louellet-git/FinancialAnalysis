# A-007: beta of the automatic model (weight and data rules)

Updated: 2026-11-21. Commit: 90ed737 (base of the branch).

## Choice
Beta = 0.667 x raw regression beta + 0.333 x 1.0 (`beta_weight` = 2/3, the rest is 1.0). The raw beta is
`cov / var` of the weekly returns of the stock against SPY. Rules of the data:
- At most 260 weekly returns (5 years), up to the model date.
- At least 104 weekly returns (2 years). With fewer, the model uses beta 1.0. The model prints a warning only when it has no price history at all.
- A weekly return needs two weeks in sequence in both series. The week takes the last close on or before the date.
- The SPY series is the price cache of SPY if it has at least 520 rows. Else the daily file `trading/SPY_1d.csv` of the cache.
- With no stock price file, beta is 1.0 and the equity weight is 1.0, with warnings.
The beta is a constant for each path (not drawn).

## Where used
| Model | File | Parameter name |
|---|---|---|
| Automatic path model | `rust/src/dcf/auto.rs` | `Rules::beta_weight`; `regression_beta` (104, 260); `index_from_cache` (520) |
| Return test (the same rule: 260 weeks, same weight) | `rust/src/dcf/auto_returns.rs` | `regression_beta(..., 260, beta_weight)` |
| IBM driver model (not the same rule) | `analysis/IBM/driver_model.toml` | driver `beta` = 0.85, fixed |

## Alternatives considered
| Alternative | Command | Why not chosen |
|---|---|---|
| Raw beta | `--rules beta_weight=1` | Not known. Measured only |
| Beta 1.0 for all | `--rules beta_weight=0` | Not known. Measured only |
| IBM: 0.85 deterministic | (IBM file) | A closed IBM review decision, on epistemic grounds (`analysis/IBM/DRIVER-REVIEW.md`, row `beta`) |

## Why this choice
The weight 2/3 and the numbers 104, 260 and 520 have no source in any file that was read.
The code and `docs/HIDDEN-ASSUMPTIONS.md` #196 state them as the rule. No document of this repo names a reason for the shrink to 1.0. The standard error of a
5-year weekly beta is about 0.1 (#196). The IBM file calls the beta "an estimation constant,
not a random process" (`DRIVER-REVIEW.md`, IBM only).

## Effect on the result
438 tickers, 2026-11-19, median value / price (`analysis/priors/dcf_investigation_2026-11-20.md`, section 2):
`beta_weight=1`: +0.011 (stable set +0.057). `beta_weight=0`: -0.037 (stable set -0.118).
The rules 104 and 520: no sensitivity number is known.

## Who decided and when
Not known. The weight is in the first commit of the automatic model (176aa8e, 2026-11-19).

## When to examine it again
When a sector beta or an own-index beta is available. When a person wants the effect of the rows 104 and 520 measured.
