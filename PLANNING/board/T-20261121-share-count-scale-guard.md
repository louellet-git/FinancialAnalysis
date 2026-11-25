---
id: T-20261121-share-count-scale-guard
title: Flag a share count that is about 1000 times its neighbors in the mapping
area: importer
status: backlog
owner: louellet-git
requested_by: louellet-git
head: technology
order: 2
branch:
created: 2026-11-21
acceptance: `mapcheck WAT --detail` flags the 10-Q filed 2026-08-11 (diluted shares 98,204,000,000 against about 59.5M in the other periods), the value is not given to the models for that period, and a fixed-answer test covers the rule.
---
Found by the attribution review of the Head of Fundamental Research (2026-11-21): WAT reports
`WeightedAverageNumberOfDilutedSharesOutstanding` = 98,204,000,000 in the 10-Q filed 2026-08-11, a scale
error of the filer. Each user of `shares_diluted` (and of the basic count, if the same error occurs)
gets a count 1000 times too large for that period.

Rule to test: compare the share count with the next period, the period before, and
`dei:EntityCommonStockSharesOutstanding` of the same filing. A ratio near 1000 (or 1/1000) is a scale
error: flag it, and drop the value or divide it by 1000 only when the cover-page count agrees. Measure the
rule on the 2,003 tickers of the mapping test before the merge (how many periods it flags).

From the data audit (2026-11-21): the same scale family in the weighted average or the cover count: MCD (S&P 100; weighted 194K, in millions), ERIE, FOX, FOXA (cover count 57 and 64 shares), WMG, SEI, NTNX, CYTK, LOAR. Order raised to 2.
