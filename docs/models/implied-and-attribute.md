# Model: what the price implies, and where the price move went (`fa dcf implied`, `fa dcf attribute`)

Updated: 2026-11-21. Commit: 2e70201 (branch `claude/premium-series`). These two commands share one document: they share
the value at the means, the searches and the code in `auto_implied.rs`.

Backtest: none applies, because these commands measure what a price implies and split a past change; they do not forecast. Their point-in-time checks apply (facts filed on or before each date, the stale-price check, the truncation test).

## Purpose
`fa dcf implied` answers: what cost of capital, growth or margin does the market price imply, for each ticker
and for groups? `fa dcf attribute` answers: between two dates, how much of the log price change is facts, time,
rates and the implied premium (or growth)? Both are measurements. No input of the automatic model reads them
(`docs/QUALITY-STANDARDS.md`, section 6). The split is accounting, not a cause.

## Inputs
| Input | Source | Note |
|---|---|---|
| The automatic model | `rust/src/dcf/auto.rs` and the table (`analysis/priors/process_priors_2026-11-19.toml`) | The builder runs again for each search step |
| Price | `--prices FILE`, or the last close; with `--price-date`, the last close on or before it | At most 10 days older (A-012) |
| Facts | With `--as-of` or `--from/--to`: facts filed on or before the date | A 10-K of the cache does not fill a missing input (see the rule below) |
| Curve, beta | `--curve auto` (default, being built; A-027), `gsw`, `fedinvest` or `par`; SPY of the date | One source at both dates. A GSW row older than 3 days is flagged (A-012). See "Curve" below |
| Groups | SIC code of the cache (today's code), or `--groups FILE` | Fama-French 12 sector; industry with 5 tickers or more |

## Method (short)
1. Value at the means: one path, each parameter at its mean, each shock zero (`PathEngine::deterministic`). No seed.
2. `implied`, three bisections, one number each, all others fixed: the premium mean (-4% to +20%), the growth
   shift (-30 to +60 points), the margin shift (-50 to +50 points). No solution: the bound and a flag.
3. List premium: the premium for which the median of value / price of the list is 1. Then each group gets the
   median of its value at that premium over its price.
4. `attribute`: the price of each date is the value of the model of that date at the premium that the price
   implies (x1 at D1, x2 at D2).
   - The change `ln(P2/P1)` moves in steps: facts, time (stub), rates, then the plug and the search residual.
   - The plug is the premium, with the equity weight and the beta of D2.
   - Both orders of rates and plug are printed. Their range is the order effect.
5. Two chains: the premium view and the growth view (where the premium view has no value). A third line: the
   growth shift that P2 needs at the premium of D1.
6. A list: the whole-list change weighted by price x shares at D1. The share count is guarded (A-016).

## Curve (`--curve`)
- `gsw`: the GSW row of the Federal Reserve Board. It lags about one week.
- `fedinvest`: the NSS curve that `fa recon rich-cheap` fits to the FedInvest prices of the last file on or before the date.
- `par`: the same NSS fit to the Treasury par yield curve of the last row on or before the date. The tenors are
  1 to 30 years (`PAR_TENORS`); the fit does not go beyond 30 years. Each tenor is a par bond.
- Rule of the code (`par_fit_rmse`, `load_curve_par`): a par fit with a root-mean-square error
  above `PAR_FIT_MAX_RMSE_BP` = 5.0 bp is not used (A-028).

## The full bridge
- Each ticker with a weight is in it once. A second class of one company has no weight (one weight for one company).
- The lines add to the change of the list: facts, time, rates, implied premium, implied growth, residual, and
  "no model value" (the price change of a ticker with no view).
- Premium first: a ticker uses the premium view where it exists, else the growth view. Growth first: the reverse.
  The two readings are two readings of one price change. The model cannot tell them apart.
- A list of 20 tickers or fewer (`BRIDGE_TICKER_LINES`) gets one line for each ticker (A-032).
- Stale price: a close older than the SPY close of the same date leaves every sum (A-031).
  `fa dcf attribute` prints the list of the tickers left out. `fa dcf premium-series` does not print it.
- One group line: the tickers with an SEC 8-K item 2.02 in the window.

## Split of the plug: common part and company part
The plug of a view splits in two engine steps (the two add exactly to the plug). Common step: the premium (or the
growth shift) goes from x1 to x1 + c. Company step: from there to x2, with the beta, weight and facts of D2.
The common step uses the facts, the date and the curve of D2, and the beta and the equity weight of D1. The company part
is the rest of the plug: it holds the change of the stock's own number, and the change of the beta and of the weight from
D1 to D2. With the default `median`, c is the median of the same price changes that the split explains. Thus the company
part is a difference from the list. It is not news about the company.
Option `--common` sets c (A-029):
- `median` (default): the median of x2 - x1 over the tickers of the run. `--common-list FILE` uses the tickers of
  that file in place of the run (needed for a run of one ticker; with no list c = 0).
- `vw`: the mean weighted by price x shares at D1.
- `file` with `--common-file FILE`: the economics view. The file has the header `date,erp_change,growth_change,source`
  (decimals; the row of D2). In both views the common step moves the premium and G of A-001 (`Rules.long_run`)
  together. G moves the value only where G binds, because L = min(10-year zero, G). An empty field means "not
  covered": no information, not a change of zero. With both fields empty, the company part is the whole plug and
  the text gives the count of covered and not covered tickers.
- The economics view is an input from the Head of Economics (its own engine). It never comes from the prices that
  the attribution explains. If it did, the split would be circular (`T-20261121-premium-company-part`, design change 2).
- A beta is inside the value (cost of equity = rf + beta x premium). So no separate beta line exists.

## Extra CSV columns
`beta1`, `beta2` (the beta of the model at D1 and D2; with no regression beta the model uses 1.0, the cell has 1.0000
and the flags have `beta1-default` or `beta2-default`); `dur_premium1` = -(ln V(x1 + h) - ln V(x1 - h)) / (2 h),
h = 0.001 (A-030); `earnings_8k` (the dates of the 8-K item 2.02 in the window); then `c_premium`, `c_growth`,
`p_premium_common_pct`, `p_premium_company_pct`, `g_growth_common_pct`, `g_growth_company_pct`.

## Outputs
`implied`: one CSV row for each ticker (value over price, implied rate, premium, growth and margin shift, flags),
the group medians, the list premium. `attribute`: one table or one CSV row for each ticker (parts in percent),
medians by size and sector, the list change, top contributors, flagged tickers. No random part; the same input
gives the same output for any `--jobs`.

## Assumptions
| Record | Name | Value | One line |
|---|---|---|---|
| A-015 | Search ranges and list premium | -4% to 20%; -30 to +60; -50 to +50 points | Design of 2026-11-20; ranges have no other source |
| A-016 | Guard of the share count | 0.5x-2x note; 0.01x-100x replace | Used for the weight of a list |
| A-012 | Age of a price or a curve | 10 days; 3 days | Conventions, no source |
| A-022 | Equity risk premium | 4.41% | The model premium that the search moves |
| A-017 | Share basis | filing date rule | Past price against filed shares |
| A-027 | Curve rule | GSW at most 3 days older, else par | Used by `premium-series` (see `premium-series.md`) |
| A-028 | Par fit limit | 30 years; 5.0 bp | Limit of the `par` curve |
| A-029 | Common change | list median | `--common median`, alternatives `vw`, `file` |
| A-030 | Sensitivity step | h = 0.001 | `dur_premium1` |
| A-031 | Stale-price rule | close older than the SPY close | Leaves every sum |
| A-032 | Ticker lines | 20 tickers or fewer | Per-ticker bridge lines |

## Limits
- The value at the means is not the expected value and not the P50. A search can end at the floor of the rate at
  0%, not at the end of the range (23 of 57 `premium<` rows in the first run).
- The weight of a group is not a market weight unless the list is a market list. Sectors use today's SIC code.
- The split of the move depends on the order of the steps (the order effect is printed). It is not a cause.
- The run of 2026-11-20 (`analysis/priors/market_implied_2026-11-20.md`) was before the share-basis rule (A-017).
- Rule of the change pit-supplement-fill (Technology, merge pending): with `--as-of`, a missing input is filled from
  the 10-K supplement only when its SEC filing date is on or before the cut. The point-in-time test of this fill
  is the planted test and the unit tests of `rust/mapcheck`. `fa dcf pit` does not use the supplement path.
- Do not read a list premium or a ratio as a signal without a review (`HIDDEN-ASSUMPTIONS.md` #260).

## Code and command
| What | Path or command |
|---|---|
| Code | `rust/src/dcf/auto_implied.rs`, `rust/src/dcf/auto_attribute.rs`; series: `premium-series.md`; reality: `baserates-and-reality.md` |
| Model or parameter file | `analysis/priors/process_priors_2026-11-19.toml` |
| Command | `fa dcf implied --cache data/raw --tickers-file rust/mapcheck/universe/sp500.txt`; `fa dcf attribute --from D1 --to D2 --tickers-file FILE [--curve par] [--common median|vw|file]` |
| Tests | `rust/src/dcf/auto_implied_tests.rs`, `auto_attribute_tests.rs` |

## Review status
Not reviewed. First run: `analysis/priors/market_implied_2026-11-20.md`. No skeptic, reviewer or auditor looked at
either command. A-009 is INVALID until the reruns (see its header).
