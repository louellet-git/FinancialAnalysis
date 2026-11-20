---
id: T-20261120-portfolio-backtest
title: Build one portfolio backtester for the strategies of the two research heads
area: trading
status: review
owner: louellet-git
head: technology
order: 2
branch: claude/portfolio-backtest
created: 2026-11-20
acceptance: One command runs a strategy over past dates with point-in-time data only and gives the return, the drawdown, the turnover and the costs against a benchmark, for one fundamental strategy and one quant strategy, and the Head of Quality and Risk passes its look-ahead test.
---
Owner request, 2026-11-20: "we need a means of backtesting".

What exists: `fa trade backtest` (option bots, model prices only), `fa backtest <T>` (forecast errors of one
company, not a portfolio), `fa dcf returns` (a measurement of value against later returns, not a strategy).
No code turns a signal into positions and a profit and loss over time.

Scope (lean, Rust):
1. Input: for each rebalance date, a list of (ticker, target weight) from a strategy. The strategy sees only
   data known at that date (SEC facts by their filing date, prices up to the day before).
2. Engine: positions, daily marked to the close, costs (a spread and a fee in basis points), cash, benchmark
   (SPY). Output: return, volatility, drawdown, turnover, cost, and the same for the benchmark.
3. Two first strategies, one for each research head: Fundamental, a rank of the automatic model value against
   the price (`fa dcf auto`); Quant, one option bot of `fa trade` (compare with `fa trade backtest`).
4. Tests: fixed answers on a small made-up price set; a look-ahead test (a strategy that reads tomorrow fails).

Known limits to state in each result: the ticker lists are today's lists (survivorship), the option prices of
`fa trade` are model prices.

Owners: the Head of Technology builds the engine; each research head writes its strategy; the Head of Quality
and Risk validates (skill `point-in-time-test`).

State, 2026-11-20 (Head of Technology). `fa portfolio backtest --strategy value-rank|momentum`. Real run on the
store, `sp500.txt`, 2018 to 2026-11-13, monthly, 10 bp (price returns; today's ticker list, so the result is
too good): value-rank +9.51% a year against SPY +12.64% (new automatic model defaults of 2026-11-20); momentum
+19.82% against +12.35%. Next: each research head writes its own strategy on this engine; an option bot as a
strategy, a list of past index members, and dividends are not in this version.
