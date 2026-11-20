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
   by year 10 to L, the lower of the 10-year zero yield and 4.0% (A-001). A mean below L
   stays. L is also the mean of the terminal growth.
3. Margin: free cash flow before SBC and interest, shrunk the same way. SBC is a cost.
4. Discount rate: CAPM with the shrunk beta, the equity risk premium, and a Hull-White
   risk-free rate. Net debt and shares come from the latest balance sheet. The model
   charges `w` times the mean ratio of the cash for businesses to revenue (A-002).
5. Write a model file (`engine = "paths"`) and run the path engine (`rust/src/dcf/paths.rs`).
6. Terminal value: Gordon form with the terminal growth. Its mean is L (the same number
   as the fade level), its standard deviation is 0.75%.

## Outputs
The model numbers with their source (`--explain`), the P10, P50 and P90 of the value
for each share, the warnings, and with a price what the price implies. Exit code: 0 a
value, 3 no value, 2 an error. `fa dcf screen` gives one CSV row for each ticker.

## Assumptions
| Record | Name | Value | One line |
|---|---|---|---|
| A-001 | One long-run growth L | min(10-year zero yield, 4.0%); terminal std 0.75% | Fade level and terminal mean. 4.0% = FOMC SEP 2026-09-16 (2.0% + 2.0%). Decided 2026-11-20 |
| A-002 | Acquisition charge | `w` x mean spend / revenue, 5 years | `w` = weight of the history in the growth mean. Decided 2026-11-20 |
| not written | Equity risk premium | 4.41% (std 0.75%, lower bound 2%) | Damodaran implied value, 2026-11-19 |
| not written | Risk-free rate | 10-year zero yield of the GSW curve | Hull-White `a` 0.14, `sigma` 0.0075 |
| not written | Horizon | 10 years | Repo convention |
| not written | Beta | 2/3 regression beta + 1/3 | Weekly returns, 5 years |
| not written | SBC | A cost of the model | IBM review F5 |

The records "not written" belong to the board ticket
`T-20261120-model-docs-backlog`.

## Limits
- The model is a screen. Its P10 to P90 band covers 84% to 87% of real revenue in the
  point-in-time test, with known errors (`docs/HIDDEN-ASSUMPTIONS.md`, #199).
- No bank, insurer, REIT or fund. No pension, lease or minority item in the net debt.
- The two defaults of 2026-11-20 (A-001, A-002) change every value. Run on the data of
  2026-11-19 (4,000 paths, seed 7): median ratio 0.4895 to 0.427 (430 tickers; stable
  set 0.523 to 0.504); tickers with P50 at or below 0: 38 to 50. Old `auto_pit_*.txt`
  files are from the model before them (`--rules long_run=off,acq_w=0`).
- Point-in-time test, 6,030 company-years: revenue coverage P10 to P90 86.1 / 83.8 /
  86.9% at T+1 / T+2 / T+3 (the same as before). Free cash flow after acquisitions:
  coverage 76.3 / 77.1 / 77.4% (old model, same definition: 77.3 / 78.0 / 78.4%).
  Files: `analysis/priors/auto_pit_new_2026-11-20.txt`, `auto_pit_old_afteracq_2026-11-20.txt`.
- The acquisition charge over-charges heavy buyers (spend above 5% of revenue), and the
  5-year mean can hold one large deal. A company with no reported spend has no charge.
- L is a typed constant. A new FOMC SEP changes it by a change of the code and of A-001.
- The median value is far below the market price for most tickers (median ratio 0.49 on
  2026-11-19 before the two defaults of 2026-11-20; 0.427 after them). The investigation shows that no single defect causes this
  (`analysis/priors/dcf_investigation_2026-11-20.md`). Do not read the ratio as a signal
  without the review.

## Code and command
| What | Path or command |
|---|---|
| Builder and rules | `rust/src/dcf/auto.rs` (`Rules`, `build`) |
| Commands | `rust/src/dcf/auto_cli.rs`; `fa dcf auto IBM --cache data/raw --explain` |
| Point-in-time test | `rust/src/dcf/auto_pit.rs`; `fa dcf pit --cache data/raw` |
| Return test | `rust/src/dcf/auto_returns.rs`; `fa dcf returns --cache data/raw` |
| Tests | `rust/src/dcf/auto_tests.rs` |
| Alternatives | `fa dcf auto --help` (`--rules K=V`) |

## Review status
Not reviewed as a valuation. Tested by the point-in-time test (2026-11-19, 2026-11-20)
and investigated on 2026-11-20 (four defects corrected). No skeptic, reviewer or auditor
looked at it.
