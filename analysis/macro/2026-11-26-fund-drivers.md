# URA and GLD: What Moves Them

*Which outside drivers the monthly returns of URA (three parts) and GLD are consistent with, November 2023 to October 2026. Layer 3 of 3.*

The first two pages showed what URA and GLD hold. This page asks why those holdings moved. The test was fixed before we read the data. The answer: each of the four parts is consistent with its drivers, but the drivers explain only 12% to 43% of the moves. The rest is not explained.

## The answer

1. **URA miners are consistent with the uranium price and the stock market.** The drivers explain 35% of the monthly moves. Uranium is the clearest driver (coefficient 0.94, t 2.10, a wide range).
2. **URA reactor and fuel companies are consistent with the stock market, not with uranium.** The drivers explain 25%. The market and the chip stocks give most of it; the uranium link is not clear (t 0.65).
3. **The foreign part of URA is consistent with the uranium price.** The drivers explain 43% (12 quarters; 49% without the one doubtful holding, Daewoo E&C).
4. **GLD is only weakly consistent with real yields and the dollar.** The two drivers together explain 12%. Neither one is clear alone.

**What this means for you.** For the miners and the foreign part, the uranium price is the clearest link. For the reactor names, the stock market is a clearer link than uranium. For gold, real yields and the dollar explain little in these 3 years: 88% of the monthly moves of GLD are not explained by this test.

**Table 1: The four tests** (monthly log returns; URA-F quarterly). See the column "Not explained": the drivers leave most of each move unexplained.

| Part | Drivers | Explained (R²) | Not explained | Out of sample (last 12 months) | Placebo p | Words (fixed rule) |
|---|---|---|---|---|---|---|
| URA miners (9 names) | uranium, market, chips | 35% | 65% | R² 0.28 | 0.04 | consistent with the drivers |
| URA reactors and fuel (7 names) | uranium, market, chips | 25% | 75% | R² 0.37 | 0.08 | consistent with the drivers |
| URA foreign part (quarterly) | uranium | 43% | 57% | no check (12 quarters) | 0.10 | consistent with the drivers |
| GLD | real yield, dollar | 12% | 88% | R² 0.09 | 0.08 | consistent with the drivers |

"Consistent" needs a positive out-of-sample R² and a placebo p below 0.10. For URA-F, 9 shifts make 0.10 the smallest possible p, so its rule is 0.10 or less, and its p is at that floor. With four tests at 10%, one false "consistent" is possible.

**Table 2: What each driver adds** (coefficient, t value in brackets). See that uranium is clear for the miners and the foreign part, and the market for the reactor names.

| Part | Uranium | Market and chips together | Real yield | Dollar |
|---|---|---|---|---|
| URA miners | 0.94 (2.10) | adds 11% of R² | | |
| URA reactors and fuel | 0.47 (0.65) | adds 19% of R² | | |
| URA foreign part | 0.76 (4.53) | | | |
| GLD | | | -0.05 (-1.26) | -0.51 (-1.41) |

The intercept (the part of the return with no driver, not an explanation) is -62 pp for the miners, -144 pp for the reactor names, +32 pp for the foreign part and +65 pp for GLD over the sample.

**The last 12 months (Nov 2025 to Oct 2026).** Each part did worse than the drivers predicted. The sums of the monthly errors (actual less predicted) are -80 pp for the miners (return -39 pp), -217 pp for the reactor names (return -160 pp) and -31 pp for GLD (return -3 pp).

---
Status: `fa macro fund-drivers` (run of record 2026-11-26; design and its records R1 to R5 in analysis/macro/2026-11-25-fund-drivers-design.md). Source of the uranium price: International Monetary Fund, Primary Commodity Prices. Gold: World Bank Commodity Price Data (CC BY 4.0).

<div style="page-break-before: always"></div>

## Description and limits

**Table 3: The coefficients in each year** (description only, no test; the market and chips are read together). See that the links change from year to year.

| Part, coefficient | 2024 | 2025 | 2026 |
|---|---|---|---|
| URA miners: intercept / uranium | 0.003 / 0.82 | 0.009 / 1.28 | -0.086 / 3.52 |
| URA miners: market / chips | 1.85 / -0.71 | -0.46 / 1.05 | 0.93 / 0.11 |
| URA reactors: intercept / uranium | 0.063 / 0.27 | -0.016 / 1.78 | -0.156 / 1.44 |
| URA reactors: market / chips | 1.76 / -0.70 | 6.59 / -0.10 | 2.46 / 0.43 |
| URA foreign part (4, 4, 3 quarters): intercept / uranium | 0.011 / 0.63 | 0.106 / 1.44 | -0.086 / 3.58 |
| GLD: intercept | 0.022 | 0.038 | 0.018 |
| GLD: real yield / dollar | -0.023 / -0.14 | -0.020 / -0.30 | -0.048 / -3.64 |

2026 has 10 months (3 quarters for the foreign part: 3 points for 2 numbers, so that value is very weak).

**GLD against the gold price.** The monthly GLD return against the World Bank gold change: correlation 0.81, slope 1.03. The World Bank price is a monthly average, so it lags a month-end return by about half a month.

**Table 4: The limits of this page.** See the column "What it does to the answer".

| Limit | What it does to the answer | What removes it | Who | When |
|---|---|---|---|---|
| (a) 36 months (12 quarters for the foreign part) | Wide uncertainty; a yearly coefficient uses 10 to 12 months (3 or 4 quarters for the foreign part) | More months | No owner (time) | each year |
| (b) The uranium and gold prices are monthly averages | They lag a month-end return by about half a month; the link with uranium is measured lower | A month-end price (licensed sources only) | Technology | not planned |
| (c) The foreign part ends one quarter earlier (2023-07 to 2026-07) | Its window differs from the monthly tests | The report of 2026-10-31 (public about end of December) | No owner (time) | next run |
| (d) Cameco, the largest holding, is in the miners group | Its fuel services and its share of Westinghouse tie the miners to the reactor theme | A split of Cameco by business (not in our data) | No owner | not planned |
| (e) The IMF can revise a value | A new run can give other numbers; the run of record is fixed by its hash (record R5) | None needed | Economics | each run |
| (f) The yearly EIA uranium prices are not yet shown as a cross-check | No second uranium source on the page | A reader of the EIA annual prices | Technology | no ticket yet |
| (g) A fit is not a cause | "Consistent with" only | A test on news days | Economics (E3.S5) | after this ticket |
| (h) The quarterly central-bank tonnes of the World Gold Council are not shown next to GLD (design rule 3.9) | The demand side of gold is not on the page | The WGC data, after a check of its terms | Technology | no ticket yet |
