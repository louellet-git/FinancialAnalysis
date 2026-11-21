---
id: T-20261121-economics-data
title: Put the macro series of the Head of Economics in the data store
area: importer
status: backlog
owner: louellet-git
requested_by: louellet-git
head: technology
order: 3
branch:
created: 2026-11-21
acceptance: The store has the FRED series below with their dates, the ALFRED vintages of the labor, GDP and PCE series, EFFR and SOFR of 2025, and the Cboe VX settlements of 2026-08-20 and 2026-10-06; `SNAPSHOT.md` lists them; the Head of Economics reads them with no request.
---
Request of the Head of Economics (2026-11-21, not urgent).
- FRED: PAYEMS, UNRATE, ICSA, CES0500000003, GDPC1, PCEPI, PCEPILFE, PCEC96, RSAFS, NFCI, STLFSI4,
  SAHMREALTIME, T10Y3M, BAMLC0A0CM, BAMLH0A0HYM2. ALFRED vintages of the labor, GDP and PCE series.
  FRED needs a key. In a cloud session the key goes through the secure credential form (never a file or a
  chat); ask the person first. Check the FRED terms for a public copy of each series (some ICE series have
  limits), and keep a series out of the public store if the terms do not permit it.
- EFFR and SOFR of 2025: with item 1 of `T-20261120-store-arb-data-rest` (a cache file for `fa rates nyfed`).
- Cboe VX settlement files of more dates (at least 2026-08-20 and 2026-10-06): with item 2 of the same ticket.
- Done on 2026-11-21: the Treasury nominal and real files of 2025 (store commit 33d673b).

## Long history (second request of the Head of Economics, 2026-11-21)
For `T-20261121-dcf-engine-backtest` (Fundamental Research; owner word of 03:17 UTC: "look for as long of data
as you have") and `T-20261121-economics-long-history`. Ranked; each needs a check of its terms for a public copy
before it goes into the store (skill `data-source-onboarding`):
1. Shiller `ie_data.xls` (Yale): monthly S&P composite price, dividends, earnings, CPI and the 10-year rate from 1871.
2. FRED BAA, AAA (from 1919; Moody's terms), TB3MS (from 1934), GS10 (from 1953), CPIAUCNS (from 1913).
3. NBER Macrohistory: short rates before 1934 (commercial paper, call money).
4. Jorda-Schularick-Taylor Macrohistory database (17 countries, 1870-2020; its licence).
5. Bank of England, "A millennium of macroeconomic data" (UK consol yields).
6. Damodaran implied equity risk premium history (annual from 1960).
The files are small (monthly or annual): one request each, no bulk download.

## Older daily stock prices (request of Fundamental Research, 2026-11-21)
Prices of `sp500.txt` and `next500.txt` from 2009-01-01 (the Nasdaq files start on 2016-11-18; Nasdaq gives at
most 10 years; Yahoo refuses the cloud). The person said no to Tiingo (its terms forbid a public copy) and asked
for a better source. Research of free sources with open terms is running.
