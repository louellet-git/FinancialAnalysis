---
id: T-20261120-valuation-move-attribution
title: Explain where the market value went in a price drop, one stock at a time, then the market
area: engine
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: fundamental
order: 0.2
branch: claude/valuation-move-attribution
created: 2026-11-20
acceptance: For a ticker and two dates, a command splits the change of the price into the part that the rates explain, the part that a change of the implied premium explains and the part that a change of the implied growth explains, with the value engine; the same for a list of tickers by sector; a findings file gives the result for NVDA and for the S&P 500 list for the drop of November 2026.
---
Owner word, 2026-11-20: "I need to know why the market crashed today and over past few days, generally what I'm
looking for is an understanding of where the market valuation WENT. This is a large task that requires its own
models being built, and probably will start with you doing one stock at a time until you've built a general enough
model to explain."

Reported facts (two news summaries read on 2026-11-20 by a search tool; check with the price data before use):
on 2026-11-20 the S&P 500 fell about 3.7% and the Nasdaq Composite about 5.3%; the S&P 500 closed more than 10%
below its record of 2026-10-06, its seventh loss in a row; NVDA fell about 14%. The summaries name doubts about the
return of AI spending, credit conditions and rate fears.

Method (first plan of the Head of Fundamental Research):
1. One stock (NVDA): the value engine at the two dates with the price, the curve and the facts of each date. The
   change of the price = the part of the risk-free curve + the part of the implied premium + the part of the implied
   growth (and the part of new facts, if a filing came between the two dates). Base: `fa dcf implied` (value at the
   means; the search is deterministic). Needs: the price and the curve of a past date (the command uses the last
   close now).
2. The same for the S&P 500 and the next 500 lists, by sector and size: which sectors lost value, and is the loss
   a change of the discount (premium, rates) or of the growth that the price implies.
3. Findings file (skill `investigation-report`), audited.
Rule: a market-implied value is a measurement, not an input (`docs/QUALITY-STANDARDS.md`, section 6).
Needs data of the days of the drop: prices to 2026-11-20 and the GSW curve to the newest date (request to the Head
of Technology, 2026-11-20).

Branch note, 2026-11-21: this branch starts from `claude/dcf-market-implied` (38b6460, in review), because it needs `fa dcf implied`. After that pull request merges, `main` is merged here and only the attribution diff stays.

Change note (review cycle 1): `fa dcf implied --as-of` now filters the facts by filing date (before, `--as-of` set only the curve, the beta and the stale warning). `fa dcf auto` and `fa dcf screen` are unchanged.
