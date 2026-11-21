# A-021: conventions of the portfolio backtest for the value-rank strategies

Updated: 2026-11-21. Commit: 90ed737 (base of the branch).

## Choice
`fa portfolio backtest --strategy value-rank` (monthly or quarterly; `value-rank-weekly` is A-009):
- Score: P50 / price of the automatic model, from the SEC facts filed before the date (a vintage), with the
  curve, the beta and the price of the day before. Default 300 paths, seed 7 (`--paths`, `--seed`).
- Weight: equal weight in the top quintile of the scored list (`top_quintile`: at least 1 name, one fifth of the list).
- A strategy with fewer than 10 scored tickers holds cash (`MIN_SCORED` = 10).
- `--neutral sector`: the top (or bottom, `--pick bottom`) fifth inside each Fama-French 12 sector, with the
  sector weights of the scored list.
- Cost: 10 basis points of each traded value, buy and sell (`--cost-bp` 10). Price returns, SPY as the benchmark.
- A close older than 10 days is not a price to trade at (`STALE_DAYS`; A-012).
- The universe: the tickers of the list that have a price file. The lists are today's lists (survivorship).

## Where used
| Model | File | Parameter name |
|---|---|---|
| Strategies | `rust/src/portfolio/strategies.rs` | `ValueRank`, `top_quintile`, `MIN_SCORED`, `Pick` |
| Engine and cost | `rust/src/portfolio/mod.rs` | `Config::cost_bp`, `STALE_DAYS` |
| Harness | `rust/src/portfolio/harness.rs` | hypothesis file, run log, poison check |
| Hypothesis files | `trading/hypotheses/value-rank-sp500-v2.toml` | `prior_variants` = 5 |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Top fifth of the whole list | A-009: the gain is the choice inside a sector |
| Another cost level | The harness prints the cost levels 0x, 1x and 2x |

## Why this choice
The hypothesis file says "after a cost of 10 bp on each traded value" and the test period from 2018-01-01. The
top quintile, 10 bp and 300 paths have no other source in a file read. The parameter table is in sample (fitted on
all years through 2025): the out-of-sample window from 2023 is not clean for value-rank (`trading/hypotheses/README.md`).
The runs before the share-basis rule (A-017) have a split look-ahead (A-009 header). Momentum is not in this area.

## Effect on the result
The exploratory run of 2018 to 2026-11: +19.02% a year against +14.56% for the equal weight (pull request 104;
hypothesis file v2). That run was before A-017 and is not valid until the reruns (ticket `T-20261121-split-basis-fix`).

## Who decided and when
Not known by name. The harness and the file v2: 2026-11-21. Review: not done.

## When to examine it again
After the reruns with the share basis. When point-in-time index lists exist (survivorship).
