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

Result (2026-11-21, store 9059672, after the review of cycle 1). Six pre-2018 tags are in a new variable
`revenue_total_pre2018` (SalesRevenueGoodsNet, SalesRevenueServicesNet, RefiningAndMarketingRevenue,
UtilityRevenue, HealthCareOrganizationRevenue, RealEstateRevenueNet). `revenue_from_contracts` keeps its
three tags. The `revenue` formula reads the new variable after `revenue_from_contracts`: it fills an empty
period only. Fixed-answer tests for each of the six tags.

Counts (mapcheck, sp500.txt, 10-K values of `revenue`, FY = year of the period end):
- Default 12 periods, annual only (5,776 10-K periods): with a value 5,662 -> 5,755 (+93). 0 values changed, 0 lost.
  The quarters (5,997 periods) do not change.
- All years (--periods 40): annual 8,084 -> 8,441 (+357) of 8,528 periods; quarterly 18,303 -> 18,820 (+517).
  0 values changed in the sp500 list.
- Whole store (2,499 tickers, --periods 40, 28,050 annual periods): +1,230 annual values, +1,950 quarterly values,
  0 lost, and 1 changed value: UMH FY2010 2,817,059 -> 34,010,964 (the base took investment_income_operating;
  the RealEstateRevenueNet value is right).
- Tickers with no revenue (sp500, annual, of the tickers with a 10-K in the year): FY2014 41 -> 6 (of 349),
  FY2015 49 -> 7 (of 445), FY2016 17 -> 2 (of 459). Still without FY2015: CMG, EQT, FANG, KKR, HCA, TRGP, REG.
- mapcheck panel (default 12 periods): PASS 69 -> 82, WARN 432 -> 419, FAIL 0.

Scope check: where both exist (sp500, 40 periods, revenue from another tag), `revenue_total_pre2018` is below
98% of `revenue` in 2,088 of 2,696 periods (113 tickers): goods or services tags that are a part where a total
tag exists. These periods never use it. In the 3,180 store-wide periods that it fills (1,863 with cogs_total),
revenue is below cogs_total in 12 periods (0.6%).

Backtest (`fa dcf pit`, IN SAMPLE: the parameter table is estimated on all years; one run, seed 7). The median
error of revenue (T+1/T+2/T+3) goes from -1.6/-1.5/-1.3% (base) to -1.8/-1.8/-1.7% (new), only because of the
changed companies. The 1,057 unchanged companies have the same errors on base and new (-1.7/-1.7/-1.4%). The 108
changed sp500/next500 companies go from +0.5/+1.4/+0.6% to -1.3/-1.1/-1.4% (407 -> 981 company-years); 101 other
companies go from -0.1/-1.4/-1.9% to -2.9/-4.1/-5.3% (299 -> 813). The run after the scope change gives the same
10,833 company-years and the same errors (-1.8/-1.8/-1.7%). The number is not evidence of a better or worse model.

Gross profit guard (mapper): no `gross_profit` from `revenue - cogs_total` when the filing of the one term has a
different value for the other (a restatement: two bases). A negative result from one basis stays (a cruise line in
2020-21, CPT, GIS can be real). HWM 2014 (-6.595B: revenue from the filing of 2017, cogs_total from the filing of
2015) has no value now. Test: `gross_profit_needs_one_basis`. sp500, --periods 40: 93 values removed (default 12
periods: 42); 133 values are new, because revenue now exists. Operating income (formula with gross profit): 1 value
lost, OXY 2014 Q2 (1.407B). It was a mixed basis: revenue 5.133B (restated in the filing of 2015-08-04, after the
spin-off of California Resources) less cogs 3.297B and opex 0.429B (the original filing of 2014-08-05 gave revenue
6.275B and opex 0.498B, which would give 2.48B). So 1.407B was wrong. A different accession number alone is not a
rule: it removes 241 good values, so the rule needs a different value in the other filing.

IBM 83.84 / 151.84 / 258.03. Trap: docs/traps/2026-11-21-revenue-excise-before-2018.md (BF-B, STZ, TAP, PM).
