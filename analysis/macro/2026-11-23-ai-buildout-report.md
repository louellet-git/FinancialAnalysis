# The AI Build-Out: Money and Limits

*Why Anthropic went public and arranged debt, and the reasons that are not about investing that could slow AI down.*

**Disclosure.** The agents that wrote this report run on Anthropic models. This report is not a valuation and gives no trade view of the stock. The company numbers come only from its public prospectus. A statement of the company is named as such.

Anthropic listed its shares on 2026-11-19. Its revenue grows very fast. But its commitments to buy computing power are much larger than its revenue and its cash.

## The answer

1. **Why go public, when revenue grows?** The company has committed to pay about $900.9 billion, mostly for compute and data centers, over many years. That is about 4 times its cash and investments after the IPO ($232.4 billion pro forma). $70.7 billion of it is due within 12 months. Revenue in the last quarter was $17.0 billion. So revenue pays for the running costs, but not for the build-out. The private investors gave money before. But the largest of them (Amazon, Google, chip makers) are also its suppliers, so a public market adds a source of money that is not tied to a supplier.
2. **"A lot of debt."** Most of the debt in the news is not borrowed money. It is the right to borrow, and it has a narrow purpose. The $42 billion facility with Broadcom can only pay the rent of Broadcom equipment, and nothing is drawn. The debt that the company actually owes on 2026-09-30 is $6.2 billion of Amazon notes. The company expects to pay these notes in shares.
3. **Can AI slow down for reasons that are not about money?** Yes, there are real physical limits. Power and the grid are the tightest now. Chip memory and packaging are sold out but are expanding. Data and the returns of scaling are disputed. The evidence today is consistent with a build-out that the supply of power and chips limits, not with a stop in the progress of the technology.

## Part A: the money

**Table 1: The company in numbers** (prospectus, form 424B4, 2026-11-18). See that revenue grows fast and that operating cash flow became positive, but the commitments are far larger.

| Item | Value | Page |
|---|---|---|
| Revenue 2024 / 2025 / first 9 months of 2026 | $0.4 billion / $4.6 billion / $33.2 billion | p. 27 |
| Revenue in Q3 2026 | $17.0 billion | p. 164 |
| Operating loss, first 9 months of 2026 | -$1.3 billion | p. 27 |
| Operating cash flow, first 9 months of 2026 | +$6.6 billion | p. 176 |
| Cash and short-term investments, 2026-09-30 | $131.8 billion ($232.4 billion pro forma after the IPO) | p. 28, p. 168 |
| Commitments, mostly compute and infrastructure | about $900.9 billion ($70.7 billion within 12 months; $186.2 billion with termination rights or conditions) | p. 177 to 178 |
| IPO and private placement, net | about $104.8 billion | cover, p. 128 |

The net loss of the first 9 months of 2026 was $104.0 billion. Most of it ($94.3 billion) is a change in the fair value of convertible notes and derivatives. That is an accounting change, not cash spent (p. 27).

**Table 2: The debt items** (prospectus). See that only one item is drawn.

| Item | Size | Drawn on 2026-09-30 | Purpose or terms |
|---|---|---|---|
| Broadcom note purchase agreement | up to $42.0 billion | $0 (the draw period has not started) | only to pay the quarterly rent of the Broadcom equipment lease; 9% a year, paid in notes (p. 172 to 174) |
| Bank revolving facility | $15.0 billion | $0 ($205 million of letters of credit) | senior secured; to 2031 (p. 170) |
| Amazon convertible notes | $6.2 billion | $6.2 billion | no interest; due 2034 to 2035; expected to settle in shares (p. 172) |
| Undrawn note agreements with Amazon and Google | $15.0 billion and $10.0 billion | $0 | (p. 172) |

The company says in the prospectus that it will use the IPO money for "general corporate purposes", including "investments in compute and infrastructure", and that it has "broad discretion" (p. 128).

**The same pattern at other AI builders.** Debt now pays a larger part of AI spending everywhere. For the large cloud companies, new debt was 9% of capital spending in FY2024 and 32% in the 12 months to June 2026 (FactSet, 2026-07-23). Amazon, Google, Meta, Microsoft and Oracle issued $121 billion of debt in 2025 (BNY Mellon). S&P Global Ratings expects their AI spending to pass $1.3 trillion by 2027, with negative free cash flow in 2026 and 2027 (press release, 2026-08-27). So the company is not alone: the build-out costs more than the cash that the business makes now.

## Part B: can AI slow down?

**Table 3: The limits other than money.** See the column "Sign to watch": each limit has a number that would show a slow-down first.

| Limit | Evidence for a limit | Evidence against | Sign to watch |
|---|---|---|---|
| Power and grid | Data-center use: 485 TWh in 2025, about 950 TWh in 2030 (IEA). Median wait to connect to the US grid: 61 months, against 36 in 2015 (LBNL, *Queued Up*, 2026) | Data centers stay about 3% of world power in 2030 (IEA) | Utilities cut large-load forecasts; the queue wait gets longer |
| Chips | HBM memory "effectively sold out" through 2029 (SK hynix, Q3 2026 call). Chip packaging short by about 20% (TrendForce, 2026-06-15) | The packaging gap falls to about 10% by the end of 2026 as capacity grows (TrendForce) | Prices of HBM or packaging fall |
| Build time | 30% to 50% of 2026 data-center projects may slip (Sightline, Currence, via Network World, 2026) | SemiAnalysis disputes the 50% figure | US capacity under construction falls more (it fell from 6.35 to 5.99 GW in 2025, CBRE) |
| Returns of scaling | Some papers find saturation in pre-training (arXiv 2510.06548, 2502.04066) | The length of tasks that models can do doubles in about 89 days (METR, 2026-01-29) | The METR doubling time gets longer |
| Data | Public text may be used up between 2026 and 2032 (Epoch AI, arXiv 2211.04325) | Synthetic data helps in some fields (math, code) (Epoch AI) | Labs report flat gains from more data |
| Physics | Compute grows 3.0x per 2 years, memory bandwidth only 1.6x (arXiv 2403.14123). Each new chip process gives a smaller gain (TSMC) | The price of a fixed model quality falls about 40x a year (Epoch AI) | Gains in performance per watt stall |

**Precedents.** Earlier build-outs slowed when the use fell behind the capacity, not when the technology stopped. British railway shares fell 64% from 1845 to 1849 (Odlyzko). In 2001 only 7% to 10% of the new US fiber was in use (Atlantic-ACM). The data-center operator Exodus failed in September 2001 with $4.4 billion of debt. In each case, the first sign was low use of the new capacity and falling prices.

**What this means.** The physical limits slow the pace at which the build-out can grow, but no source shows that they stop it. The risk that history shows is different: a build-out financed by debt can slow fast if the revenue does not follow the capacity. For this company, the numbers to watch are the revenue of each quarter against the commitments that fall due, and how much of the Broadcom, Amazon and Google facilities it draws.

---
Sources: the prospectus numbers with their pages are in `analysis/priors/ant_filing_numbers_2026-11-23.md` (Head of Fundamental Research). Some facts in Table 3 come from secondary press reports; the audit trail names each source.
