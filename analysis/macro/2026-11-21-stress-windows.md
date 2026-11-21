# Stress windows 2016 to 2026-11-19, for backtests

Data: the store `louellet-git/fa-data`, commit 1804f48 (2026-11-21), copy `data/raw`. Run date 2026-11-21. This file is context for backtests. It is not a model input, a forecast or a trade signal.

## Main finding

- The rule below gives 12 closed windows and 1 open window (2026-11), 13 in all, from 2016-01 to 2026-11-19. Eight more trigger windows have fewer than 3 closes of 25 or more and are not listed.
- Data gaps: no SPY for the 2016-02 window (SPY starts 2016-11-18). No HY B spread before 2023-11-20, so only 4 windows (2024-08, 2025-04, 2026-03, 2026-11) have it. The 10Y series change at 2026 (GSW to Treasury par): do not compare across.
- Quant Research can use the list as a set of candidate dates. That head chooses the test dates. The 2020 window is 13 months long: it does not show the stress only.
- Uncertain: the levels 25, 20, 10 days and 3 closes are conventions. A different level gives different dates (see the edge case under the rule).
- Watch: the open 2026-11 window ends at the first VIX close below 20.

## Rule (one rule, used for each window)

1. Calendar: the trading days of the Cboe VIX file (`vix/VIX_History.csv`).
2. Trigger: a VIX close of 25 or more.
3. Start: the last close below 20 before the trigger. "Below 20" means < 20. "20 or more" means >= 20.
4. Peak: the highest close from the trigger to the first close below 20 after the trigger.
5. End: the first close below 20 after the trigger.
6. Open window: if no close below 20 exists by the last date of the file, the end is the last close, marked "open".
7. Merge: the gap is the number of trading days after the end day, up to and including the next trigger day. Join two windows if the gap is 10 or fewer. A merge can chain (three or more windows). The joined peak is the highest close of all joined windows. Apply the merge to all trigger windows before step 8.
8. Listing: list a window only if it has 3 or more closes of 25 or more.
9. Worst SPY close: the lowest SPY close from the start to the end, and its date.
10. Changes: the end value less the start value. A series with no value on a date uses the last earlier value.

I checked this rule on the file: it gives the table below, row by row. No row needed a change.

Edge case: the 2025-02/03 window ends 2025-03-19. The 2025-04 trigger is 2025-04-03. The gap is 11 trading days (2025-03-20 to 2025-04-03), so the two do not join. With a gap limit of 11, they join: the 2025-04 start becomes 2025-02-28 and its values change.

2020: the first part is one run at 20 or more, from 2020-02-24 to 2021-02-12 (first close below 20: 19.97). The re-trigger on 2021-02-25 is 8 trading days later, so step 7 joins it. The end is 2021-03-16. The 2020 row therefore covers the recovery: SPY +18.72% over the window does not show the stress. SPY worst close from the start is -33.14% on 2020-03-23 (a fact from the file).

## Table (windows from the rule)

Changes are start to end. SPY: close change in % (worst close from the start, and its date). Yields and spreads: pp. 10Y: GSW zero yield to 2025, Treasury par yield (CMT) in 2026 (not equal: do not compare across). BAA10Y: Moody's Baa less the 10Y Treasury. HY B: ICE BofA B OAS.

| Window | Start | End | Event label (not checked, no source) | Peak VIX [date] | SPY % (worst) | 10Y pp | BAA10Y pp | HY B pp |
|---|---|---|---|---|---|---|---|---|
| 2016-02 | 2016-01-05 | 2016-02-22 | Oil price fall, China growth fear | 28.14 [2016-02-11] | not in store | -0.49 | +0.32 | not in store |
| 2018-02 | 2018-02-02 | 2018-02-14 | Unwind of short-volatility products | 37.32 [2018-02-05] | -2.13 (-6.47, 02-08) | +0.09 | +0.08 | not in store |
| 2018-Q4 | 2018-12-03 | 2019-01-09 | Fed tightening, trade dispute, growth fear | 36.07 [2018-12-24] | -7.64 (-16.10, 12-24) | -0.27 | +0.13 | not in store |
| 2020-03 | 2020-02-21 | 2021-03-16 | COVID-19 lockdowns | 82.69 [2020-03-16] | +18.72 (-33.14, 2020-03-23) | +0.25 | +0.10 | not in store |
| 2021-12 | 2021-11-24 | 2021-12-08 | Omicron news, Fed policy turn | 31.12 [2021-12-01] | +0.02 (-4.03, 12-01) | -0.14 | +0.07 | not in store |
| 2022-Q1 | 2022-01-14 | 2022-03-28 | Start of Fed rate rises, war in Ukraine | 36.45 [2022-03-07] | -1.90 (-10.43, 03-08) | +0.64 | +0.15 | not in store |
| 2022-Q2/Q3 | 2022-04-04 | 2022-08-10 | Fed rate rises, high inflation | 34.75 [2022-05-09] | -8.06 (-19.91, 06-17) | +0.35 | +0.48 | not in store |
| 2022-H2 | 2022-08-18 | 2023-01-12 | Fed rate rises, UK gilt turmoil | 33.63 [2022-10-11] | -7.23 (-16.67, 10-12) | +0.54 | -0.27 | not in store |
| 2023-03 | 2023-03-08 | 2023-03-28 | US regional bank failures | 26.52 [2023-03-13] | -0.83 (-3.40, 03-13) | -0.40 | +0.40 | not in store |
| 2024-08 | 2024-08-01 | 2024-08-13 | Yen carry unwind, weak US jobs data | 38.57 [2024-08-05] | -0.18 (-4.72, 08-05) | -0.13 | +0.09 | +0.20 |
| 2025-04 | 2025-03-27 | 2025-05-12 | US tariff announcements | 52.33 [2025-04-08] | +2.81 (-12.45, 04-08) | +0.11 | +0.14 | -0.06 |
| 2026-03 | 2026-02-27 | 2026-04-09 | not checked | 31.05 [2026-03-27] | -0.89 (-7.87, 03-30) | +0.32 | -0.07 | -0.22 |
| 2026-11 (open) | 2026-11-04 | 2026-11-19* | not checked | 28.44 [2026-11-19] | -6.64 (-6.64, 11-19) | +0.14 | 0.00 | +0.22 |

\* Open: the VIX has not closed below 20. The end is the last day in the store. BAA10Y and HY B end on 2026-11-18 (the last FRED value).

## Not from the rule

| Row | Start | End | Peak VIX [date] | SPY % (worst) | 10Y pp | BAA10Y pp | HY B pp |
|---|---|---|---|---|---|---|---|
| 2022, the three 2022 windows joined by hand | 2022-01-14 | 2023-01-12 | 36.45 [2022-03-07] | -14.58 (-23.27, 2022-10-12) | +1.56 | +0.18 | not in store |
| Named by the Head of Quant/Fundamental: SPY high to last day | 2026-10-06 | 2026-11-19* | 28.44 [2026-11-19] | -8.11 (-8.11, 11-19) | -0.23 | +0.06 | +0.71 |

The two 2022 gaps are above 10 trading days, so the rule keeps the three windows apart. The second row uses a start that is not a VIX date.

## Not listed

The rule does not list these windows, because each has fewer than 3 closes of 25 or more: 2016-06 (Brexit, peak 25.76), 2018-10 (25.23), 2021-05, 2021-09, 2024-12, 2025-02/03 (peak 27.86, 2 closes), 2025-10 and 2025-11 (1 close each).

2019-08 has no window: the VIX peak is 24.59 [2019-08-05], below the trigger. 2020-09/10 has no separate window, because the VIX stayed at 20 or more. The peaks inside the 2020 window are 40.79 [2020-06-11], 33.60 [2020-09-03] and 40.28 [2020-10-28].

## Sources (last data date)

- VIX close: `data/raw/vix/VIX_History.csv`, column CLOSE (Cboe, points). Last date 2026-11-19.
- SPY close: `data/raw/SPY/prices.json`, field `close` (Nasdaq site, split-adjusted, not adjusted for dividends). Starts 2016-11-18. Last date 2026-11-19.
- 10Y to 2025: `data/raw/rates/gsw_feds200628.csv`, column SVENY10 (fitted zero yield, continuously compounded, %). Last date 2026-11-13. Not a par yield.
- 10Y in 2026: `data/raw/rates/treasury_nominal_2026.csv`, column "10 Yr" (par yield, %).
- BAA10Y: `data/raw/bonds/fred_BAA10Y.csv` (%, Moody's, daily). Last date 2026-11-18 (FRED).
- HY B OAS: `data/raw/bonds/fred_BAMLH0A2HYB.csv` (ICE BofA B, %). Last date 2026-11-18 (FRED).
- Event label column: public context of mine. No file in the store gives it. Not checked.

## Data gaps

- ICE BofA OAS (HY and rated) starts 2023-11-20 (FRED licence: 3 years). A source would be a licensed copy.
- BAA10Y is a Moody's index, not the ICE IG OAS. It is the only IG series for the long period. It has a different maturity and rating mix.
- SPY starts 2016-11-18. The SPX index is not in the store for that period.
- GSW ends 2026-11-13 and the Treasury file starts 2026-01-02. No one 10Y series covers all windows.
- The cache has no close for SPY and the VIX on 2026-11-20.

## Commands

No `fa` command ran. I read the files with a Python script outside the repo. Anyone can run the rule again from the files in Sources.
