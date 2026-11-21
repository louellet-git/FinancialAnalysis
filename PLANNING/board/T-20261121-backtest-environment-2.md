---
id: T-20261121-backtest-environment-2
title: Backtest environment parts 2 and 3 (total return, risk outputs, the deflated Sharpe, point-in-time members)
area: trading
status: backlog
owner: louellet-git
requested_by: louellet-git
head: technology
order: 4
branch:
created: 2026-11-21
acceptance: The harness reports total return with dividends, the risk outputs of the Head of Quality and Risk and a deflated Sharpe from the variant count; part 3 has a point-in-time member list with delisted names, from a source whose terms permit it.
---
The rest of `PLANNING/board/done/T-20261121-backtest-environment.md` (the needs of the heads are there).
Part 2: total return with dividends (a source first); the risk outputs (max drawdown and its length, exposure, concentration,
worst day and worst 5 days, CVaR 95/99, stress dates, turnover); the deflated Sharpe by name; the equal-weight line on the
scored names. Part 3: point-in-time index members and delisted prices (a new source, terms first; no free source with
open terms was found on 2026-11-21 for prices before 2016). The split basis is ticket `T-20261121-split-table`.
Only for the heads that need it (word of the person, 2026-11-21).

## Harness notes for the next change (Head of Quality and Risk, 2026-11-21, MIN)
1. The poison check compares only the targets and the trades before D: a score that moves without crossing a rank does
   not show. Compare the signals of D and earlier too (a small change inside `poison_compare`).
2. `index_from_cache` drops the market index with no message when SPY has fewer than 520 rows (the beta falls back to 1),
   so a truncation test cut early looks like a look-ahead (Fundamental Research hit this). Print a warning when the index
   is dropped.
