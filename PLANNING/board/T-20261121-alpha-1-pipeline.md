---
id: T-20261121-alpha-1-pipeline
title: Put the best candidate through the three stages, isolation, bigger data, paper
area: trading
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: quant
order: 0.8
branch: claude/quant-alpha-1
created: 2026-11-21
acceptance: Stage 1 gives, for the candidate s05_ic_0dte and for the volatility premium it sells, a test in isolation with a dated hypothesis, the trial count, the deflated Sharpe ratio and t, and the result under other model prices; the gate decision (go to stage 2 or stop) is written with its numbers.
---
Owner request, 2026-11-21: "First test in isolation, then on a bigger dataset, then try on paper after and improve
the model recursively." No alpha is proven today (record of 2026-11-21). The best candidate in the repo is the bot
`s05_ic_0dte` (same-day SPY iron condors; `trading/research/LEADERBOARD.md`: out-of-sample Sharpe 1.69 on 56 trades),
but each price in that result is a model price, the sample is small, and the leaderboard says that some bots peeked.

## Stages and gates (each gate is written down before the stage starts)
1. Isolation (now, stored data): (a) the premium itself: VIX against the later 30-day realised volatility of SPY,
   1990 to 2026 (`data/raw/trading/_VIX_1d.csv`, `SPY_1d.csv`), and VIX1D against the same-day realised move, 2023 to
   2026: is the premium there, how large, and in which stress windows (`analysis/macro/2026-11-21-stress-windows.md`)
   does it lose? (b) s05 again with a dated hypothesis, every variant counted, the deflated Sharpe ratio and t, and the
   result when the model price changes (spread x2, skew ±50%, a fill at the bid or ask, not the mid).
   Gate 1: the premium is positive outside the stress windows, and s05 keeps t > 2 and a positive return under the
   worse prices. Else stop and write why.
2. Bigger data: the daily SPY chain copies with bid and ask (from 2026-11-23, 20 days): the real credit of each condor
   that s05 would sell, against its model credit. Gate 2: the real credit is at least 80% of the model credit.
3. Paper: s05 in `fa algo` (an option strategy and an option quote source are a later ticket) for at least 20 market
   days, the same rules frozen. Gate 3: the paper result is inside the range that stage 1 predicts.
Then improve and repeat: each change is a new trial, counted, and goes through the gates again.

Agent: `strategy-researcher` (with `volatility-derivatives-quant` for the option prices). Gate: `model-validator`.
Limits: model prices; no order code; `fa algo` stays outside the sleeves until limits in percent of capital exist.
