# Stress windows 2016 to 2026-11-19, for backtests

Data: the store `louellet-git/fa-data`, commit 1804f48 (2026-11-21).

Run date 2026-11-21. Data from the store copy `data/raw` (SNAPSHOT.md). This file is context for backtests. It is not a model input, a forecast or a trade signal.

## Rule (one rule, used for each window)

1. Trigger: a VIX close of 25 or more.
2. Start: the last VIX close below 20 before the trigger.
3. End: the first VIX close below 20 after the VIX peak. The peak is the highest VIX close in the window.
4. Merge: if the next trigger comes 10 trading days or fewer after an end, the two windows join.
5. List a window only if it has 3 or more closes of 25 or more.
6. A change is the value on the end date less the value on the start date. A series with no value on a date uses the last earlier value.

The rule uses VIX only. I did not choose a date from a price or a story. The levels 25, 20, 10 days and 3 closes are conventions of mine. A different level gives different dates. Rule 4 joins 2020-02 to 2021-03 (13 months) and the two 2026 episodes. Thus the 2020 row covers the recovery too: its changes (SPY +18.72%) do not show the stress. For a test of the acute phase only, use 2020-02-21 to the worst SPY close, 2020-03-23.

## Table

Changes are start to end. SPY: close change in % (worst close from the start, and its date). Yields and spreads: pp. 10Y: GSW zero yield to 2025, Treasury par yield (CMT) from 2026 (the two are not equal: do not compare across). BAA10Y: Moody's Baa less the 10Y Treasury. HY B: ICE BofA B OAS.

| Window | Start | End | Cause (context, not data) | Peak VIX [date] | SPY % (worst) | 10Y pp | BAA10Y pp | HY B pp |
|---|---|---|---|---|---|---|---|---|
| 2016-02 | 2016-01-05 | 2016-02-22 | Oil price fall, China growth fear | 28.14 [2016-02-11] | not in store | -0.49 | +0.32 | not in store |
| 2018-02 | 2018-02-02 | 2018-02-14 | Unwind of short-volatility products | 37.32 [2018-02-05] | -2.13 (-6.47, 02-08) | +0.09 | +0.08 | not in store |
| 2018-Q4 | 2018-12-03 | 2019-01-09 | Fed tightening, trade dispute, growth fear | 36.07 [2018-12-24] | -7.64 (-16.10, 12-24) | -0.27 | +0.13 | not in store |
| 2020-03 | 2020-02-21 | 2021-03-16 | COVID-19 lockdowns | 82.69 [2020-03-16] | +18.72 (-33.14, 2020-03-23) | +0.25 | +0.10 | not in store |
| 2021-12 | 2021-11-24 | 2021-12-08 | Omicron news, Fed policy turn | 31.12 [2021-12-01] | +0.02 (-4.03, 12-01) | -0.14 | +0.07 | not in store |
| 2022-Q1 | 2022-01-14 | 2022-03-28 | Start of Fed rate rises, war in Ukraine | 36.45 [2022-03-07] | -1.90 (-10.43, 03-08) | +0.64 | +0.15 | not in store |
| 2022-Q2/Q3 | 2022-04-04 | 2022-08-10 | Fed rate rises, high inflation | 34.75 [2022-05-09] | -8.06 (-19.91, 06-17) | +0.35 | +0.48 | not in store |
| 2022-H2 | 2022-08-18 | 2023-01-12 | Fed rate rises, UK gilt turmoil | 33.63 [2022-10-11] | -7.23 (-16.67, 10-12) | +0.54 | -0.27 | not in store |
| 2022 (the three joined) | 2022-01-14 | 2023-01-12 | Rate rise year | 36.45 [2022-03-07] | -14.58 (-23.27, 2022-10-12) | +1.56 | +0.18 | not in store |
| 2023-03 | 2023-03-08 | 2023-03-28 | US regional bank failures | 26.52 [2023-03-13] | -0.83 (-3.40, 03-13) | -0.40 | +0.40 | not in store |
| 2024-08 | 2024-08-01 | 2024-08-13 | Yen carry unwind, weak US jobs data | 38.57 [2024-08-05] | -0.18 (-4.72, 08-05) | -0.13 | +0.09 | +0.20 |
| 2025-04 | 2025-03-27 | 2025-05-12 | US tariff announcements | 52.33 [2025-04-08] | +2.81 (-12.45, 04-08) | +0.11 | +0.14 | -0.06 |
| 2026-03 | 2026-02-27 | 2026-04-09 | Not checked (no source in store) | 31.05 [2026-03-27] | -0.89 (-7.87, 03-30) | +0.32 | -0.07 | -0.22 |
| 2026-11 (open) | 2026-11-04 | 2026-11-19* | Not checked; see the brief of 2026-11-21 | 28.44 [2026-11-19] | -6.64 (-6.64, 11-19) | +0.14 | 0.00 | +0.22 |
| Named by the Head: 2026-10-06 to 2026-11-19 (SPY high to last day; not from the rule) | 2026-10-06 | 2026-11-19* | as above | 28.44 [2026-11-19] | -8.11 (-8.11, 11-19) | -0.23 | +0.06 | +0.71 |

\* The window is open: the VIX has not closed below 20. The end is the last day in the store. BAA10Y and HY B end on 2026-11-18 (the last FRED value).

Not listed by the rule: 2016-06 (Brexit, peak 25.76), 2018-10 (25.23), 2021-05, 2021-09, 2024-12, 2025-02/03 (peak 27.86, 2 closes), 2025-10 and 2025-11 (1 close each), because they have fewer than 3 closes of 25 or more. 2019-08: the VIX peak is 24.59 [2019-08-05], below the trigger, so no window. 2020-09/10: no separate window, because the VIX stayed 20 or more. The peaks inside the 2020 window: 40.79 [2020-06-11], 33.60 [2020-09-03], 40.28 [2020-10-28].

## Sources

- VIX close: `data/raw/vix/VIX_History.csv`, column CLOSE (Cboe, points).
- SPY close: `data/raw/SPY/prices.json`, field `close` (Nasdaq site, split-adjusted, not adjusted for dividends). Start 2016-11-18, so the windows before that have no SPY value.
- 10Y to 2025: `data/raw/rates/gsw_feds200628.csv`, column SVENY10 (fitted zero yield, continuously compounded, %; ends 2026-11-13). Not a par yield.
- 10Y in 2026: `data/raw/rates/treasury_nominal_2026.csv`, column "10 Yr" (par yield, %).
- BAA10Y: `data/raw/bonds/fred_BAA10Y.csv`, series BAA10Y (%, Moody's, daily).
- HY B OAS: `data/raw/bonds/fred_BAMLH0A2HYB.csv`, series BAMLH0A2HYB (ICE BofA B, %).
- Cause column: public context of mine. No file in the store gives it.

## Data gaps

- ICE BofA OAS (HY and rated) starts 2023-11-20 (FRED licence: 3 years). Before that it is not in the store. A source would be a licensed copy.
- BAA10Y is a Moody's index, not the ICE IG OAS. It is the only IG series for the long period. It has a different maturity and rating mix.
- SPY starts 2016-11-18: no SPY for 2016-02. The SPX index is not in the store for that period.
- GSW ends 2026-11-13 and the Treasury file starts 2026-01-02: no one 10Y series covers all windows. A 2025-to-2026 window cannot use one series.
- The cache has no close for SPY and the VIX on 2026-11-20.

## Commands

No `fa` command ran. I read the files with a Python script (not in the repo). Anyone can run the rule again from the files in Sources.
