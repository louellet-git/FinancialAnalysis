# Model: automatic path model

Updated: 2026-11-20.

## Purpose
`fa dcf auto TICKER` makes a Monte Carlo DCF model for one ticker with no hand input, and
runs it. `fa dcf screen` does this for many tickers. It is a screen. It is not a reviewed
valuation. Do not use it alone for a trade (`CLAUDE.md`, "Review of a valuation report").

## Inputs
| Input | Source | Note |
|---|---|---|
| Company facts | The standard variables from `mapcheck` (cache `data/raw`) | Models read standard names only |
| Parameter table | `analysis/priors/process_priors_2026-11-19.toml` | AR(1) cells by size bucket and fade cells |
| Yield curve | GSW file in the cache (`fa forwards`) | Else a fixed 4.2% with a warning |
| Price and beta | `data/raw/<T>/prices.json`, `SPY_1d.csv` | Else beta 1.0 and equity weight 1.0, with warnings |
| Model date | `--as-of` | Default: today |

## Method (short)
1. Map the company facts. Refuse a ticker with mapping status FAIL, fewer than 3
   fiscal years, or a template other than `standard` (bank, insurer, REIT, fund, foreign).
2. Growth: an AR(1) around a company mean that is shrunk to the cell mean. The mean fades
   to the 10-year zero yield by year 10.
3. Margin: free cash flow before SBC and interest, shrunk the same way. SBC is a cost.
4. Discount rate: CAPM with the shrunk beta, the equity risk premium, and a Hull-White
   risk-free rate. Net debt and shares come from the latest balance sheet.
5. Write a model file (`engine = "paths"`) and run the path engine (`rust/src/dcf/paths.rs`).
6. Terminal value: Gordon form with the terminal growth.

## Outputs
The model numbers with their source (`--explain`), the P10, P50 and P90 of the value
for each share, the warnings, and with a price what the price implies. Exit code: 0 a
value, 3 no value, 2 an error. `fa dcf screen` gives one CSV row for each ticker.
`fa dcf implied` gives one CSV row for each ticker (what the price implies) and the group medians. It is a measurement: no input of the model reads it.

## Assumptions
| Record | Name | Value | One line |
|---|---|---|---|
| A-001 | Terminal growth | 2.25% (std 0.75%) | Repo convention from the IBM driver review |
| not written | Equity risk premium | 4.41% (std 0.75%, lower bound 2%) | Damodaran implied value, 2026-11-19 |
| not written | Risk-free rate | 10-year zero yield of the GSW curve | Hull-White `a` 0.14, `sigma` 0.0075 |
| not written | Horizon | 10 years | Repo convention |
| not written | Beta | 2/3 regression beta + 1/3 | Weekly returns, 5 years |
| not written | SBC | A cost of the model | IBM review F5 |

The records "not written" belong to the board ticket
`T-20261120-model-docs-backlog`.

## Limits
- The model is a screen. Its P10 to P90 band covers about 80% of real revenue in the
  point-in-time test, with known errors (`docs/HIDDEN-ASSUMPTIONS.md`, #199).
- No bank, insurer, REIT or fund. No pension, lease or minority item in the net debt.
- The median value is far below the market price for most tickers (median ratio 0.49 on
  2026-11-19). The investigation shows that no single defect causes this
  (`analysis/priors/dcf_investigation_2026-11-20.md`). Do not read the ratio as a signal
  without the review.

## Code and command
| What | Path or command |
|---|---|
| Builder and rules | `rust/src/dcf/auto.rs` (`Rules`, `build`) |
| Commands | `rust/src/dcf/auto_cli.rs`; `fa dcf auto IBM --cache data/raw --explain` |
| Point-in-time test | `rust/src/dcf/auto_pit.rs`; `fa dcf pit --cache data/raw` |
| Return test | `rust/src/dcf/auto_returns.rs`; `fa dcf returns --cache data/raw` |
| Implied by the price | `rust/src/dcf/auto_implied.rs`; `fa dcf implied --cache data/raw` (a measurement) |
| Tests | `rust/src/dcf/auto_tests.rs`, `rust/src/dcf/auto_implied_tests.rs` |
| Alternatives | `fa dcf auto --help` (`--rules K=V`) |

## Review status
Not reviewed as a valuation. Tested by the point-in-time test (2026-11-19, 2026-11-20)
and investigated on 2026-11-20 (four defects corrected). No skeptic, reviewer or auditor
looked at it.
