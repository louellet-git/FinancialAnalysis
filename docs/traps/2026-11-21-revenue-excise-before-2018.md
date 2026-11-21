# Revenue before 2018 can include excise taxes

Date: 2026-11-21. Ticket: `T-20261121-revenue-old-tags`.

Before ASC 606 (2018), some filers gave revenue with excise taxes in it, and later revenue without
them. The mapped `revenue` then has a break at the tag change:

- PM and MO: `SalesRevenueNet` includes excise (PM 73.9B in 2015, 26.7B in 2016 from the contract tag).
- BF-B and STZ: `SalesRevenueGoodsNet` includes excise (BF-B 4.10B in FY2015, 2.99B in FY2017).
- VLO: `RefiningAndMarketingRevenue` includes excise.

No tag for revenue net of excise exists in the store for these filers. A growth rate, a base year or a
margin across the break is wrong. Use the years on one side of the break only, or remove the excise
amount by hand from the 10-K text.
