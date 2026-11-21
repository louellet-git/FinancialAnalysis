# Model: what the price implies, and where the price move went (`fa dcf implied`, `fa dcf attribute`)

Updated: 2026-11-21. Commit: 90ed737 (base of the branch). These two commands share one document: they share
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
| Facts | With `--as-of` or `--from/--to`: facts filed on or before the date | A 10-K of the cache does not fill a missing input |
| Curve, beta | GSW file and SPY of the date | A curve older than 3 days is flagged in `attribute` |
| Groups | SIC code of the cache (today's code), or `--groups FILE` | Fama-French 12 sector; industry with 5 tickers or more |

## Method (short)
1. Value at the means: one path, each parameter at its mean, each shock zero (`PathEngine::deterministic`). No seed.
2. `implied`, three bisections, one number each, all others fixed: the premium mean (-4% to +20%), the growth
   shift (-30 to +60 points), the margin shift (-50 to +50 points). No solution: the bound and a flag.
3. List premium: the premium for which the median of value / price of the list is 1. Then each group gets the
   median of its value at that premium over its price.
4. `attribute`: the price of each date is the value of the model of that date at the premium that the price
   implies (x1 at D1, x2 at D2). The change `ln(P2/P1)` moves in steps: facts, time (stub), rates, then the plug
   (premium, with the equity weight and the beta of D2), and the search residual. Both orders of rates and plug
   are printed; their range is the order effect.
5. Two chains: the premium view and the growth view (where the premium view has no value). A third line: the
   growth shift that P2 needs at the premium of D1.
6. A list: the whole-list change weighted by price x shares at D1. The share count is guarded (A-016).

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
| A-004 | Equity risk premium | 4.41% | The model premium that the search moves |
| A-017 | Share basis | filing date rule | Past price against filed shares |

## Limits
- The value at the means is not the expected value and not the P50. A search can end at the floor of the rate at
  0%, not at the end of the range (23 of 57 `premium<` rows in the first run).
- The weight of a group is not a market weight unless the list is a market list. Sectors use today's SIC code.
- The split of the move depends on the order of the steps (the order effect is printed). It is not a cause.
- The run of 2026-11-20 (`analysis/priors/market_implied_2026-11-20.md`) was before the share-basis rule (A-017).
- Do not read a list premium or a ratio as a signal without a review (`HIDDEN-ASSUMPTIONS.md` #260).

## Code and command
| What | Path or command |
|---|---|
| Code | `rust/src/dcf/auto_implied.rs`, `rust/src/dcf/auto_attribute.rs` |
| Model or parameter file | `analysis/priors/process_priors_2026-11-19.toml` |
| Command | `fa dcf implied --cache data/raw --tickers-file rust/mapcheck/universe/sp500.txt`; `fa dcf attribute --from D1 --to D2 --tickers-file FILE` |
| Tests | `rust/src/dcf/auto_implied_tests.rs`, `auto_attribute_tests.rs` |

## Review status
Not reviewed. First run: `analysis/priors/market_implied_2026-11-20.md` (ticket `T-20261120-dcf-market-implied`).
Attribution: ticket `T-20261120-valuation-move-attribution`. No skeptic, reviewer or auditor looked at either.
