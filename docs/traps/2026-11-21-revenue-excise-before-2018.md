# Revenue before 2018 can include excise taxes: a level break in the mapped revenue

*Trap:* Some filers give revenue with excise taxes in it before ASC 606 and without them after. The mapped
`revenue` then has a level break at the year of the tag change. A growth rate, a base year or a margin across
the break is wrong. The tag `revenue_total_pre2018` (goods tag, refining tag) gives the gross value for these
filers. No tag for revenue net of excise exists for the early years. The store values below are from the
store at e06070c (all in USD).

| Ticker | Years before the break | Break |
|---|---|---|
| BF-B (fiscal year ends April) | FY2009 to FY2016: goods tag with excise (FY2016 4.011B; excise 0.922B) | New. FY2017 is 2.994B from the contract tag without excise (the goods tag, 3.857B, has excise 0.863B). Break: -25%. |
| STZ (fiscal year ends February) | FY2009 to FY2016: goods tag with excise (FY2016 7.224B) | New. FY2017 is 7.321B from the contract tag without excise (the goods tag, 8.062B, has excise 0.730B). The growth at the break reads about +1%, and the value without excise is lower than the gross value by about 9% in the earlier years. The jump from FY2013 (3.171B) to FY2014 (5.411B) is a consolidation, not excise. |
| TAP | 2011 to 2014: goods tag with excise (2014 5.928B; excise 1.781B). 2015: 3.568B, net of excise (the filing of 2018 gave the goods tag again without excise; the filings of 2016 and 2017 gave 5.127B). | New. 2016 on: the total tag `Revenues` with assessed tax (2016 6.597B; 2017 13.472B with MillerCoors). Three bases in three years: 5.93B, 3.57B, 6.60B. |
| PM | 2007 to 2015: with excise (2015 73.908B) | Old: the values are the same before and after this change. Break at FY2016 (26.685B, without excise). |

These filers have no break, and an earlier version of this file was wrong about them:

- MO: all years 2013 to 2018 come from the sales or contract tag, and they include excise
  (24.5B, 25.4B, 25.7B). No break.
- VLO: `RefiningAndMarketingRevenue` and the contract tag give the same value for 2016 (75.659B), and the
  excise (5.49B for 2016) is a separate tag in both. Both include excise. No break.

*What to do:* Use the years on one side of the break only. For a long history of BF-B, STZ and TAP, remove the
excise amount from the 10-K text by hand, or use net sales from the 10-K. Before you use a growth rate
across 2016 to 2018 for a producer of alcohol or tobacco, print `mapcheck values <TICKER>` and read the source
of `revenue` for each year. The register `docs/HIDDEN-ASSUMPTIONS.md` does not hold this trap.

*Evidence:* `mapcheck values BF-B`, `mapcheck values STZ`, `mapcheck values TAP`, `mapcheck values PM`
with `--periods 40`; the tags `ExciseAndSalesTaxes`, `SalesRevenueGoodsNet` and
`RevenueFromContractWithCustomerExcludingAssessedTax` in `<ticker>/company_facts.json.gz` of the store.
The tag list: `revenue_total_pre2018` in `financial_analysis/standard_variables.toml`.
Ticket: `T-20261121-revenue-old-tags`.
