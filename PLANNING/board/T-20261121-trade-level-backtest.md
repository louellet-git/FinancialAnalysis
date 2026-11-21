---
id: T-20261121-trade-level-backtest
title: Trade-level backtest engine in Rust: DCF signals on many stocks and dates, buy or short, exit at a target or a time, walk-forward rule choice
area: engine
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: fundamental
order: 0.01
branch: claude/trade-level-backtest
created: 2026-11-21
acceptance: (1) A Rust signal panel gives, for each ticker and each week, the point-in-time DCF output (P10, P25, P50, P75, P90, mean, price, filing accession) from facts and prices known at that date only, computed once and cached; (2) a Rust trade simulator turns each signal into a long or a short trade, entered at the open of the next trading day, closed at a price target, a stop or a time limit, with costs and a short borrow fee, and writes a trade log; (3) a walk-forward run chooses the rule at each date from a grid fixed before the run, using only trades closed before that date, and its P&L is the reported expected return; (4) a grid of 500 rules over 2018-2026 runs in under one minute after the panel exists; (5) the truncation and k-tests of the earlier work pass for the panel.
---
Owner words, 2026-11-21 14:25 UTC: "We need a backtesting engine, then we will need to use our DCF engine across a variety
of stocks and different points in time in combination with our backtesting engine until we find a positive expected
return, any questions? We need rust for this in particular so that we can optimize our run times". Answers: the data
rule is "Any data that is ahead of the point in time, since we are dealing with stochastic processes they must be F_t
adapted. If we're simulating a DCF in 2004, we don't use info from 2005"; the measure is "We effectively just need to
know if the stock is a buy or a short, and for how long or until what price point, then we sell or buy back accordingly
and that's our profit over that time period. Accounting for reality after trade would have supposedly been placed".

Design (Head of Fundamental Research, written before any run):
1. **Signal panel** (`fa portfolio panel`): each Friday, each ticker of the list: the automatic model (split basis,
   vintages filed before the date, the curve and the beta of the day before) gives P10, P25, P50, P75, P90 and the mean
   per share; the close of the day before; the filing accession. Computed once in parallel and stored as a binary or
   CSV cache keyed by the store commit and the code commit. It is the costly part (about 150,000 models).
2. **Trade simulator** (`fa portfolio trades`): a rule reads only the panel row of date t and the prices before t.
   - Signal: long if the close is at or below a chosen quantile of the value (for example P25); short if it is at or
     above another (for example P75).
   - Entry: the open of the next trading day (the order is placed after the signal).
   - Exit: the first of: a price target (for example the P50 of the entry row); a stop (a loss of x%); a time limit
     (n weeks), at the close. A target is tested on the daily high (long) or low (short).
   - Cost: a spread and commission in bp on each side; a short pays a borrow fee a year (general collateral, stated).
   - Output: one row a trade (ticker, side, entry and exit dates and prices, reason of exit, return after cost).
3. **Walk-forward choice (F_t-adapted at the rule level).** The grid of rules is fixed before the first run. At each
   date t, the rule in use is the one with the best mean return of the trades of each rule that CLOSED before t (with a
   minimum count; before that, no trade). The reported expected return is the P&L of these walk-forward trades. The
   in-sample best rule of the whole period is shown only as a label "chosen with later data, not a result".
4. **Speed:** Rust, rayon, the panel read once; the rules run on the cached panel.
5. **Checks:** the panel passes the truncation test and the split k-test; a planted read of a later price changes a
   decision (the test of the test); the simulator has fixed-answer tests for each exit reason, the next-open entry,
   the costs and the borrow fee.
Limits: prices from 2016-11 and SEC facts from about 2009 (no stock run before about 2018); today's lists
(survivorship; no delisted names); price returns (a short also pays no dividend in the test).
