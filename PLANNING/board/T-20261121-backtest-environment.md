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
