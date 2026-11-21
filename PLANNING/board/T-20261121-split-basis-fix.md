---
id: T-20261121-split-basis-fix
title: Put each past price on the share basis of its SEC vintage (split look-ahead), and add a live split guard
area: engine
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: fundamental
order: 0.01
branch: claude/dcf-engine-backtest
created: 2026-11-21
acceptance: A backtest or a study that compares a past model value per share with a past price uses a price on the share basis of the vintage of that date; a test multiplies all prices before a later date by k and shows that no earlier decision changes; a live split guard stops a pick whose share count of the last 10-K and the current share basis differ by a split; the runs that the defect touched run again once, as new logged variants, after a test file written before them.
---
Found by the strategy gate (Head of Quality and Risk, 2026-11-21, CRIT F1). The price files of the store are
split-adjusted to today (`prices.meta.json`: "adjusted": "splits"). The model value per share uses the share
count of each SEC vintage. So a company that split after the date gets a value per share that is too high by the
split factor. Example: BKNG closes at 72.94 on 2018-01-05 in the store (it traded at about 1,800); 25:1 split
filed 2026-04-28. The later splitters are the winners of 2018 to 2026, so the defect made the value rank look good.

Touched (invalid until the reruns): the variant log runs 1 to 14 and R1, R2
(`analysis/priors/dcf_engine_backtest_2026-11-21.md`); the blind projection T2 to T4
(`analysis/priors/blind_projection_test_2026-11-21.md`); `fa dcf returns` and its recorded files
(`analysis/priors/auto_returns_*`); the value-rank backtest of pull request 104. Not touched: `fa dcf pit` (no
price), momentum (price only), the market-level test (an index), the NVDA driver model (its own share count; the
10:1 split of 2024 is before its last 10-K).

Plan:
1. Data: a split table (ticker, ex-date, ratio) or unadjusted closes, from the Head of Technology (request
   2026-11-21). No guess from the share-count steps for a result of record.
2. Code: one function that gives the factor from a date to today; the strategy and `fa dcf returns` divide the
   price by it (or multiply the shares). The k-test. A live guard: the current share basis against the last 10-K.
3. A new test file (before the reruns), with the other gate findings: a no-rebalance mode (F2), the cut at each
   rebalance Friday (F3), a deflated t and a confirmation window that the design did not use (F4).
