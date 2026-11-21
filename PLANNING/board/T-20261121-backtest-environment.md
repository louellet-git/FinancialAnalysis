---
id: T-20261121-backtest-environment
title: Make one backtesting environment for the stock and option strategies of the heads
area: trading
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: technology
order: 1
branch: claude/backtest-environment
created: 2026-11-21
acceptance: The needs of the Quant, Fundamental and Quality and Risk heads are written in this ticket; the first part that they agree on runs as one command on a copy of the data store, with a point-in-time test.
---
Words of the person (2026-11-21): "Please get to work on the backtesting environment, ask the heads what they require."

What exists now: `fa portfolio backtest` (stocks: value-rank, momentum; SEC vintages; monthly rebalances;
cost 10 bp; equal-weight line and SPY), `fa trade backtest` (option bots on SPY/QQQ with model option
prices), `fa backtest <TICKER>` (revenue and FCF forecast vintages), `fa dcf pit` and `fa dcf returns`.

## Needs of the heads
### Head of Quant Research (2026-11-21), ranked by need
1. Strategies: (a) SPX volatility premium: trade options where the market implied volatility differs from
   `fa vol fit`, delta-hedged with SPY, hold 1 to 30 days, daily rebalance; (b) stock factors: momentum and an
   implied-cost-of-capital value factor (`fa dcf implied --as-of`, not `fa dcf auto --as-of`, which looks
   ahead), cross-section, monthly; (c) option relative value from `fa options check` (box rate, SPX against SPY
   parity): a measurement first. The option bots stay as they are.
2. Data: the daily bid/ask chain copies (from 2026-11-23) as the only option fill source; total return
   (dividends) for stocks and SPY; point-in-time index members and delisted prices; SOFR/EFFR/TGCR history;
   VX futures history.
3. Execution: option fill at bid or ask with a parameter for the share of the spread paid (default the
   touch); commission per contract and fees; a stated Reg-T margin approximation for short options; borrow
   fee for a short stock; the SPY hedge pays its spread; limits in delta and vega.
4. Outputs: daily NAV, drawdown, Sharpe and Sortino, turnover, exposure (delta, vega), P&L by Greek for
   option strategies, a per-trade log in the form of `trading/journal/`.
5. Validity: quote time not after decision time; walk-forward with frozen parameters; a held-out last 30%;
   the number of variants tried, with a deflated or haircut Sharpe; a run at 2x cost; 2020-03 and 2022 shown
   apart.
6. Must not: a fill at a model price or at the mid for a result that a person uses.

### Head of Fundamental Research (2026-11-21), stocks only, ranked by need
1. Strategies: (a) rank by model value / price (value at the means and P50 of the automatic model, built
   point-in-time), long the top quintile, monthly, held 12 months; (b) the same inside each FF12 sector;
   (c) rank by the implied premium of `fa dcf implied`; (d) later: entry the day after a 10-K, 10-Q or 8-K guide.
2. Data: point-in-time index members (S&P 500 and next 500), delisted prices, total return with dividends,
   the GSW curve of each past date. The model value of each past date is built with our code, not stored.
3. Execution: trade at the close of the first session after the signal date (the latest filing date of the
   facts used); 10 bp a side; equal weight; at most 25% in each FF12 sector; no short in part 1.
4. Outputs: Spearman rank correlation of value/price with the 12-month forward return, by year; quintile
   spreads; hit rate of "below P25 / above P75"; NAV and drawdown; a trade log with the ticker, date, model
   value, price and the filing accession of the signal.
5. Tests: facts filtered with `CompanyFacts::filed_up_to`; the poison test (later facts x1000: same result);
   the close on or before the date; the universe of that date; rule built on `--half 0`, scored on `--half 1`
   and `--cut 2015`; survivorship stated in each result.
6. Must not: a model input set from the price; a rule that the scoring set selects.

### Head of Quality and Risk (2026-11-21)
1. Gate for a result used for a paper or live trade: `model-validator` (reproduces the run from its log,
   poison and out-of-sample checks), `auditor` (headline numbers from the trade list), `risk-compliance-officer`
   before the first live use and for each new strategy class. Evidence: the run log (code commit, store
   commit, config, seed), the pre-registered hypothesis with its date, the list of all variants tried, the
   trade list, the cost model, the risk outputs, the write-up in `trading/journal/`. Label paper against live
   and model price against market price.
2. Validity (each a gate check): a poison test for each data source that the strategy reads; an
   out-of-sample period fixed before the first run; a record of each variant (a hidden variant is a CRIT) with
   a deflated Sharpe or similar control; the universe of each date with delisted names; costs at 0x, 1x, 2x;
   model prices named, and the result on market prices for the dates with quotes.
3. Risk outputs (part 2): max drawdown and its length, exposure and gross notional, concentration, worst
   day and worst 5 days, CVaR 95/99, Greeks for options, stress dates (2008-10, 2020-03, 2022, 2026-11),
   capital need at the peak, trade count and turnover.
4. Rules in code: no order code (`rg -n -i "place_order|submit_order|/orders"` stays empty); no input set from
   the scored outcome or price; no run without a pre-registered hypothesis file (hash and date in the run
   log); each run logged, add-only, as the source of the variant count; a run that reads the out-of-sample
   window is flagged; results reproducible from the log; live data and keys out of the backtest path.
5. Must not: any path that sends an order or holds broker keys.

## Common needs
- All three: point-in-time decisions with a poison test; costs that are paid (never a mid or model fill for
  a result that a person uses); a per-trade log; survivorship stated or removed.
- Quant and Fundamental: total return with dividends; point-in-time index members and delisted prices.
- Quality and Risk: a hypothesis file, an add-only run log that counts the variants, a locked
  out-of-sample window.

## Proposal for part 1 (the run harness, stocks)
Build on `fa portfolio backtest`, no new engine:
1. `--hypothesis FILE`: a run needs a hypothesis file (`trading/hypotheses/<id>.toml`: the hypothesis, its
   date, the strategy and its parameters, the universe, the out-of-sample start). No file, no run.
2. Run log: one file for each run, add-only, `trading/runs/<id>/<UTC time>.json` (code commit, store commit,
   the hypothesis hash, the full config, the results). The count of files of an `<id>` is the variant count,
   printed in each result. One file for each run, so heads never write the same file.
3. Out-of-sample lock: a run whose end is after the out-of-sample start gets the flag `oos-read` in its log
   and in its output.
4. Costs: each run prints the result at 0x, 1x and 2x cost.
5. Trade log: `--trades FILE` (CSV: date, ticker, action, weight, price, signal value, filing accession).
6. `--poison-check D`: the run is done two times, the second with each fact filed after D and each close
   after D changed by a factor that differs by ticker; the command fails if a weight before D moves.
Part 2 (after part 1): total return with dividends, then the risk outputs, the deflated Sharpe ratio (the variant count of part 1 is a warning, not a multiple-testing control), and the equal-weight line on the scored names (part 1 prints the line of the whole list). Part 3: point-in-time members and
delisted prices (a new source, terms first). The model-value strategy (a) of Fundamental Research and the
option strategies of Quant Research use the harness when they are ready; the option fills wait for enough
days of saved chains.
