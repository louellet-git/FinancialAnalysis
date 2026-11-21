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
   charges `w` times the expected ratio of the cash for businesses to revenue, 0.5226
   x the past 5-year mean ratio capped at 25%, through the origin: no past purchases, no charge (`acq_persist`, A-002; it applies to
   the `acq_w` charge only, not to `acq_charge`, `acq_f` or the `chain` cut). The charge falls to zero with the fade of the growth when the company mean is above L.
   The spend has the second line of the business purchases when its value differs from the
   first line (`acq_second`, default on). `acq_persist=off` gives the past ratio as the base.
   The option `acq_keep` is off (the charge keeps the share L / mean, with no step at a mean
   equal to L).
5. Write a model file (`engine = "paths"`) and run the path engine (`rust/src/dcf/paths.rs`).
6. Terminal value: Gordon form with the terminal growth. Its mean is L (the same number
   as the fade level), its standard deviation is 0.75%.

## Outputs
The model numbers with their source (`--explain`), the P10, P50 and P90 of the value
for each share, the warnings, and with a price what the price implies. Exit code: 0 a
value, 3 no value, 2 an error. `fa dcf screen` gives one CSV row for each ticker.
`fa dcf implied` gives one CSV row for each ticker (what the price implies) and the group medians. It is a measurement: no input of the model reads it.

## Assumptions
| Record | Name | Value | One line |
|---|---|---|---|
| A-001 | One long-run growth L | min(10-year zero yield, 4.0%); terminal std 0.75% | Fade level and terminal mean. 4.0% = FOMC SEP 2026-09-16 (2.0% + 2.0%). Decided 2026-11-20 |
| A-002 | Acquisition charge | `w` x mean spend / revenue, 5 years | `w` = weight of the history in the growth mean; fades with the growth above L; second line of the purchases added when it differs (`acq_second`, proposed 2026-11-21). Decided 2026-11-20 |
| A-003 | Floor on the growth of the base year | 0% | A base year with growth below 0% enters the company mean and the first step of the growth as 0% (`last_floor`) |
| A-010 | Acquisition charge as a draw | off (`--rules acq_cv=1`) | The charge has the spend variation of the company (standard deviation of the own spend ratios). Measured 2026-11-21 on `fa dcf pit --cut 2015`: it fixes the thin low tail of the free cash flow band on half 1, but T1 still fails on revenue at T+3. Not adopted |
| not written | Equity risk premium | 4.41% (std 0.75%, lower bound 2%) | Damodaran implied value, 2026-11-19 |
| not written | Risk-free rate | 10-year zero yield of the GSW curve | Hull-White `a` 0.14, `sigma` 0.0075 |
| not written | Horizon | 10 years | Repo convention |
| not written | Beta | 2/3 regression beta + 1/3 | Weekly returns, 5 years |
| not written | SBC | A cost of the model | IBM review F5 |

The register of the code is `fa dcf assumptions` (`rust/src/dcf/assumptions.rs`): it lists each row with its default, its code place, its record and its state. The procedure of a study: skill `assumption-study`.
The records "not written" belong to the board ticket
`T-20261120-model-docs-backlog`.

## Limits
- Share basis. The price files of the store are split-adjusted up to the last date of each file. The share counts of the
  SEC facts are not. The model multiplies each share count by the product of the factors of the splits with
  `filing date of the fact < ex_date <= last date of the price file` (`rust/src/market/splits.rs`, `rust/src/dcf/share_basis.rs`, table
  `<cache>/_corporate/splits.csv`; one place: `YearData::from_period`). The filing date is used, not the period end,
  because a filing issued after a split shows the counts already adjusted for it (ASC 260). A spin-off and a class
  change are not splits. A row with `sec_check` other than `match` is not confirmed by the SEC facts. A missing
  table is a warning in `fa dcf auto`, `screen`, `implied` and `attribute`, and an error in `returns` and
  `fa portfolio backtest`. `fa dcf pit` has no factor: it scores revenue and free cash flow, not values per share.
  The classic engine (`classic.rs`, `scenarios.rs`, `verdict`, `report`, `serve`, `analyze`) has no factor: it takes
  its shares from a curated file of the ticker, and the curator must use the post-split basis.
- The model is a screen. Its P10 to P90 band covers 84% to 87% of real revenue in the
  point-in-time test, with known errors (`docs/HIDDEN-ASSUMPTIONS.md`, #199).
- After a year of decline the revenue P50 stays too low (T+2 and T+3), in a few base years with a common shock (2018 to 2021; A-003)
  (`docs/HIDDEN-ASSUMPTIONS.md`, #286). The bands for a stable margin are too wide; the rule `margin_sigma_nu` is
  measured and is not on by default (#286).
- The output gives the mean of the simulated value for each share, its standard error and the share of the sum that the
  largest 1% of the paths hold. The mean is a measurement of the distribution, not a better estimate than the P50 (the
  headline stays the P50). It is unstable when one tail holds a large share, and it is not tested against later returns.
  The screen CSV (last column `mean`), `fa dcf pit` and `fa dcf returns` leave a path with a value that is not finite out of the mean with no count; `fa dcf auto` prints the count.
- No bank, insurer, REIT or fund. No pension, lease or minority item in the net debt.
- The two defaults of 2026-11-20 (A-001, A-002) change every value. Data of 2026-11-19
  (data store commit 41116d0), `fa dcf screen` of `sp500.txt`, 4,000 paths, seed 7, 430
  tickers with a price; old = `--rules long_run=off,acq_w=0`. Median P50 / price 0.4895 to
  0.4745 (L alone 0.5605; the charge alone 0.4245; the two do not add). Stable set of 55:
  0.523 to 0.511. P50 at or below 0: 38 to 39. Median spread (P90 - P10) / P50 3.11 to 3.73.
  Median terminal value share 0.611 to 0.685. IBM `fa dcf auto` (10,000 paths): P10 13.09
  to -16.35, P50 $132.87 to $95.84 (IBM has no drift: the charge is constant).
- Point-in-time test, 6,030 company-years: revenue coverage P10 to P90 86.1 / 83.8 /
  86.9% at T+1 / T+2 / T+3 (unchanged). Free cash flow after acquisitions, coverage 76.3 /
  77.4 / 77.8% (old model, same definition: 77.3 / 78.0 / 78.4%). Files:
  `analysis/priors/auto_pit_new_2026-11-20.txt`, `auto_pit_old_afteracq_2026-11-20.txt`.
- The charge over-charges heavy buyers (spend above 5% of revenue): error of the free
  cash flow +1.8 / +1.8 / +2.4% of revenue at T+1 / T+2 / T+3 (old model -3.3 / -2.9 /
  -2.4%). A P50 at or below 0 of such a company (for example ILMN, WMB, whose growth has no
  drift, so the charge is constant) is an effect of the charge, not a value. A charge of
  exactly 0 for 34.1% of the tickers (the acquisition tag is missing).
- L is a typed constant. A new FOMC SEP changes it by a change of the code and of A-001.
- The median value is far below the market price for most tickers (median ratio 0.49 on
  2026-11-19 before the two defaults of 2026-11-20; 0.4745 after them). The investigation
  shows that no single defect causes this
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
