# Macro brief: conditions on 2026-11-21 and the drawdown of 2026-10-06 to 2026-11-19

Run date 2026-11-21. Cache only (`data/raw`). This brief gives context. It is not a forecast, a model input or a trade signal. Rates are in percent, spreads in percentage points (pp). A date in brackets is the date of the data, not of the run.

## Part 1. Current conditions

| Indicator (unit) | Latest [data date] | 3M ago (2026-08-20) | 12M ago | Source |
|---|---|---|---|---|
| EFFR, % | 3.88 [11-19] | 3.63 | no data | NY Fed (`fa rates nyfed`) |
| SOFR, % | 3.91 [11-19] | 3.63 | no data | NY Fed |
| FOMC target range, % | 3.75 to 4.00 [11-19] | 3.50 to 3.75 | 3.75 to 4.00 | `fa rates`; Atlanta Fed file |
| Treasury 3M / 2Y, % | 4.32 / 4.63 [11-20] | 3.87 / 4.19 | no data | Treasury (`fa rates curve`) |
| Treasury 10Y / 30Y, % | 4.92 / 5.31 [11-20] | 4.69 / 5.23 | 10Y: GSW zero 4.20 [2025-11-19], now 5.13 [11-13] | Treasury; GSW (`fa forwards`) |
| 10Y minus 2Y, pp | +0.29 [11-20] | +0.50 | no data | calculated |
| 10Y minus 3M, pp | +0.60 [11-20] | +0.82 | no data | calculated |
| Real 10Y (TIPS), % | 2.77 [11-20] | 2.35 | no data | Treasury |
| Breakeven 10Y, % | 2.15 [11-20] | 2.34 | no data | `fa rates inflation` |
| 5y5y forward inflation, % | 2.12 [11-20] | 2.34 | no data | `fa rates inflation` |
| CPI-U 12-month, % | 3.74 [Sep 2026] | 3.53 [Jun 2026] | 3.01 [Sep 2025] | BLS (`bls_cpi.json`) |
| Core CPI 12-month, % | 2.56 [Sep 2026] | 2.59 [Jun] | 3.02 [Sep 2025] | BLS |
| OAS AAA / AA / A / BBB, pp | 0.47 / 0.67 / 0.77 / 1.16 [11-18] | 0.42 / 0.61 / 0.69 / 1.00 | 0.37 / 0.49 / 0.69 / 1.07 [2025-11-18] | ICE BofA via FRED (`fa bonds`) |
| OAS BB / B / CCC, pp | 2.05 / 3.73 / 13.53 [11-18] | 1.63 / 2.94 / 10.35 | 1.95 / 3.40 / 9.22 | same |
| Moody's Baa minus 10Y, pp | 1.52 [11-18] | 1.64 | 1.79 | FRED `BAA10Y` |
| VIX close, points | 28.44 [11-19] | 16.01 | 23.66 [2025-11-19] | Cboe (`vix/VIX_History.csv`) |
| Policy-rate path, window of 2026-12-16, mean % | 4.29 [11-19] | 3.93 | 3.11 [2025-11-19] | Atlanta Fed (`fa futures policy-path`) |

Notes. (1) The Treasury file has no year before 2026 in the cache, so the 12M change of the Treasury, TIPS and breakeven series is a data gap. GSW is a fitted zero yield, not a par yield: compare it only with itself (+0.93 pp, 2025-11-19 to 2026-11-13). (2) The Oct 2026 CPI index exists (337.511) but has no 12-month change: the Oct 2025 index is not in the file. (3) VX settlements of 2026-11-19: front 24.54, Jan 2027 23.99, Jul 2027 23.20. The VIX close (28.44) is above every future, so the front of the curve is inverted and the rest slopes down slowly. Only one VX file is in the cache. (4) The policy path is the price of SOFR options, not a forecast of this repo or of the Fed. 4.29% is above the top of the range (4.00%): the market prices a hike (probability above range 98%).

Reading. Fact: short rates rose 0.25 to 0.5 pp in 3 months, real yields rose, inflation expectations fell, CPI rose. One reading: a tight policy setting with a fall in the long yield in the last days. Other reading: the fall of the 10Y on 11-20 (-12 bp in one day, 2Y -13 bp) is a growth scare, not a gain in inflation control. The cache cannot tell which. I did not look for the cause of the 11-20 move.

## Part 2. Question of the Head of Fundamental Research

Fact (SPY close, adjusted, `fa prices show`): 779.09 on 10-06, 715.87 on 11-19, -8.11%. The cache has no close for 11-20.

### (a) A higher equity risk premium

| Series | 2026-10-06 | 2026-11-19 (or last) | Change |
|---|---|---|---|
| VIX, points | 15.01 | 28.44 | +13.43 (highest close since 10-06) |
| OAS B / CCC, pp [11-18] | 3.02 / 12.14 | 3.73 / 13.53 | +0.71 / +1.39 |
| OAS BB / BBB / AAA, pp [11-18] | 1.85 / 1.02 / 0.39 | 2.05 / 1.16 / 0.47 | +0.20 / +0.14 / +0.08 |
| Baa minus 10Y, pp [11-18] | 1.46 | 1.52 | +0.06 |
| 10Y / 2Y Treasury, % | 5.27 / 4.79 | 5.04 / 4.76 (4.92 / 4.63 on 11-20) | -0.23 / -0.03 |
| 10Y minus 3M, pp | +1.06 | +0.70 (+0.60 on 11-20) | -0.36 |
| Real 10Y / breakeven 10Y / 5y5y, % | 2.91 / 2.36 / 2.35 | 2.88 / 2.16 / 2.10 | -0.03 / -0.20 / -0.25 |
| Path, 2026-12 / 2027-12 / 2028-12 window, % | 4.24 / 4.76 / 4.77 | 4.29 / 4.67 / 4.52 | +0.05 / -0.09 / -0.25 |

Facts: spreads widen in the weak grades more than in the strong grades. HY B reached 3.46 on 10-22, fell to 3.22 on 10-27, and reached 3.73 on 11-18. The VIX rise is mainly 11-10 (19.46) to 11-19 (28.44), and the credit data stop on 11-18. The fall of the 10Y to 11-19 is breakeven (-0.20 pp), not real yield (-0.03 pp). The near path moved up 5 bp. The far path moved down 25 bp.

Reading (agrees): a rise of the required return on risk: VIX, HY and CCC spreads are all higher. The policy path gives no sign of a surprise cut that could explain a rate gain. Other reading (disagrees in part): the IG move is small (BBB +14 bp, Baa-10Y +6 bp) for an 8% fall of the index. The fall of the breakeven and of the far path agrees with lower expected growth, not with a higher premium. The VIX can rise by itself when a few large names fall. The split's plug takes every price change that rates and new facts do not explain, so a high plug does not prove a premium. What decides: HY and IG total OAS to 11-20; the VX curve of 10-06; the 10-06 option skew (the SPX option copy is of 2026-11-20 only); a market premium series from an independent method.

### (b) A lower expected growth of AI capital spending

Capex, USD billion, quarter (SEC company facts; the quarter is the YTD value less the earlier YTD value where the filing gives YTD only; tag `PaymentsToAcquirePropertyPlantAndEquipment`, AMZN and NVDA `PaymentsToAcquireProductiveAssets`):

| Name | Latest quarter | Year before | Quarters before (newest first) | Filed |
|---|---|---|---|---|
| MSFT (Sep-26) | 39.37 | 19.39 | 35.80, 30.88, 29.88 | 10-28 |
| GOOGL (Sep-26) | 51.35 | 23.95 | 44.92, 35.67, 27.85 | 10-29 |
| META (Sep-26) | 33.94 | 18.83 | 30.12, 19.00, 21.38 | 10-22 |
| AMZN (Sep-26) | 77.26 | 35.09 | 54.21, 44.20, 39.52 | 10-30 |
| ORCL (Aug-26) | 28.50 | 8.50 | 16.49, 18.64, 12.03 | 09-11 |
| NVDA (Jul-26) | 2.68 | 1.90 | 1.76, 1.28, 1.64 | 08-26 |

Price 10-06 to 11-19 (`fa prices show`): NVDA -38.98%, ORCL -23.62%, GOOGL -21.58%, META -19.31%, MSFT -5.86%, AMZN +0.42%.

Estimates. The store has one snapshot (2026-11-19) for each name, and a second only for NVDA (2026-11-20). So a change over the window cannot be measured. The files have EPS estimates, the mean price target, and revision counts (no window stated); the revenue and capex estimate fields are empty. Mean target, USD (price on 11-19): NVDA 232.59 (145.98), MSFT 632.48 (498.30), GOOGL 398.50 (272.66), META 857.24 (596.22), AMZN 324.41 (257.37), ORCL 241.83 (110.57). EPS revisions, current fiscal year, up/down: GOOGL 0/13, AMZN 1/8, ORCL 2/2, META 4/0, MSFT 8/0, NVDA 13/2. NVDA reported on 11-17: EPS 2.68 against a consensus of 2.47 (+8.5%), and the price fell on each later day. The NVDA target is the same in both snapshots, so it may be old.

Facts: reported capex rose in each name, and four of the six filings (META, MSFT, GOOGL, AMZN) came inside the window. The cache has no capex guidance, so it does not show a cut.
Reading (agrees): the prices of NVDA, GOOGL, META, ORCL fell most, and the revisions of GOOGL and AMZN lean down. Other reading (disagrees): no cached number shows a lower spend. The doubt may be about the return on the spend (cash flow and funding), not about its growth. This gives the opposite sign: more capex lowers free cash flow. AMZN has the largest rise and an unchanged price. What decides: capex guidance in the earnings text; a daily series of consensus capex and EPS over the window; NVDA's FQ3 10-Q (quarter to Oct 2026, not in the cache); debt issue by the five names.

## Data gaps (request to the Head of Technology)

- Not in the store: ALFRED vintages (so no real-time data), labor, GDP, PCE. Likely sources (FRED ids): payrolls `PAYEMS`, unemployment `UNRATE`, claims `ICSA`, hourly earnings `CES0500000003`, GDP `GDPC1`, PCE price `PCEPI` and `PCEPILFE`, real PCE `PCEC96`, retail sales `RSAFS`. ALFRED: the same ids with vintage dates.
- Credit: total OAS `BAMLC0A0CM`, `BAMLH0A0HYM2`; conditions `NFCI`, `STLFSI4`; credit data of 11-19 and 11-20.
- Treasury, TIPS and breakeven files for 2025 (12M changes); EFFR and SOFR for 2025.
- VX settlement files for 2026-10-06 and 2026-08-20, and the close of SPY/SPX on 2026-11-20.
- Capex guidance and consensus capex (no free source known).

## Process note

Some `fa` commands download a missing file when they get a date that is not in the cache (`fa rates inflation --date 2025-11-20`, `fa futures positions --weeks 6`). No number from such a file is in this brief. The Head of Technology has the details.
