# GLD: Why It Is Where It Is

*Why the SPDR Gold Shares fund (GLD) closed at $322.87 on 2026-11-20, explained with the path of the last 3 years.*

GLD holds physical gold, so it follows the gold price, less a fee of 0.40% a year (spdrgoldshares.com). It closed at $322.87 on 2026-11-20: 35% below its closing high of $495.90 (2026-01-29) and 14% below one year before, but 76% above three years before.

## The answer

1. **The rise (2023 to January 2026):** the data fits a rise that buyers who do not look at yields drove. Central banks bought more than 1,000 tonnes (t) a year in 2023 and 2024 (World Gold Council, secondary summary). Gold ETFs added 801 t in 2025 (World Gold Council, secondary summary). From 2023-11-20 to 2024-12-31, GLD rose 32% while the 10-year real yield also rose, from 2.12% to 2.24%. In 2025, gold often rose in weeks when the real yield rose.
2. **The fall (since January 2026):** the market now expects Fed rate increases. The 10-year real yield rose from 1.90% (2026-01-28) to 2.77% (2026-11-20). In 2026, gold again falls in most weeks when the real yield rises.
3. **The last week:** GLD fell 9.2%, about as much as the stock market (SPY -8.6%). Markets priced about a 90% chance of a Fed increase in December (Kitco, 2026-11-19, secondary). UUP (the dollar fund) rose 0.4%.

The data is consistent with this reading. It does not prove it.

**Table 1: The 3 years in four periods** (GLD closes from our data; reasons from market sources at the time). See that the reason changed from rate cuts and central-bank buying to rate increases.

| Period | GLD | Reason given at the time (source) |
|---|---|---|
| Nov 2023 to Oct 2024 | $183.37 to $257.50 | expected Fed cuts; the US election (FXStreet, 2024-10-30, secondary); central-bank buying of more than 1,000 t a year (WGC, Gold Demand Trends 2024, secondary summary) |
| Nov 2024 to Oct 2025 | to $403.15 (2025-10-20, the day of a gold record) | central banks and record ETF inflows, 801 t in 2025 (WGC, secondary summary); US-China tension |
| Oct 2025 to Jan 2026 | to $495.90 (2026-01-29) | the all-time high of gold, $5,589/oz on 2026-01-28 (StoneX, secondary); then the gold price fell 9.5% in 2 days as the dollar rose on the Fed-chair nomination (Morningstar, secondary); GLD fell 13.9% in 2 days |
| Jan 2026 to Nov 2026 | down to $322.87 | Fed increase bets after the oil rise of the Iran war (Kitco, 2026-03-20, secondary); a nominal 10-year yield near 5% (4.92% on 2026-11-20, Treasury); margin calls when tech stocks fell (investingLive, 2026-11-02, secondary) |

**Table 2: The tests** (weekly log returns from our data; a week ends on the last day of each ISO week on which all series have a value; 2024 runs from 2023-11-17; a year has only the weekly returns that start and end in it (n = 58 / 51 / 46); real yield from the Treasury real curve). See the sign of the real-yield link in each year.

| Test | Result | Reading |
|---|---|---|
| Weekly correlation of GLD with the change of the 10-year real yield: 2024 / 2025 / 2026 | -0.25 / +0.29 / -0.26 | negative in 2024, a weak positive correlation in 2025, negative again in 2026 |
| Weekly correlation of GLD with the dollar fund (UUP): 2024 / 2025 / 2026 | -0.19 / -0.45 / -0.33 | a stronger dollar lowers gold, as usual |
| Weekly correlation of GLD with SPY: 2024 / 2025 / 2026 | 0.19 / -0.07 / 0.46 | in 2026 gold moves more with stocks, so it protects less |
| Week 2026-11-13 to 11-20 | GLD -9.2%, gold miners (GDX) -16.2%, SPY -8.6%, dollar +0.4% | gold fell with the market; it did not protect |

A metals analyst says the same: the correction "restored" the inverse link with real yields (Heraeus, via Kitco, 2026-11-18, secondary).

## What the level implies

Gold has no cash flows, so our valuation engine gives no value for GLD. Central banks bought 675 t in the first 9 months of 2026, 6% more than one year before (World Gold Council, 2026-11-03). Gold ETFs held a record 4,275 t at the end of October (World Gold Council, 2026-11-05, secondary summary). The data fits a level that this buying supports and that the expected rate increases lower. Two numbers would show a change in this reading first: the Fed decision of 2026-12-09 (federalreserve.gov), and the monthly central-bank and ETF data of the World Gold Council.

**Limits.** The page has five data and method gaps. Page 2 lists them, with the work that removes each one.

---
Status: GLD, UUP, GDX and SPY daily closes from the store (fa-data 2d4619d, Nasdaq site); real yields from the Treasury real curve (2023 to 2026). Each other fact has its source in the table or the text.

<div style="page-break-before: always"></div>

## Limits and the work to remove them

**Table 3: The limits of the GLD page.** See the column "What removes it". The owners are proposed; no limit has a board ticket yet.

| Limit | What it does to the answer | What removes it | Who | When |
|---|---|---|---|---|
| (a) The gold price is not in our data; GLD stands in for it | The fee and any premium or discount of GLD are not separated | A gold price series (for example the LBMA price), with its terms | Technology (proposed) | no date yet |
| (b) Central-bank and ETF tonnes come from World Gold Council summaries, several read only as search summaries | The demand side is not tested on our data | The WGC data files, after a check of their terms | Technology (proposed) | no date yet |
| (c) GLD tonnes held now and earlier come from press reports | The fund's own flow is not measured | The daily holdings file of the fund, with its terms | Technology (proposed) | no date yet |
| (d) The correlations come from a one-time script | Not repeatable for a different fund | A command to be built (URA page, limit g) | Economics (proposed) | no date yet; not before 2026-11-25 |
| (e) A correlation shows that two prices move together, not why | "The link with real yields is back" stays a reading | A test on news days (Fed days, central-bank data days) | Economics (proposed) | with (d) |
