---
id: T-20261121-dcf-engine-backtest
title: Test the DCF engine on the backtester, over the longest data that exists
area: engine
status: done
owner: louellet-git
requested_by: louellet-git
head: fundamental
order: 0.05
branch: claude/dcf-engine-backtest
created: 2026-11-21
merged: pull requests 133 (market level) and 136 (stock level: no evidence of an edge), 2026-11-21; the longer history continues with the Head of Technology and the Head of Economics
acceptance: A findings file gives, for the automatic DCF model, the point-in-time portfolio result of `fa portfolio backtest --strategy value-rank` (and the rank test of `fa dcf returns`) over the longest stock window of the store, against SPY, equal weight and momentum, at 0x, 1x and 2x cost; and a market-level test of the same discount logic over the longest index history (with the Head of Economics), with the result of each period stated. Each number has its command and store commit. A rule that a scoring period selects is not used.
---
Owner word, 2026-11-21 03:17 UTC: "I need you to begin working on testing your DCF engine on the back
tester. That's the real value. You and the chief of economics will have to look for as long of data
as you have, chief of economics could/should even consider periods of time that are outside the
length of the stock market when that makes sense."

Data length in the store (2026-11-21): SEC company facts from about 2009; most stock prices from
2016-11-18; SPY and IBM prices from 1962; the GSW curve from 1961. So the stock-level window is about
2017 to 2026 now. A longer stock window needs older prices (request to the Head of Technology).

Plan (Head of Fundamental Research):
1. Stock level, now: `fa portfolio backtest --strategy value-rank` (the automatic model, point in
   time, PR 104) from the first date with enough vintages, quarterly and monthly, against SPY, the
   equal-weight list and momentum. Read with the earlier rank test `analysis/priors/auto_returns_2026-11-20.txt`.
2. Stock level, longer: prices from 2009 for the universe (Head of Technology, data store), then the
   same runs from 2010.
3. Market level, long (with the Head of Economics): the same discount logic on the index (earnings or
   payout, the curve of each date, a premium rule fixed before the run) and the later 5- and 10-year
   returns, as far back as the series go. Then the premium that the price implies at each date
   against the later return.
4. Findings file (skill `investigation-report`), then the gate of the Head of Quality and Risk.
Survivorship: today's lists. Each result says so. The harness of the Head of Technology
(`T-20261121-backtest-environment`, part 1) gives the run log and the poison check when it is merged.
