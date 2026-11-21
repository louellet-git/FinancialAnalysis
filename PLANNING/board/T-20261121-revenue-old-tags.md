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

Result (2026-11-21, store e06070c, sp500). Six tags appended to `revenue_from_contracts`
after `SalesRevenueNet`: SalesRevenueGoodsNet, SalesRevenueServicesNet, RefiningAndMarketingRevenue,
UtilityRevenue, HealthCareOrganizationRevenue, RealEstateRevenueNet. Tickers without revenue: 2014
41 -> 6, 2015 49 -> 7 (of 445 with a 10-K), 2016 17 -> 2. No existing value changed; 334 values are
new. Still without 2015: CMG, EQT, FANG, KKR, HCA, TRGP, REG (only part tags). mapcheck panel PASS
69 -> 82, WARN 432 -> 419, FAIL 0. `fa dcf pit`: 9745 -> 10833 company-years; mean errors -1.6/-1.5/-1.3%
-> -1.8/-1.8/-1.7%. IBM 83.84 / 151.84 / 258.03. Trap: docs/traps/2026-11-21-revenue-excise-before-2018.md.
Open: HWM gross_profit 2014 is -6.6B (revenue 12.5B, cogs_total 19.1B from a different basis): a
cogs_total defect, for the data audit.
