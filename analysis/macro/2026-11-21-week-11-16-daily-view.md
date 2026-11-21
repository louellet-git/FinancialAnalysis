# Daily view, week 11-16 to 11-20 (2026-11-21)

## Main answer

1. No day was a rate day. Yields fell or stayed flat on all five days while SPY fell on all five. On 11-20 the 10-year fell 12 bp and SPY fell 3.64%.
2. 11-20 was not a plain yield event. It had a sector part (chips -12.8%, about 3.5 times SPY) and a broad part (SPY -3.64%, equal weight -2.28% per Fundamental Research). Defensive names rose. Both readings below fit.
3. The store has no news, no intraday prices, no options data and no VIX for 11-20. It cannot give a cause.

## 1. Daily table

Sources: `data/raw/{SPY,NVDA,AVGO,AMD,MU,AMAT,JNJ,PG,KO}/prices.json` (close to close, %); `data/raw/rates/treasury_nominal_2026.csv`, `treasury_real_2026.csv` (par yield, change from the day before, bp). Breakeven = nominal 10Y minus real 10Y. Chip basket = equal-weight mean of NVDA, AVGO, AMD, MU, AMAT. Defensive = mean of JNJ, PG, KO. No equal-weight index file is in the store (not done by me).

| Day | SPY % | Chips % | Defensive % | 2Y bp (level) | 10Y bp (level) | Real 10Y bp (level) | Breakeven bp (level) |
|---|---|---|---|---|---|---|---|
| 11-16 | -1.91 | -5.34 | +0.34 | -5 (4.83) | -3 (5.12) | +1 (2.92) | -4 (2.20) |
| 11-17 | -1.01 | +0.13 | -0.94 | -3 (4.80) | -2 (5.10) | -3 (2.89) | +1 (2.21) |
| 11-18 | -0.31 | +1.19 | -1.23 | 0 (4.80) | -3 (5.07) | -1 (2.88) | -2 (2.19) |
| 11-19 | -1.96 | -3.80 | -1.84 | -4 (4.76) | -3 (5.04) | 0 (2.88) | -3 (2.16) |
| 11-20 | -3.64 | -12.81 | +1.03 | -13 (4.63) | -12 (4.92) | -11 (2.77) | -1 (2.15) |

Base 11-13: 2Y 4.88, 10Y 5.15, real 2.91, breakeven 2.24. SPY over the five days: about -8.5% (my product of the daily returns). The 11-20 real-yield fall (-11 bp) is the main part of the 10-year fall (-12 bp). Breakeven did not move.

Reading A (sector event): chips fell 10 to 14% in each of the five names, and NVDA-type names are heavy in SPY. The index fall may be mostly this. Reading B (broad risk-off): the equal-weight index fell 2.28% without the chips, and the 11-19 fall was broad (defensives -1.84%). Falling real yields fit a move to safety, or a lower growth outlook. These are readings. The data show co-movement, not cause. The two readings can both be true.

## 2. Official calendar, 11-19 and 11-20

| Source page (read 2026-11-21) | 11-19 | 11-20 |
|---|---|---|
| federalreserve.gov/newsevents/2026-november.htm | Jefferson, IMF/Central Bank of Brazil conference, 9:25 a.m.; Barr, FinRegLab AI symposium, 2:15 p.m. H.4.1 weekly balance sheet, 4:30 p.m. | Waller, College Fed Challenge welcome, 8:25 a.m.; Cook, Central Bank of Chile, 12:15 p.m. H.8 bank data, 4:15 p.m. |
| census.gov/economic-indicators (2026 calendar) | Advance Services Report, Q3, 10:00 a.m. | None listed |
| bea.gov release schedule | Page shows only upcoming releases (first is 11-25): not confirmed for 11-19/20 | same |
| bls.gov schedule | not checked (page refused, HTTP 403) | not checked |
| dol.gov weekly claims | not checked (page refused, HTTP 403) | not checked |

The Fed page shows no FOMC meeting. FOMC minutes of 10-27/28 came on 11-18 at 2:00 p.m., and the next meeting is 12-08/09 (earlier note). I found no major scheduled release on 11-20. This is a fact about the calendar, not a cause.

## 3. What would settle the cause

| Data | In the store? |
|---|---|
| Company news for the chip supply chain, by date | No. NVDA filed its 8-K item 2.02 on 2026-11-17 (`data/raw/NVDA/sec_8k_earnings.json`; nasdaq date 11-17 in `earnings_dates.json`). The store has no time of day. NVDA moved -0.4% on 11-17 and -2.9% on 11-18, so 11-20 is three trading days later. No 11-19/20 earnings 8-K of the 10 largest (Fundamental Research). |
| Intraday prices (did the fall start before or after a time?) | No (daily bars only). |
| Options volume and implied volatility | No. |
| VIX close for 11-20, credit spreads, dollar | No VIX for 11-20. Others not checked. |
| Full-index sector returns, news wires, export-rule or customer-capex news | Not in the store. Needs the Head of Technology or a news source. |

Not done: no download, no check of unsourced figures from Fundamental Research (equal weight, sector returns); I did not check those against a file.
