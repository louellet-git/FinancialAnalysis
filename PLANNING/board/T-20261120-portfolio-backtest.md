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

State, 2026-11-20 (Head of Technology), after review cycle 1. `fa portfolio backtest --strategy value-rank|momentum`.
Real run on the store, `sp500.txt` (502 tickers), 2018-01-01 to 2026-11-13, monthly, 10 bp (price returns; today's
ticker list, so the result is too good): value-rank +19.02% a year (431 tickers scored, first position 2018-01-02)
against SPY +12.35%; momentum +19.82%; equal-weight of the whole list +14.56% (same costs). The first version of
value-rank gave +9.51%: it took the latest filing of each fiscal year, so a year became known about 2 years after its end (about 790 days). The 550-day cap then dropped almost each ticker (0 to 19 of 119 scored in the review run), and the portfolio held cash on 766 of 2,190 days (35%). The new rule (the vintage of each rebalance date: only facts filed before it) gives 57 to 93 scored in the same run and 0 days in cash. On the full list the command prints 290 to 377 scored at the rebalances (median 355) and 0 days with no position. The parameter table is in-sample. Poison test of the review (each fact filed from 2022-06-01 times 1000, on a copy of the store): after the fix the scores change from 2022-07-01 and the portfolio from 2022-09-01 (before it, the portfolio first differed on 2020-11-03). The unit poison tests scale each ticker by a different factor, and a mutation check shows that they fail when `known` reads the last vintage or when the strategy reads the price series after the date. Value-rank takes 88 s (was
17 s): one set of years for each annual filing day. Next: each research head writes its own strategy on this engine;
an option bot as a strategy, a list of past index members, and dividends are not in this version.
