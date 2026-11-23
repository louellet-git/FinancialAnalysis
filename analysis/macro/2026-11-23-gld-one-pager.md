# GLD: Why It Is Where It Is

*Why the SPDR Gold Shares fund (GLD) closed at $322.87 on 2026-11-20, explained with the path of the last 3 years.*

GLD holds physical gold, so it follows the gold price, less a fee of 0.40% a year (spdrgoldshares.com). It closed at $322.87 on 2026-11-20: 35% below its high of $495.90 (2026-01-29) and 14% below one year before, but 76% above three years before.

## The answer

1. **The rise (2023 to January 2026):** buyers who do not care about yields lifted gold. Central banks bought more than 1,000 t a year in 2023 and 2024. Gold ETFs added 801 t in 2025 (World Gold Council, secondary summary). In 2025, gold rose even in weeks when the US real yield rose.
2. **The fall (since January 2026):** the market now expects Fed rate increases. The 10-year real yield rose from 1.90% (2026-01-28) to 2.77% (2026-11-20). In 2026, gold again falls when the real yield rises.
3. **The last week:** GLD fell 9.2%, about as much as the stock market (SPY -8.6%), while markets priced about a 90% chance of a Fed increase in December (Kitco, 2026-11-19, secondary). The dollar fund was flat.

The data is consistent with this reading. It does not prove it.

**Table 1: The 3 years in four periods** (GLD closes from our data; reasons from market sources at the time). See that the reason changed from rate cuts and reserve buying to rate increases.

| Period | GLD | Reason given at the time (source) |
|---|---|---|
| Nov 2023 to Oct 2024 | $183.37 to $257.50 | expected Fed cuts; the US election (FXStreet, 2024-10-30, secondary); central-bank buying of more than 1,000 t a year (WGC, Gold Demand Trends 2024, secondary summary) |
| Nov 2024 to Oct 2025 | to $403.15 (2025-10-20, the day of a gold record) | central banks and record ETF inflows, 801 t in 2025 (WGC, secondary summary); US-China tension |
| Oct 2025 to Jan 2026 | to $495.90 (2026-01-29) | the all-time high of gold, $5,589/oz on 2026-01-28 (StoneX, secondary); then -9.5% in 2 days as the dollar rose on the Fed-chair nomination (Morningstar, secondary) |
| Jan 2026 to Nov 2026 | down to $322.87 | Fed increase bets after the oil rise of the Iran war (Kitco, 2026-03-20, secondary); a 10-year yield near 5%; margin calls when tech stocks fell (investingLive, 2026-11-02, secondary) |

**Table 2: The tests** (weekly log returns from our data; real yield from the Treasury real curve). See the sign change of the real-yield link from 2025 to 2026.

| Test | Result | Reading |
|---|---|---|
| Weekly correlation of GLD with the change of the 10-year real yield: 2025 / 2026 | +0.29 / -0.26 | in 2025 gold ignored real yields; in 2026 the usual inverse link is back |
| Weekly correlation of GLD with the dollar fund (UUP): 2024 / 2025 / 2026 | -0.19 / -0.45 / -0.33 | a stronger dollar lowers gold, as usual |
| Weekly correlation of GLD with SPY: 2024 / 2025 / 2026 | 0.19 / -0.07 / 0.46 | in 2026 gold moves more with stocks, so it protects less |
| Week 2026-11-13 to 11-20 | GLD -9.2%, gold miners (GDX) -16.2%, SPY -8.6%, dollar +0.4% | gold fell with the market, not against it |

A metals analyst says the same: the correction "restored" the inverse link with real yields (Heraeus, via Kitco, 2026-11-18, secondary).

## What the level implies

Gold has no cash flows, so our valuation engine gives no value for GLD. What holds the price now is the buyers who do not look at yields. Central banks bought 675 t in the first 9 months of 2026, 6% more than one year before. Gold ETFs held a record 4,275 t at the end of October (World Gold Council, 2026-11-03 and 2026-11-05, secondary summary for part). So the level holds while that buying continues. The rate path presses it down. Two numbers would show a change in this reading first: the December Fed decision (12-09), and the monthly central-bank and ETF data of the World Gold Council.

**Limits.** The page has six data and method gaps. Page 2 lists them, with the work that removes each one.

---
Status: GLD, UUP, GDX, SPY and SMH daily closes from the store (fa-data 8f289cc, Nasdaq site); real yields from the Treasury real curve (2025 and 2026 only). Each other fact has its source in the table or the text.

<div style="page-break-before: always"></div>

## Limits and the work to remove them

**Table 3: The limits of the GLD page.** See the column "What removes it". The owners are proposed; no limit has a board ticket yet.

| Limit | What it does to the answer | What removes it | Who | When |
|---|---|---|---|---|
| (a) The real yield in the store starts in 2025 | No real-yield test for 2023 and 2024, the years when the link broke | The Treasury real curve files of 2023 and 2024 | Technology | asked for on 2026-11-23; no date yet |
| (b) The gold price is not in our data; GLD stands in for it | The fee and any premium or discount of GLD are not separated | A gold price series (for example the LBMA price), with its terms | Technology (proposed) | no date yet |
| (c) Central-bank and ETF tonnes come from World Gold Council summaries, several read only as search summaries | The demand side is not tested on our data | The WGC data files, after a check of their terms | Technology (proposed) | no date yet |
| (d) GLD tonnes held now and earlier come from press reports | The fund's own flow is not measured | The daily holdings file of the fund, with its terms | Technology (proposed) | no date yet |
| (e) The correlations come from a one-time script | Not repeatable for a different fund | The same command as for the URA page | Economics (proposed) | no date yet; not before 2026-11-25 |
| (f) A correlation shows that two prices move together, not why | "The link with real yields is back" stays a reading | A test on news days (Fed days, central-bank data days) | Economics (proposed) | with (e) |
