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
(To be filled from the answers.)
