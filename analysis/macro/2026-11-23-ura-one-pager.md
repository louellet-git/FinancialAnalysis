# URA: Why It Is Where It Is

*Why the Global X Uranium ETF (URA) closed at $26.79 on 2026-11-20, explained with the path of the last 3 years.*

URA holds uranium miners, a physical-uranium trust and nuclear-industry companies. It closed at $26.79 on 2026-11-20: 57% below its high of $61.81 (2026-01-28), 36% below one year before, and 8% below three years before.

## The answer

1. **The uranium price did not fall, so it does not explain the fall.** The long-term contract price is at a record, and spot is higher than one year ago. URA fell while uranium rose.
2. **The fund now moves with the AI and chip stocks.** Since 2025, the story of the market for URA has been power for AI data centers and new US reactors. The weekly correlation of URA with the chip fund SMH rose from 0.44 in 2024 to 0.69 in 2026. So when the AI stocks fell in 2026, URA fell with them.
3. **Company news added to the fall:** a mine suspension and weaker results at Cameco (21.6% of the fund), and fewer new term contracts.

The data is consistent with this reading. It does not prove it.

**Table 1: The 3 years in four periods** (URA closes from our data; uranium prices from Cameco, which quotes UxC and TradeTech). See that URA and uranium moved together until 2025 and apart in 2026.

| Period | URA | Uranium | Reason given at the time (source) |
|---|---|---|---|
| Nov 2023 to May 2024 | $29.12 to $33.21 | month-end spot $100.25/lb (Cameco); a peak near $106/lb in Jan 2024 (Kitco, secondary) | supply risk: Kazatomprom warned of acid shortages and cut guidance (Kitco, 2024-01-22, secondary) |
| May 2024 to Apr 2025 | down to $20.26 (2025-04-08) | month-end spot $64.23/lb (Cameco, 2025-03-31) | lower uranium price; DeepSeek doubt about AI power demand: URA -11% on 2025-01-27 (mining.com, secondary) |
| Apr 2025 to Jan 2026 | up to $61.81 | spot $80/lb (2025-10-31) | US orders for 400 GW of nuclear by 2050 (World Nuclear News, 2025-05-23, secondary): URA +12% that day (etf.com, secondary); AI data-center power deals (Zacks, 2026-01-29, secondary) |
| Jan 2026 to Nov 2026 | down to $26.79 | spot $91.55/lb, term $98.00/lb (2026-10-30), term a record | Cigar Lake suspension and a Cameco miss (XTB, 2026-07-07, secondary); term contracting -15% (Goldman via 24/7 Wall St, secondary); the fall of the AI stocks (Table 2, last row) |

**Table 2: The tests** (weekly log returns of the funds from our price data; month-end points for uranium). See that URA follows SMH more each year, and fell like it in the last week.

| Test | Result | Reading |
|---|---|---|
| URA against uranium, 1 year to Oct 2026 | URA -39%, spot +14%, term +15% (Cameco) | not a uranium-price fall |
| Weekly correlation of URA with SMH (chips): 2024 / 2025 / 2026 | 0.44 / 0.58 / 0.69 | URA moves more with the AI stocks each year |
| Weekly correlation of URA with Cameco, 3 years | 0.91 | the largest holding drives the fund |
| Week 2026-11-13 to 11-20 | URA -15.9%, SMH -17.8%, SPY -8.6%, Cameco -18.0%, Centrus -21.5% | URA fell like the chip stocks, about 1.8 times as much as the market |

A market report gives the same reason for the last week. Funds hold uranium stocks together with AI stocks, and funds that had to cut their positions sold them (Discovery Alert, 2026-11-22, secondary).

## What the level implies

The supply facts stay tight. Reactors need about 68,920 tonnes of uranium (tU) in 2025, against 60,213 t of mine output in 2024 (World Nuclear Association, 2025; secondary summary). Kazatomprom cut its 2026 plan by about 10% (World Nuclear News, 2025-08, secondary). So the market now prices the nuclear-for-AI story lower, not uranium itself. Two numbers would show a change in this reading first: the term uranium price, and the correlation of URA with the chip stocks.

**Limits.** The page has eight data and model gaps. Page 2 lists them, with the work that removes each one.

---
Status: URA, SMH, SPY, CCJ, UEC, NXE and LEU daily closes from the store (fa-data a798b9b, Nasdaq site). Each other fact has its source in the table or the text.

<div style="page-break-before: always"></div>

## Limits and the work to remove them

**Table 3: The limits of the URA page.** See the column "What removes it". The work is story E3.S5 on the board (PLANNING/board/EPICS.md).

| Limit | What it does to the answer | What removes it | Who | When |
|---|---|---|---|---|
| (a) The engine gives a number for one holding only (Centrus) | No "what the price implies" for most of the fund | A model for commodity producers (b) and the foreign data (c) | Fundamental Research (E3.S5: the fund look-through) | after 2026-11-25; after (b) and (c) |
| (b) The miners have no model: their value comes from reserves and the uranium price | Only the market explains the miner part of the level; no value exists | A reserve-based model: pounds in the ground, cost per pound, the term price | Fundamental Research (E3.S5: the miner model design) | after 2026-11-25 |
| (c) Foreign filers are not in our SEC data (Cameco 21.6%, NexGen, Kazatomprom) | The largest holding has prices but no facts | A source of IFRS facts (40-F filings or company reports) | Technology (E3.S5: the scout of the foreign filers) | next |
| (d) The physical-uranium trust (9.1%) is not in our data | No test of the fund against physical uranium | Daily price, NAV and pounds held of the trust | Technology (E3.S5) | needs the consent of Sprott (a decision of Luc) |
| (e) The holdings have no date and come from a secondary site | Weights can be old | The dated holdings from the SEC form N-PORT | Technology (ticket T-20261123-fund-holdings-nport) | in progress |
| (f) Uranium prices come only as month-end points from Cameco's page; several facts are secondary | No weekly test of URA against uranium; some reasons come from press reports only | A spot and term price series from the original source, with its terms | Technology (agreed) | after 2026-11-25: IMF monthly and EIA annual series |
| (g) The correlations come from a one-time script | Not repeatable for a different fund | A command that gives the period table and the correlations for any fund | Economics (E3.S5) | after 2026-11-25 |
| (h) A correlation shows that two prices move together, not why | "Moves with the AI theme" stays a reading | A test on news days (for example the AI sell-off days against uranium news days) | Economics (E3.S5) | with (g) |
