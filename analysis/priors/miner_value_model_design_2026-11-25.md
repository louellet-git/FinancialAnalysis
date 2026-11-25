# Miner Value Model: Design

*A net asset value model for a mining company: the output, the blocks, the inputs and the tests. Nothing is built before your review.*

Background: our engine values a company from the cash flows of its last filings. This does not fit a miner: Uranium
Energy (UEC) gets a value of -14.16 a share at its close of 5.30 on 2026-11-20. The value of a miner is in its
reserves, its costs and the price of its commodity. This is limit (b) of the URA page.

Proposal: value each mine as the cash flow of its mine plan on random paths of the commodity price. **The main output
answers "why is the price at this level": the commodity price that the stock price needs.** It is a measurement, and it
never becomes an input. A miner can trade away from its value for years, so the page will not call a gap between value
and price an error of the market. Input rule: a dated spot price of the commodity is permitted; a stock price, a
forward curve, a price forecast of analysts and a price-to-value ratio are not.

**Table 1: The value of a miner in blocks** (a value for each share; the blocks add to the total; each block after tax).
See: the first block is the large one for a producer; for a developer it is the same block, less the build cost.

| Block | How it is calculated |
|---|---|
| + Mines with a plan | For each year of the mine life: pounds sold at the price (contracted pounds at their contract terms), less the cash cost, the sustaining and growth capital, the royalty and the closure cost of the plan; discounted. Not yet built: less the build cost, times the chance of the permit |
| + Resources outside a plan | Measured and indicated pounds, times a conversion share, times the margin of the nearest mine after tax and capital, with a delay. Inferred pounds: zero |
| + Other assets | Uranium held, at the price; a stake in a different company at its own value (Cameco: Westinghouse) |
| - Debt and costs | Net debt (debt less cash); the closure liability of sites with no plan; the corporate costs for the years of the longest mine life; the shares of minority and joint-venture partners |
| **= Value for each share** | A range (P10, P50, P90) from 10,000 price paths: the price starts at the dated spot price and goes to a long-run level |

**Table 2: The inputs and where they come from.**
See: the reserve data are text in filed reports, not data fields; reading them is the large part of the work.

| Input | Source | In our data today |
|---|---|---|
| Pounds by mine and class; mine life; yearly output; costs; capital; tax and royalty | The last technical report filed before the date. US filer: SEC Regulation S-K 1300, with the 10-K. Canadian filer: rule NI 43-101, with the 40-F | No |
| Commodity price, spot | Technology: IMF monthly series, used by its release date | Month-end points only |
| Debt, cash, closure liability, shares | The SEC data. For Cameco, NexGen and Denison: IFRS data (Technology) | US filers only |
| Discount rate | The rate model of the engine; the beta from data before the test window, the same beta in T3 | Yes |
| Conversion share; chance and date of a permit; long-run price level | A judgment of this head: a dated record for each, with its reason. For T3, each judgment uses only documents dated on or before 2023-12-31 (for example the long-run price of the technical report of that time) | New records |

**Table 3: The tests, with the pass rule written before the first run.**
See: T1, T2 and T4 judge the engine. T3 judges the price sensitivity of the mine plans, not the level of the value.

| Test | Pass rule |
|---|---|
| T1 Replication | On one path with the price, the rate and the tax of the technical report, the model gives the after-tax value of that report for each mine, within 2%. The gap with our own price path is a measurement |
| T2 Point in time | The value at a date does not change when all data filed or released after the date are removed or made false |
| T3 Price sensitivity, on a hold-out | Each month. Benchmark: beta times the market, plus the raw commodity return with a fitted loading. The change of the model value from the commodity price must lower the out-of-sample error of the stock return against the benchmark (Clark-West test, 10% level), pooled over 5 names: the 5 largest miners with a technical report in the URA holdings of the form N-PORT nearest to 2023-12-31 (expected: CCJ, NXE, UEC, DNN, UUUU). Fit 2017-01 to 2023-12; hold-out 2024-01 to 2026-10, scored one time. One commodity: about one trial; a pass is "consistent with", not proof |
| T4 Sum | The blocks add to the total. A check shows that no input reads a stock price |

**Size of the work** (estimates): the first value of one company with T1, T2 and T4 (the code, one analyst agent, one gate) is
about 6% to 8% of the weekly token limit. T3 needs 4 more names and a monthly price series: about 3 to 4 more weeks.
Stop point: a T1 failure after one correction of the typed inputs.

**Decisions for you**: (1) build, or stop here; (2) the first company: UEC (a US filer; its SEC data and prices are in the
store) if its technical reports have a mine plan with costs (not read yet), else NexGen (one project with a full study).
Cameco (22.2% of URA) needs the IFRS data and the Westinghouse stake: later; (3) accept or change the pass rules.

Status: a design only; no code, no run and no value exist. Not covered: reactor developers, the uranium trust, Kazatomprom.
