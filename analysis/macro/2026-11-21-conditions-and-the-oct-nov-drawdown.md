# Macro brief: conditions on 2026-11-21 and the drawdown of 2026-10-06 to 2026-11-19

**Main finding.** SPY fell 8.11% from 2026-10-06 to 2026-11-19 (779.09 to 715.87, split-adjusted only). Two readings fit. (a) A higher equity risk premium: for it, VIX 15.01 to 28.44, HY B +0.71 pp, CCC +1.39 pp; against it, the IG move is small (BBB +0.14 pp) and the breakeven fell 0.20 pp. (b) A lower growth of AI capex: for it, NVDA -38.98%, GOOGL -21.58%, META -19.31%, and EPS revisions of GOOGL and AMZN lean down; against it, reported capex rose in all six names. The cache has no capex guidance and no daily estimates, so it cannot decide. It has no credit data after 11-18 and no SPY close for 11-20.

Data: the store `louellet-git/fa-data`, commit 1804f48 (2026-11-21). Run date 2026-11-21. This brief gives context. It is not a forecast, a model input or a trade signal. Rates are in percent, spreads in percentage points (pp). A date in brackets is the date of the data, not of the run. A fact is a published value. A reading is what it can mean.

## Part 1. Current conditions

| Indicator (unit) | Latest [data date] | 3M ago (2026-08-20) | 12M ago | Source |
|---|---|---|---|---|
| EFFR, % | 3.88 [11-19] | 3.63 | no data | live NY Fed query (`fa rates nyfed`), 2026-11-21 |
| SOFR, % | 3.91 [11-19] | 3.63 | no data | live NY Fed query (`fa rates nyfed`), 2026-11-21 |
| FOMC target range, % | 3.75 to 4.00 [11-19] | 3.50 to 3.75 | 3.75 to 4.00 | Atlanta Fed file (`fa rates`) |
| Treasury 3M / 2Y, % | 4.32 / 4.63 [11-20] | 3.87 / 4.19 | no data | Treasury (`rates/treasury_*_2026.csv`) |
| Treasury 10Y / 30Y, % | 4.92 / 5.31 [11-20] | 4.69 / 5.23 | no data | Treasury |
| GSW 10Y zero, % | 5.135 [11-13] | n/a | 4.1965 [2025-11-19] | Fed GSW (`rates/gsw_feds200628.csv`); +0.94 pp |
| 10Y minus 2Y, pp | +0.29 [11-20] | +0.50 | no data | calculated |
| 10Y minus 3M, pp | +0.60 [11-20] | +0.82 | no data | calculated |
| Real 10Y (TIPS), % | 2.77 [11-20] | 2.35 | no data | Treasury |
| Breakeven 10Y, % | 2.15 [11-20] | 2.34 | no data | `fa rates inflation` |
| 5y5y forward inflation, % | 2.12 [11-20] | 2.34 | no data | `fa rates inflation` |
| CPI-U 12-month, % | 3.74 [Sep 2026] | 3.36 [Jul 2026, latest published at 08-20] | 3.01 [Sep 2025] | BLS (`rates/bls_cpi.json`) |
| Core CPI 12-month, % | 2.56 [Sep 2026] | 2.48 [Jul 2026] | 3.02 [Sep 2025] | BLS |
| OAS AAA / AA / A / BBB, pp | 0.47 / 0.67 / 0.77 / 1.16 [11-18] | 0.42 / 0.61 / 0.69 / 1.00 | 0.37 / 0.49 / 0.69 / 1.07 [2025-11-18] | ICE BofA via FRED (`bonds/`) |
| OAS BB / B / CCC, pp | 2.05 / 3.73 / 13.53 [11-18] | 1.63 / 2.94 / 10.35 | 1.95 / 3.40 / 9.22 | same |
| Moody's Baa minus 10Y, pp | 1.52 [11-18] | 1.64 | 1.79 | FRED `BAA10Y` |
| VIX close, points | 28.44 [11-19] | 16.01 | 23.66 [2025-11-19] | Cboe (`vix/VIX_History.csv`) |
| Policy-rate path, window of 2026-12-16, mean % | 4.29 [11-19] | 3.93 | 3.11 [2025-11-19] | Atlanta Fed (`fa futures policy-path`) |

Notes. (1) The Treasury file starts in 2026, so the 12M change of Treasury, TIPS and breakeven is a data gap. GSW is a fitted zero yield: compare it only with itself. (2) The Oct 2026 CPI index (337.511) has no 12-month change: the Oct 2025 index is not in the file. (3) VX settlements of 2026-11-19: front 24.54, Jan 2027 23.99, Jul 2027 23.20. The weeklies and Dec carry the same 24.5357 (`docs/HIDDEN-ASSUMPTIONS.md` #73), so the front is not an independent point. (4) The policy path is the price of SOFR options. It is not a forecast of this repo or of the Fed. At 4.29% it is above the top of the range (4.00%).

Reading. Facts: short rates rose 0.25 to 0.5 pp in 3 months. Real yields rose. Inflation expectations fell. Headline CPI rose (3.36 to 3.74); core fell (2.59 in Jun to 2.56 in Sep). One reading: a tight policy setting, with a fall in the long yield in the last days. Other reading: the fall of the 10Y on 11-20 (-12 bp; 2Y -13 bp) is a growth scare, not a gain in inflation control. The cache cannot tell which. I did not look for the cause of the 11-20 move.

## Part 2. Question of the Head of Fundamental Research

Fact: SPY close, split-adjusted only (`SPY/prices.meta.json`, field `adjusted`): 779.09 on 10-06, 715.87 on 11-19, -8.11%. The title window is exact only for SPY. Other names peaked on other days: MSFT peaked on 11-04 (555.80; -10.35% to 11-19), GOOGL and META on 10-13, AMZN on 11-10 (-4.98%). NVDA and ORCL peaked on 10-06.

### (a) A higher equity risk premium

| Series | 2026-10-06 | 2026-11-19 (or last) | Change | Source |
|---|---|---|---|---|
| VIX, points | 15.01 | 28.44 | +13.43 | Cboe |
| OAS B / CCC, pp [11-18] | 3.02 / 12.14 | 3.73 / 13.53 | +0.71 / +1.39 | FRED `bonds/` |
| OAS BB / BBB / AAA, pp [11-18] | 1.85 / 1.02 / 0.39 | 2.05 / 1.16 / 0.47 | +0.20 / +0.14 / +0.08 | FRED `bonds/` |
| Baa minus 10Y, pp [11-18] | 1.46 | 1.52 | +0.06 | FRED `BAA10Y` |
| 10Y / 2Y Treasury, % | 5.27 / 4.79 | 5.04 / 4.76 (4.92 / 4.63 on 11-20) | -0.23 / -0.03 | Treasury |
| 10Y minus 3M, pp | +1.06 | +0.70 (+0.60 on 11-20) | -0.36 | calculated |
| Real 10Y / breakeven 10Y / 5y5y, % | 2.91 / 2.36 / 2.35 | 2.88 / 2.16 / 2.10 | -0.03 / -0.20 / -0.25 | Treasury; `fa rates inflation` |
| Path, 2026-12 / 2027-12 / 2028-12 window, % | 4.24 / 4.76 / 4.77 | 4.29 / 4.67 / 4.52 | +0.05 / -0.09 / -0.25 | Atlanta Fed |

Facts. Spreads widen more in the weak grades than in the strong grades, in pp (B +0.71, CCC +1.39, BBB +0.14). In percent the order differs. The VIX close of 11-19 (28.44) is the highest since 2026-03-30 (30.61). The VIX closed 28.18 on 11-05, near the 11-19 level, then fell to 17.49 on 11-11. HY B rose to 3.46 on 10-22 and fell to 3.18 on 10-28. It then jumped to 3.60 on 10-30 and stayed between 3.39 and 3.68 until 11-17 (3.68). It reached 3.73 on 11-18. The credit data stop on 11-18. The fall of the 10Y to 11-19 is breakeven (-0.20 pp), not real yield (-0.03 pp). The near path moved up 5 bp. The far path moved down 25 bp.

Reading (agrees): the required return on risk may be higher. VIX, HY and CCC spreads are all higher. The policy path shows no surprise cut.

Other reading (disagrees in part): the IG move is small for an 8% fall of the index. The fall of the breakeven and of the far path agrees with lower expected growth, not with a higher premium. The VIX can rise by itself when a few large names fall. The "plug" is the part of the price change that rates and new facts do not explain in `fa dcf attribute` (ticket T-20261120-valuation-move-attribution). A high plug does not prove a premium.

What decides: total OAS of HY and IG to 11-20; the VX curve of 10-06; the option skew of 10-06 (the SPX option copy is of 2026-11-20 only); an independent premium series.

### (b) A lower expected growth of AI capital spending

Capex, USD billion per quarter. Source: SEC company facts (`<TICKER>/company_facts.json.gz`). Where a filing gives YTD only, the quarter is the YTD value less the earlier YTD value. Tag `PaymentsToAcquirePropertyPlantAndEquipment`; AMZN and NVDA use `PaymentsToAcquireProductiveAssets`.

| Name | Latest quarter | Year before | Quarters before (newest first) | Filed (2026) | Source |
|---|---|---|---|---|---|
| MSFT (Sep-26) | 39.37 | 19.39 | 35.80, 30.88, 29.88 | 10-28 | SEC company facts |
| GOOGL (Sep-26) | 51.35 | 23.95 | 44.92, 35.67, 27.85 | 10-29 | SEC company facts |
| META (Sep-26) | 33.94 | 18.83 | 30.12, 19.00, 21.38 | 10-22 | SEC company facts |
| AMZN (Sep-26) | 77.26 | 35.09 | 54.21, 44.20, 39.52 | 10-30 | SEC company facts |
| ORCL (Aug-26) | 28.50 | 8.50 | 16.49, 18.64, 12.03 | 09-11 | SEC company facts |
| NVDA (Jul-26) | 2.68 | 1.90 | 1.76, 1.28, 1.64 | 08-26 | SEC company facts |

Price change 10-06 to 11-19 (`<TICKER>/prices.json`, split-adjusted): NVDA -38.98%, ORCL -23.62%, GOOGL -21.58%, META -19.31%, MSFT -5.86%, AMZN +0.42%.

Estimates (`<TICKER>/estimates`, Nasdaq). The store has one snapshot (2026-11-19) for each name, and a second only for NVDA (2026-11-20). So a change over the window cannot be measured. The revenue and capex estimate fields are empty. Mean price target, USD, is the analyst consensus, not a value of this repo (price on 11-19): NVDA 232.59 (145.98), MSFT 632.48 (498.30), GOOGL 398.50 (272.66), META 857.24 (596.22), AMZN 324.41 (257.37), ORCL 241.83 (110.57). EPS revisions, current fiscal year, up/down: GOOGL 0/13, AMZN 1/8, ORCL 2/2, META 4/0, MSFT 8/0, NVDA 13/2. NVDA reported on 11-17: EPS 2.68 against a consensus of 2.47 (+8.5%). The price fell on each later day. The NVDA target is the same in both snapshots, so it may be old.

Facts. Reported capex rose in each name. Four of the six filings (META, MSFT, GOOGL, AMZN) came inside the window. The cache has no capex guidance, so it does not show a cut.

Reading (agrees): the prices of NVDA, GOOGL, META and ORCL fell most. The revisions of GOOGL and AMZN lean down.

Other reading (disagrees): no cached number shows a lower spend. The doubt may be about the return on the spend (cash flow and funding), not about its growth. More capex lowers free cash flow, so the sign is opposite. In USD bn, AMZN has the largest rise (+42.17 on the year before) and an unchanged price. In percent, ORCL has the largest rise (+235%).

What decides: capex guidance in the earnings text; a daily series of consensus capex and EPS; the NVDA 10-Q for the quarter to Oct 2026 (not in the cache); debt issue by the five names.

## Data gaps (request to the Head of Technology)

- Labor, GDP, PCE, retail sales, ALFRED vintages (FRED ids: `PAYEMS`, `UNRATE`, `ICSA`, `GDPC1`, `PCEPI`, `PCEPILFE`, `RSAFS`).
- Credit: total OAS `BAMLC0A0CM`, `BAMLH0A0HYM2`; `NFCI`, `STLFSI4`; credit data of 11-19 and 11-20.
- 2025 files for Treasury, TIPS, breakeven, EFFR and SOFR.
- VX files for 2026-10-06 and 2026-08-20; the SPY close of 2026-11-20.

## Process note

Some `fa` commands download a missing file for a date that is not in the cache. No number from such a file is in this brief, except EFFR and SOFR (live NY Fed query, as the table says).
