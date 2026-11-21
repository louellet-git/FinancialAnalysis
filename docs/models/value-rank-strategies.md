# Model: value-rank strategies of the portfolio backtest (`fa portfolio backtest`, `fa portfolio calibrate`)

Updated: 2026-11-21. Commit: 90ed737 (base of the branch). This document is for the value strategies only.
The strategy `momentum` belongs to the Head of Quant Research. No order goes to a broker.

## Purpose
Test whether the P50 / price of the automatic model (`docs/models/auto-path-model.md`) ranks stocks well enough
to trade: a monthly or quarterly top quintile (`value-rank`) and the weekly rule of A-009 (`value-rank-weekly`).
`fa portfolio calibrate` tests the weekly rule against what happened (tests T2 to T4 of the blind projection test).

## Inputs
| Input | Source | Note |
|---|---|---|
| Ticker list | `--tickers-file`, for example `rust/mapcheck/universe/sp500.txt` | Today's list (survivorship) |
| Prices | `data/raw/<T>/prices.json` (closes, split-adjusted, no dividends) | The strategy sees the days before the rebalance date |
| SEC facts | `data/raw/<T>/company_facts.json[.gz]` | A vintage: facts filed before the date only |
| Table | `analysis/priors/process_priors_2026-11-19.toml` (`--table`) | In sample; a cut table is `trading/hypotheses/data/process_priors_cut2016.toml` |
| Sector | The SIC code of the cache, Fama-French 12 (today's code) | For `--neutral sector` and the weekly rule |
| Split table | `<cache>/_corporate/splits.csv` | An error if missing (A-017) |

## Method (short)
1. On each rebalance day D, score each ticker: P50 / price of the automatic model with the vintage known before
   D and the curve, beta and price of the day before (`ValueRank::score`, `model_p50`).
2. `value-rank`: equal weight in the top quintile of the list (at least 10 scored names, else cash). With
   `--neutral sector`: the top (or `--pick bottom`) fifth in each sector, with the sector weights of the list.
3. `value-rank-weekly` (A-009): each Friday, the 3 best-ranked names inside their sector, not held, one for each
   sector, in the top fifth; 1/39 each; hold 13 weeks (91 days). Use `--rebalance weekly` and `--hold true`.
4. The engine trades at the close of D, marks to the close each day, charges 10 basis points of each traded value,
   and compares with SPY and with the equal-weight line of the list.
5. `calibrate`: for each Friday, the rank inside the sector from the data before it; then the outcome (return to
   91 days later, less the equal weight of the scored list). T2 rank against outcome, T3 the entries, T4 at any time
   (years and halves). The text gives each test with its pass rule, its value and PASS or FAIL.
6. The harness (`harness.rs`) reads a registered hypothesis file (`trading/hypotheses/<id>.toml`), writes the
   add-only run log (`trading/runs/<id>/`), the variant count, the out-of-sample flag and the poison check
   (`--poison-check D`: each close and fact from D on is changed; a target weight must not move).

## Outputs
Total and annual return, volatility, maximum drawdown, turnover and cost, against SPY and the equal-weight
line; `--out` (daily value), `--holdings`, `--trades` (CSV). The run log of a registered hypothesis.
`calibrate`: T2, T3, T4 and a headline (ALL PASS or NOT ALL PASS). It has no run log.

## Assumptions
| Record | Name | Value | One line |
|---|---|---|---|
| A-009 | Weekly trade rule | 3 a week, top fifth of a sector, 1/39, 13 weeks | Decided 2026-11-21. Marked INVALID in its header (split look-ahead) until the reruns |
| A-021 | Backtest conventions | top quintile, 10 bp, 10 scored names, 300 paths | Cost 10 bp: hypothesis file; others: no source |
| A-012 | Age of a close | 10 days (`STALE_DAYS`), 7 days (`FRESH`) | Conventions |
| A-017 | Share basis | filing date rule | Backtest prices against filed shares |
| A-016 | Guard of the share count | 0.01x to 100x | A data check of the weekly rule |
| A-013, A-014 | Table and test design | see records | The table is in sample |

## Limits
- Survivorship: today's list. A failed company is not in it, and an index addition is in it for the whole test.
- Price returns (no dividends); SPY has the same limit. No tax, no short sale, no leverage, no market impact.
- The table is fitted on all years to 2025: the out-of-sample window from 2023 is not clean for `value-rank`.
- All results before `T-20261121-split-basis-fix` have a split look-ahead (A-009 header). The numbers in A-009
  (R1: +21.37% a year against +14.80%; R2) and in `analysis/priors/dcf_engine_backtest_2026-11-21.md` are not valid.
- The weekly rule is held in the backtest with a rebalance to 1/39 unless `--hold true`.
- A short side is not used before its test passes.

## Code and command
| What | Path or command |
|---|---|
| Code | `rust/src/portfolio/mod.rs`, `strategies.rs`, `harness.rs`, `calibrate.rs`, `cli.rs` |
| Model or parameter file | `trading/hypotheses/value-rank-sp500-v2.toml`, `a009-weekly-hold-v1.toml`, `analysis/priors/process_priors_2026-11-19.toml` |
| Command | `fa portfolio backtest --strategy value-rank --start 2018-01-01`; `--strategy value-rank-weekly --rebalance weekly --hold true`; `fa portfolio calibrate --start D` |
| Tests | `cargo test --manifest-path rust/Cargo.toml --locked` (tests in `portfolio/`) |

## Review status
Not reviewed as a valuation. Review of the weekly rule: not done (the gate of the Head of Quality and Risk before
2026-11-27, A-009). The strategy gate found CRIT F1 on 2026-11-21 (A-009 header); the reruns are open.
