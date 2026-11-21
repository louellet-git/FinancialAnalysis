# Is a rate rise expected, and what moved the 10-year? (2026-11-21)

Question (Luc): why did the return that investors ask for rise, and do people expect a rate rise? Data is the store copy in `data/raw/rates/`. No network call. All yields are in percent (par, bond-equivalent). A change is in basis points (bp).

## Main answer

1. Yes, the bills show an expected rise, about two quarter-point steps over the next year. This is a rough reading, not a Fed forecast. The 70-80% for December is not ours and is not in the store.
2. The 10-year rise in the week to 11-13 was a real yield rise (+28 of +29 bp). Inflation compensation did not move.
3. "Late reaction of stocks to rates" is one fair reading. It does not fit all the data: yields fell in the second week while stocks fell most.

## 1. Rate-rise expectation

Facts. Source: `treasury_bill_rates_2026.csv` (coupon equivalent), `treasury_nominal_2026.csv`. Fed funds (EFFR) 3.88%, target 3.75 to 4.00 [11-19]: earlier brief, `fa rates nyfed`, not re-run.

| Date | 1M bill | 3M | 6M | 1Y | 2Y |
|---|---|---|---|---|---|
| 11-06 | 3.95 | 4.11 | 4.26 | 4.32 | 4.51 |
| 11-13 | 4.02 | 4.23 | 4.47 | 4.63 | 4.88 |
| 11-20 | 4.11 | 4.23 | 4.41 | 4.50 | 4.63 |

Method. The forward rate between two bills is about 2 x (longer yield) - (shorter yield), for a 6-month step (3M to 6M, 6M to 1Y). This approximation treats the coupon-equivalent yields as semiannual bond yields. With simple money-market interest, the forwards are 5 to 9 bp lower (auditor, 2026-11-21); this is inside the plus or minus 25 bp of the limits. On 11-20: 3M to 6M = 2 x 4.41 - 4.23 = 4.59; 6M to 1Y = 2 x 4.50 - 4.41 = 4.59. On 11-13 the same forwards were 4.71 and 4.79. On 11-06: 4.41 and 4.38.

Reading. The 1M bill (4.11) is 23 bp above EFFR (3.88). If this gap stays, a 4.59 forward means a policy rate near 4.36, about 50 bp above EFFR: two steps of 25 bp, from 3 to 12 months ahead. A step in December is not separate in the bills. The 3M bill gives a small signal: 3M minus 1M is +12 bp.

Limits. A bill yield is not a policy forecast. It has a term premium, bill supply, and a spread to EFFR that moves. The gap is large here, so the size is uncertain by plus or minus 25 bp or more. The 2-year (4.63) is 75 bp above EFFR, which fits a rise, or a high term premium. Fed funds futures and SOFR futures are not in the store. The earlier brief quoted an Atlanta Fed SOFR-option path (4.29% for the window of 12-16, price of the market); I did not re-check it.

Dates (official pages, read 2026-11-21). The next FOMC meeting is December 8-9, 2026, with a Summary of Economic Projections; no meeting is in November (federalreserve.gov/monetarypolicy/fomccalendars.htm). The October CPI came out on November 10, 2026, and the November CPI comes out on December 10, 2026, one day after the FOMC decision (bls.gov/schedule/news_release/cpi.htm). The store holds the October CPI-U index (`bls_cpi.json`): 337.511 (not seasonally adjusted), up 0.17% from September (336.940). Core (`CUUR0000SA0L1E`, 339.921) is up 0.19%. The 12-month rate for October is not in the store (the October 2025 value is missing). September was 3.74% (earlier brief). So "high October CPI" is not visible in these index changes. A seasonally adjusted value is not in the store.

## 2. Real yield or inflation compensation (10-year)

Source: `treasury_nominal_2026.csv`, `treasury_real_2026.csv`. Breakeven = nominal 10Y minus real 10Y, same day.

| Date | Nominal | Real | Breakeven |
|---|---|---|---|
| 11-06 | 4.86 | 2.63 | 2.23 |
| 11-13 | 5.15 | 2.91 | 2.24 |
| 11-20 | 4.92 | 2.77 | 2.15 |

Week to 11-13: nominal +29 bp, real +28 bp, breakeven +1 bp. Week to 11-20: nominal -23, real -14, breakeven -9.

Reading. A real-rate rise raises the discount rate for stocks directly, and the 2-year rose 37 bp too, so the rise came with short rates. A breakeven rise has a less clear effect: it can mean growth or it can mean a loss of inflation control. Here it did not move, so the first week is a real-rate story. The week to 11-20 is mixed: real yield down 14 bp, breakeven down 9 bp.

## 3. "Late reaction of stocks to rates"

Facts. Yields fell in week two (10Y -23 bp, 2Y -25 bp; store files above). These values come from the bridge of Fundamental Research (`analysis/priors/sp100_week_prelim_results_2026-11-21.md`, section 6, branch claude/attribution-reality), not checked by Economics: S&P 100 stocks +2.43% in week one and -9.22% in week two; VIX 19.3 to 28.4; Baa minus 10Y 1.47 to 1.52. Our earlier brief gives VIX 15.01 to 28.44 and Baa minus 10Y 1.46 to 1.52 [11-18] over the whole window. The readings below are only as good as these values.

Reading A (late reaction): the rise of the short and middle yields in week one (1Y bill +31 bp, 2Y +37 bp) lowers value with a delay.
Reading B (risk-off shock): VIX up and yields down together is a flight to safety. It is not a rate shock. The credit spread rose only 5 bp, which is small for a shock of this size.
Reading C: a company or sector event (for example capex news) drives a heavy index. The earlier brief on capex-led falls covers this.

The facts favor A less in week two: the rate pressure eased while stocks fell. The data cannot choose between B and C. These data would separate them: daily returns by sector and by stock (is the fall in few names?); daily yields against daily stock returns (does the fall follow a yield day?); the daily VIX, the OAS by rating, and the dollar; news by date. I do not claim a cause.

Not done: no fed funds futures, no term-premium model (ACM), no check of the 70-80% claim.
