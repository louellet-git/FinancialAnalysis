---
id: T-20261121-revenue-old-tags
title: Map the old revenue tags (fiscal years before about 2017)
area: importer
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: technology
order: 2
branch: claude/revenue-old-tags
created: 2026-11-21
acceptance: The mapped `revenue` has a value for fiscal year 2015 for the sp500 tickers that have a revenue tag in that year; the count before and after; no change for a ticker that already had a value; IBM 83.84 / 151.84 / 258.03; a fixed-answer test.
---
Found by the Head of Fundamental Research (2026-11-21): `revenue` has no value in fiscal year 2015
for 51 sp500 tickers (for example JNJ, KR, VLO, SYY, PFE, MRK, KO). The raw tags exist (for example
SalesRevenueGoodsNet, RefiningAndMarketingRevenue). Philip Morris revenue of that year includes
excise taxes. This limits each study with a 10-year history and the `fa dcf pit` base years.
