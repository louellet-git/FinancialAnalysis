# Miner Value Model: Design

*A net asset value model for a mining company: the value blocks, the inputs, and the tests that judge it. Nothing is built before your review.*

Background: our engine values a company from the cash flows of its last filings. This does not fit a miner: Uranium
Energy (UEC) gets a value of -14.16 a share at a price of 5.30. The value of a miner is in its reserves, its costs and
the price of its commodity. This is limit (b) of the URA page.

Proposal: value each mine as the cash flow of its mine plan on random paths of the commodity price, add the other
assets, and subtract the debt and the costs of the company. The inputs come from the reserve reports that each miner
files. An analyst types them one time into a file for each company, each value with its page. No input comes from the
price of a stock.

**Table 1: The value of a miner in blocks** (each block is a value for each share; the blocks add to the total).
See: the first block is the large one for a producer; the second is the large one for a developer.

| Block | How it is calculated |
|---|---|
| + Mines with a plan | For each mine and year of its life: pounds sold times the price, less the cash cost, the capital and the royalty, less tax; discounted. For a mine not yet built: less the build cost, times the chance of the permit |
| + Resources outside a plan | Measured and indicated pounds, times a conversion share, times the margin of the nearest mine, with a delay. Inferred pounds: zero in the base case |
| + Other assets | Uranium held, at the price; cash; a stake in a different company at its own value (Cameco: Westinghouse) |
| + or - Contracts | Sales at the terms of long-term contracts, where the company gives them (Cameco) |
| - Debt and costs | Net debt; the closure liability of the balance sheet; the corporate costs |
| **= Value for each share** | A range (P10, P50, P90) from 10,000 price paths: the price starts at the spot price and goes to a long-run level |

**Table 2: The inputs and where they come from.**
See: the reserve data are text in filed reports, not data fields; reading them is the large part of the work.

| Input | Source | In our data today |
|---|---|---|
| Pounds by mine and class; mine life; yearly output; cash cost; capital; tax and royalty | US filer: the reserve tables of the 10-K and its technical report (SEC Regulation S-K 1300). Canadian filer: the technical report under rule NI 43-101, filed with the 40-F | No |
| Commodity price, spot and long-term | Technology: IMF monthly and EIA annual series (agreed) | Month-end points only |
| Debt, cash, closure liability, shares | The SEC data. For Cameco, NexGen and Denison: IFRS data (Technology) | US filers only |
| Discount rate | The rate model of the engine (Treasury curve, beta, premium) | Yes |
| Conversion share; chance and date of a permit; long-run price level | A judgment of this head: a record for each, with its effect on the value | New records |

**Table 3: The tests, with the pass rule written before the first run.**
See: T1 and T2 judge the engine; T3 judges if the model says something true about the stock.

| Test | Pass rule |
|---|---|
| T1 Replication | With the price, the rate and the tax of the technical report, the model gives the after-tax value of that report for each mine, within 10% |
| T2 Point in time | The value at a date does not change when all data after the date are removed or made false |
| T3 Price link, on a hold-out | Each month: the change of the model value from the commodity price alone, added to beta times the market, gives a lower out-of-sample error for the stock return than beta alone (Clark-West test, 10% level), for 3 of 5 names or more. The hold-out is set before the first run and scored one time |
| T4 Sum | The blocks add to the total. A check shows that no input reads a stock price |

**What you get, and what the model does not do**

- A value range for each miner, and one new measurement: the commodity price that the stock price needs.
- Not covered: reactor developers (Oklo, NuScale), the physical uranium trust (pounds times price, in the fund
  look-through), and Kazatomprom until a source outside the SEC exists.
- A limit of T3: 5 names give low power. A pass is "consistent with", not proof.

**Decisions for you**

1. Build, or stop here.
2. The first company. This head recommends UEC: a US filer; its SEC data and prices are in the store. Cameco is the
   largest URA holding (21.6% on the URA page), but it needs the IFRS data, its contracts and the Westinghouse stake: second.
3. The pass rules of Table 3: accept them, or change them before the first run.

Status: a design only. No code, no run and no value of a company exist. Ticket: `T-20261123-miner-value-model-design`.
