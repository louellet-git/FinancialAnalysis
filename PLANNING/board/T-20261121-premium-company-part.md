---
id: T-20261121-premium-company-part
title: What explains the company part of the change of the implied premium (the change of a company less the common change)
area: research
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: fundamental
order: 0.006
branch: claude/premium-series
created: 2026-11-21
acceptance: (1) The design below is committed before any analysis of it; (2) `fa dcf attribute` writes, for each ticker, the beta of each date and the sensitivity of the value to the premium (d ln V / d premium at the premium of D1); (3) a description of the week 2026-11-13 to 11-20 for the S&P 100 (no test, about 65 names); (4) the test on the weekly history waits for the premium series (`T-20261121-sp100-week-attribution`, item 4.1 of the research plan) and has its own design, written before its run.
---
Owner words, 2026-11-21 about 19:00 UTC: "For the difference i.e attributable to the indiviudal company, that is somethign to be
investigated/understood." Context: in the week to 2026-11-20 the implied premium rose for all 65 S&P 100 tickers with a
value at both dates (median +0.78 points, the common part used now). The company part is the change of a company less
that median: NVDA +1.49, META +0.29, GOOG -0.30, MSFT -0.32, AAPL -0.44, WMT -0.45.

Design (Head of Fundamental Research, written before any analysis):
1. **Identity.** For a small change, the price change r_i of a company is about -D_i x dx_i plus the rates, time and facts
   parts, where D_i = -d ln V / d premium (the sensitivity of the value to the premium at D1) and dx_i the change of its
   implied premium. So dx_i is about -(r_i - other parts) / D_i. Two companies with the same price fall get different dx
   when their D differs. The company part therefore has three sources, measured in this order:
   - (a) **Sensitivity (mechanical).** D_i differs: a long-duration value (a low premium, a high growth) moves more for
     the same dx. Measure: D_i from the engine (two values at x1 +- 0.1 point).
   - (b) **Market exposure.** The price change has a market part, beta_i x r_m. Measure: beta_i of the model (D1 and
     D2), r_m = the value-weighted change of the list.
   - (c) **Company news.** The rest of the price change, r_i - beta_i x r_m. Candidates, fixed now: an earnings release in
     the week (SEC 8-K item 2.02 filed between D1 and D2; the model facts change only at the 10-Q or 10-K; in this week:
     NVDA 2026-11-17, WMT 2026-11-19); the sector (FF12); the level x1 (a premium near 0 gives a steep reading).
2. **Week of 2026-11-20 (description only).** One table for the S&P 100: dx_i, D_i, beta_i, r_i, the market part, the
   company return, the 8-K flag, the sector. The share of the cross-section variance of dx that (a), (b) and (c) give,
   in this order (the order is fixed; the parts do not add when they are not orthogonal: both orders are shown). No
   p-value and no claim with about 65 names in one week.
3. **History (a test, later).** With the weekly premium series, the same split each week from 2019; the design of that
   test (a pooled cross-section with week effects, standard errors clustered by week, an out-of-sample check) is written
   and frozen before its run.
4. **Rules.** A market-implied value is a measurement, not an input; the value at the means is not the expected value; no
   input of the model is set from the price; the model cannot tell a premium change from a growth change (both readings
   are shown); a residual is not a cause.
