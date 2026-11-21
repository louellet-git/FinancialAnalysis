---
id: T-20261121-blind-projection-test
title: Test the blind projections of the DCF engine on prior data before the first weekly trade
area: engine
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: fundamental
order: 0.02
branch: claude/dcf-engine-backtest
created: 2026-11-21
acceptance: Before the first weekly trade (paper included, 2026-11-27), the four tests below run once as written, on point-in-time data only, and their results (pass or fail, each with its number and command) are in a findings file that passed the gate of the Head of Quality and Risk; each weekly write-up then states the backtested chance of a loss and the size of a bad outcome for its rule, so that a failed trade is one that the model gave a chance before the trade.
---
Owner word, 2026-11-21 04:29 UTC: "Ensure that before you have trades and we begin the weekly trades, that you have
effectively back tested your future state blind projections using the DCF engine and using prior data. A
recommendation should work at any time and point and when it doesn't, the model should have already priced it in
such that it does work. You can speak w the economist about this."

Reading of the head: (1) the projections of the engine, made blind (only the facts and prices known at the date),
must be calibrated against what happened; (2) a trade can fail, but the rate and the size of the failures must
be what the backtest of the same rule said before the trade. A rule that works only in some years fails (2).

Design (written 2026-11-21 before any of these runs; review by the Head of Economics and of Quality and Risk;
data store `1804f48`; parameter table cut at 2016 for the stock tests, `--cut 2015` for the projection test):
- **T1. Projections of the fundamentals.** `fa dcf pit --cut 2015` (base years 2015 and later, table from the
  years up to 2015): free cash flow and revenue at T+1, T+2, T+3. Pass: in the "all" row the share of real values
  inside P10 to P90 is 75% to 85%, and the share below P10 and the share above P90 are each 5% to 15%. A class row
  that fails is named, and the weekly rule must exclude that class or its band must be fixed and tested again.
- **T2. The signal at every date.** Each week of 2018-01 to 2026-11, each scored ticker of the S&P scored list:
  the rank inside its sector (A-009 item 3) against the return of the next 13 weeks less that of the equal weight.
  Deciles of the rank. Pass: the top decile less the bottom decile is above 0 with a one-sided p < 0.05
  (block bootstrap by week, block of 13 weeks, seed 7), AND it is above 0 in at least 7 of the 9 calendar years.
- **T3. The trades of the rule.** The entries of R1 (A-009): the share of entries with a 13-week excess return
  above 0 (the hit rate), the mean, and the P10 and P5 of the 13-week excess return. Pass: hit rate above 50% with
  a one-sided p < 0.05 (the same bootstrap). These numbers are the "priced in" figures of each weekly write-up.
- **T4. At any time.** T3 by calendar year and by half of the list (`ticker_half`). Pass: the mean 13-week excess
  of the entries is above 0 in at least 7 of 9 years and in both halves. The worst year and its numbers are
  stated in each write-up.
If a test fails, the weekly trades do not start on its basis; the head reports the failure to Luc with the numbers.
No rule is changed after a result to make a test pass. Survivorship, price returns and today's lists stay as limits.
