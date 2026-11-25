# The AI Build-Out: Money and Limits

*Why Anthropic went public and arranged debt, and the reasons that are not about investing that could slow AI down.*

**Disclosure.** The agents that wrote this report run on Anthropic models, and Anthropic makes Claude. This report is not a valuation and gives no trade view of the stock. The company numbers come only from Anthropic's own prospectus (form 424B4). A statement of the company is named as such.

Anthropic priced its shares on 2026-11-18 (prospectus, cover). Its revenue grows very fast. But its commitments to buy computing power are much larger than its revenue and its cash.

## The answer

1. **Why go public, when revenue grows?** The company has commitments of about $900.9 billion, mostly for compute and infrastructure, over many years (p. 177 to 178). That is about 4 times its cash and investments after the IPO and the private placement ($232.4 billion pro forma, p. 168). $70.7 billion of the commitments are due within 12 months. Revenue in Q3 2026 was $17.0 billion (p. 164). So the build-out costs more than the cash that the business makes now. The prospectus gives the use of the IPO money as "general corporate purposes", including "investments in compute and infrastructure", with "broad discretion" (p. 128). The filing does not show whether the IPO makes the company less dependent on its suppliers, which are also its investors.
2. **The debt.** On 2026-09-30 the company owed $6.2 billion of Amazon convertible notes. It had $92.0 billion of facilities that it had not drawn ($77.0 billion of note agreements, p. 172, and the $15.0 billion bank facility). After the private placement this is $82.0 billion (our sum: Broadcom $42.0B, Amazon $15.0B, Google $10.0B, bank $15.0B). The $900.9 billion of commitments are not borrowed money, but they are obligations to pay. The largest facility, with Broadcom, is up to $42.0 billion of notes. It can only pay the rent of Broadcom equipment. Its interest is 9% a year, paid in more notes, or 13% if the revenue conditions fail, and the notes convert to shares at a discount of 10% to 15% (p. 172 to 174).
3. **Can AI slow down for reasons that are not about money?** Yes. Real physical limits exist. Power and the grid are the tightest now. Chip memory and packaging are sold out but are expanding. Data and the returns of scaling are disputed. The evidence today is consistent with a build-out that the supply of power and chips limits. It is not evidence that the progress of the technology stops.

## Part A: the money

**Table 1: The company in numbers** (Anthropic's prospectus, 424B4, 2026-11-18). See that the business now makes operating cash, but that the commitments are much larger than the cash.

| Item | Value | Page |
|---|---|---|
| Revenue 2024 / 2025 / first 9 months of 2026 | $0.4B / $4.6B / $33.2B | p. 27 |
| Operating loss and operating cash flow, first 9 months of 2026 | -$1.3B and +$6.6B | p. 27, p. 176 |
| Net loss, first 9 months of 2026 | -$104.0B (of which -$94.3B is a fair-value change of notes and derivatives, not cash) | p. 27 |
| Cash and short-term investments, 2026-09-30 | $131.8B ($232.4B pro forma) | p. 28, p. 168 |
| Commitments, mostly compute and infrastructure | about $900.9B ($70.7B within 12 months; $186.2B with termination rights or conditions) | p. 177 to 178 |
| IPO and private placement, net | about $104.8B | cover, p. 128 |

**Table 2: The debt items** (Anthropic's prospectus). See that only the Amazon notes are drawn.

| Item | Size | Drawn on 2026-09-30 | Terms | Page |
|---|---|---|---|---|
| Broadcom note purchase agreement | up to $42.0B | $0 (draw period not started) | only for equipment rent; 9% paid in notes (13% if revenue conditions fail) | p. 172 to 174 |
| Bank revolving facility | $15.0B | $0 ($205M letters of credit) | senior secured; to 2031 | p. 170 |
| Amazon convertible notes | $6.2B | $6.2B | no interest; due 2034 to 2035; expected to settle in shares | p. 172 |
| Amazon note agreement | $15.0B | $0 | undrawn | p. 172 |
| Google note agreement | up to $20.0B, reduced to $10.0B after the private placement | $0 | undrawn | p. 172 |

**Other AI builders.** Debt now pays a larger part of AI spending at the large cloud companies. New debt was 9% of their capital spending in FY2024 and 32% in the 12 months to June 2026 (FactSet, "Hyperscalers tap external financing as AI capex outruns cash flow", 2026-07-23, insight.factset.com). S&P Global Ratings expects more than $1.3 trillion of AI spending by 2027 for six large companies, with negative free operating cash flow in 2026 and 2027 (press release, 2026-08-27, press.spglobal.com).

## Part B: can AI slow down?

**Table 3: The limits other than money.** See the last column: each limit has a number that would show a slow-down first. "Secondary" marks a press report of a primary source that we did not read.

| Limit | For a limit | Against | Sign to watch |
|---|---|---|---|
| Power | Data centers use 485 TWh in 2025 and about 950 TWh in 2030 (IEA, "Energy and AI", iea.org, accessed 2026-11-23). The median US grid wait is 61 months, against 36 in 2015 (LBNL, *Queued Up*, June 2026, emp.lbl.gov) | Data centers stay at about 3% of world power in 2030 (IEA) | Utilities cut large-load forecasts |
| Chips | HBM memory is "effectively sold out" through 2029 (SK hynix Q3 2026 call, via MT, 2026-10-28, secondary) | The packaging shortage falls from about 20% to 10% by the end of 2026 (TrendForce, 2026-06-15, secondary) | HBM or packaging prices fall |
| Build time | 30% to 50% of 2026 projects may slip (Sightline and Currence, via Network World, 2026, secondary) | SemiAnalysis disputes the 50% figure (not checked) | US capacity under construction falls again (6.35 to 5.99 GW in 2025, CBRE via Bloomberg, 2026-02-25, secondary) |
| Scaling and data | Public text may be used up between 2026 and 2032 (Epoch AI, arXiv 2211.04325, 2024) | The task length that models can do doubles about every 7 months (METR, metr.org, 2026-01-29) | The METR doubling time gets longer |
| Physics | Compute grows 3.0x per 2 years, memory bandwidth only 1.6x (arXiv 2403.14123, March 2024) | The price of a fixed model quality falls about 40x a year (Epoch AI, epoch.ai, accessed 2026-11-23) | Gains in performance per watt stall |

**Precedents.** Earlier build-outs slowed when use fell behind capacity, not when the technology stopped. British railway shares fell 64% from 1845 to 1849 (A. Odlyzko, "The Railway Mania", umn.edu). In 2001 only 7% to 10% of new US fiber was in use (Atlantic-ACM, via Channel Futures, 2001-12-01, secondary). In each case, the first sign was low use of the new capacity and falling prices.

**What this means.** The physical limits slow the pace of the build-out. No source in this report shows that they stop it. History shows a different risk: a build-out that debt pays for can slow fast if revenue does not follow capacity. For this company, the numbers to watch are the revenue of each quarter against the commitments that fall due, and the part of the Broadcom, Amazon and Google facilities that it draws.

---
Sources: the prospectus numbers with their pages are in `analysis/priors/ant_filing_numbers_2026-11-23.md` (Head of Fundamental Research). Each Part B source is in its table row.
