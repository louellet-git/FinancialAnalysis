# Rates and the balance sheet: a measurement (ticket T-20261121-rates-balance-sheet, items 1 to 4)

This file gives a measurement and no model change. It shows how much the curve change of the week 2026-11-13 to 2026-11-20 moves the market value of the debt, the debt securities and the pension obligation of each company. The unit is points of the equity market value (value1, 2026-11-13). The rule: SEC facts and the Treasury curve only. No input comes from a price.

## Question
Is the balance-sheet effect above the threshold of item 3 (more than 0.25 points of the equity value, or more than 10% of the rates part)? Answer: for the S&P 10 and the S&P 100 as groups, no. For 29 of 86 single non-financial companies, yes.

## Data and commands
- Store `fa-data-split` (commit 0340bc1). Only facts with `filed` on or before 2026-11-13 and `end` on or after 2025-09-01.
- Extraction (one line for each ticker, jq): the latest value of 65 `us-gaap` tags. Calculation: awk. Both are in the session scratchpad, not in the repo.
- Curve: `rates/treasury_nominal_2026.csv`, par yields. The rates part is `p_rates_pct`, else `g_rates_pct` (premium first), else 0 (no model). This reproduces the bridge: S&P 100 +4.927, S&P 10 +5.572 points.
- Cross-check of the debt amount with `mapcheck values` (`total_debt`): 80 companies, 34 within 5%, 55 within 15%. AAPL 80.11 and 80.111 B, TMUS 86.3 and 86.28 B. Large gaps (AMZN, NVDA, GOOG, META) are because `mapcheck` uses the last annual row and this file uses the latest quarter.

## Curve change, par yield, 2026-11-13 to 2026-11-20 (basis points)
| 3 Mo | 6 Mo | 1 Y | 2 Y | 3 Y | 5 Y | 7 Y | 10 Y | 20 Y | 30 Y |
|---|---|---|---|---|---|---|---|---|---|
| 0 | -4 | -14 | -25 | -27 | -27 | -25 | -23 | -18 | -16 |

Each block uses the change of the nearest maturity to its duration (a tie goes to the shorter maturity). The 1-month yield rose 9 bp. Cash is held at zero value change.

## S&P 10 (points of value1; the sum is the equity gain)
| Ticker | Value $B | Debt $B | D debt (y) | Debt | Securities | Pension | Sum | Rates part | Sum / rates part |
|---|---|---|---|---|---|---|---|---|---|
| AAPL | 5198 | 80.1 | 6.8 | -0.023 | +0.011 | 0 | -0.012 | +10.58 | 0.1% |
| NVDA | 3955 | 33.4 | 6.5 d | -0.013 | +0.001 | 0 | -0.011 | +1.72 | 0.6% |
| MSFT | 3901 | 35.5 | 8.6 | -0.015 | +0.010 | 0 | -0.005 | +8.30 | 0.1% |
| GOOG | 3629 | 126.1 | 8.4 | -0.062 | +0.057 | 0 | -0.004 | +9.60 | 0.0% |
| AMZN | 2918 | 181.3 | 7.5 | -0.107 | +0.005 | 0 | -0.102 | none | n/a |
| AVGO | 1699 | 59.4 | 8.1 | -0.065 | 0 | 0 | -0.065 | +3.65 | 1.8% |
| META | 1651 | 83.7 | 9.4 | -0.109 | +0.013 | 0 | -0.096 | +1.65 | 5.8% |
| TSLA | 1382 | 8.0 | 2.5 | -0.004 | +0.017 | 0 | +0.014 | none | n/a |
| LLY | 1290 | 47.0 | 6.5 d | -0.059 | 0 | 0 | -0.058 | +6.80 | 0.9% |
| WMT | 1022 | 50.4 | 7.1 | -0.064 | 0 | 0 | -0.064 | +1.73 | 3.7% |
| **Group** | 26644 | | | | | | **-0.031** | **+5.57** | **0.6%** |

"d" is the default duration (no ladder). The pension block is zero because the store has no pension obligation tag for the ten. BRK-B and V have no share count and no weight (BRK-B has no debt tag; its debt securities are $17.1 B). Result: no S&P 10 company is above the threshold. The largest sum is AMZN, -0.102 points. The largest ratio is META, 5.8%. The group effect is -$8.3 B of $26.6 T.

## S&P 100 (98 companies with a weight; 86 non-financial)
The value-weighted balance-sheet effect is **-0.093 points** (-$45 B), next to the rates part of **+4.927 points**. The ratio is 1.9%: below both limits. Financials (12 companies, $3.2 T, 6.5% of the value) count zero in the sum. Assumption bracket (see table below): -0.067 to -0.109 points.

Largest single effects (points of value1; the debt block is the cause in each negative case):
| Ticker | Value $B | Debt $B | Debt | Securities | Pension | Sum | Rates part | Ratio |
|---|---|---|---|---|---|---|---|---|
| T | 152 | 150.1 | -1.865 | 0 | 0 | -1.865 | +5.86 | 32% |
| DUK | 101 | 90.2 | -1.631 | +0.091 | 0 | -1.540 | +2.97 | 52% |
| CMCSA | 90 | 83.8 | -1.448 | 0 | 0 | -1.448 | +3.26 | 44% |
| VZ | 193 | 142.3 | -1.228 | 0 | 0 | -1.228 | +6.00 | 20% |
| SO | 106 | 74.5 | -1.153 | 0 | 0 | -1.153 | +15.98 | 7% |
| NEE | 178 | 103.1 | -0.890 | 0 | 0 | -0.890 | none | n/a |
| TMUS | 181 | 86.3 | -0.771 | 0 | +0.015 | -0.756 | +3.17 | 24% |
| CVS | 115 | 60.7 | -0.949 | +0.289 | 0 | -0.660 | +2.59 | 25% |
| ORCL | 383 | 130.1 | -0.619 | +0.001 | 0 | -0.618 | +4.74 | 13% |
| UPS | 96 | 25.1 | -0.532 | 0 | +1.293 | **+0.761** | +4.22 | 18% |

Above the threshold: 29 of 86 non-financial companies. 28 are above 0.25 points, 10 are above 10% of the rates part, 9 are in both. The 29: ABBV AMGN BA BKNG BMY CMCSA COP CRM CVS DIS DUK FDX IBM LOW MCD MDLZ MDT MO NEE ORCL PEP PFE SO T TMO TMUS UNP UPS VZ. The 8 companies with no rates part (for example NEE) use the 0.25 limit only. The positive cases are pension cases: UPS +0.761 and RTX +0.180 (obligation derived, see gaps). The pension gain is gross: plan assets (bonds) lose value in the same move, so the true net is lower.

## Assumptions (every number here that is not a fact)
| # | Assumption | Value | Effect of a change |
|---|---|---|---|
| A1 | Yield for the duration; all debt is a par bond with a coupon equal to the yield; modified duration = (1 - 1.05^-T) / 0.05 | 5.0% | Bracket 4% and 6% |
| A2 | Ladder points: years 1 to 5 at 0.5 to 4.5 y; "after year five" at 15 y; when this tag is missing, the tail = carrying debt minus the five buckets (mark *) | 15 y | Bracket 10 and 25 y |
| A3 | Debt with no ladder (16 companies): one bucket | 8 y (D 6.46) | Bracket 5 and 12 y |
| A4 | All debt is fixed rate; credit spread unchanged; leases not included; no tax. No fixed or floating split is tagged for any company | 100% fixed | Overstates the debt loss |
| A5 | Securities buckets: within 1 y 0.5; 1 to 5 y 3; 5 to 10 y 7.5; after 10 y 15; rolling 2 to 5 y 3.5, 6 to 10 y 8; no buckets: one bucket | 2.5 y | Bracket 1.5 and 4 y |
| A6 | Securities base = the largest of three tag groups (debt AFS, marketable, short-term); held-to-maturity added; equity inside "marketable" not removed; `LongTermInvestments` not moved | rule | Small: the asset block is at most +0.06 points in the S&P 10 |
| A7 | Pension discount rate = tag, else 5.0%; no payments tag: Macaulay duration 11 y; tail of the obligation at 17 y; discount-rate change = Treasury change (spread unchanged); plan assets not moved | 11 y | Bracket 8 and 14 y |
| A8 | Debt at fair value only when `LongTermDebtFairValue` has the date of the debt and the ratio is 0.6 to 1.2 (21 companies); else carrying | rule | Fair value is lower than book at 5% yields: the effect is a little smaller |
| A9 | Derived obligation = plan assets - plan asset noncurrent + net liability (ABT, BMY, RTX, UPS) | rule | Mixed with OPEB |

Facts: the SEC amounts, the ladder amounts, the bucket amounts and the curve changes. Bracket result, S&P 100 / S&P 10 (all of A1, A2, A3, A5, A7 low together, then high together): -0.067 / -0.021 and -0.109 / -0.037 points. The effects of the changes do not add; the three runs are whole sets.

## Gaps (a missing tag, not a guess)
- **Debt ladder:** 16 non-financial companies, among them NVDA, LLY, PFE, LMT, BKNG, GILD, NFLX, NOW, TXN. The store has a stale ladder (for example LLY 2024, PFE 2017) or none.
- **Fair value of debt at the date:** 65 of 86. **Fixed or floating split:** all.
- **Debt not tagged:** DE and GM (finance arms), and ANET, ISRG, PANW, PLTR, SNDK, which have no debt tag. DE and GM debt is therefore missing from the sum.
- **Securities buckets:** 27 companies, among them AAPL ($101 B), TSLA ($42 B), MU ($35 B), AMGN, BA, UBER. The default 2.5 y applies.
- **Pensions:** 78 of 86 have no pension tag in the store (most tag by plan, a dimension that the facts file does not keep). The obligation is tagged for INTC, TMUS (small). It is derived for ABT, BMY, RTX, UPS. GE and LMT have plan assets only: if the obligation equals the assets, the gain is +0.16 and +0.43 points (a proxy, not in the sums). IBM, a known large plan, has no obligation tag.
- **Banks, insurers, REIT (size only, $B):** debt / securities. BAC 318.7 / 842.0; C 387.6 / none; JPM 71.3 / none; WFC 221.0 / none; GS 72.4 / 172.4; MS 378.1 / none; USB 21.0 / none; BNY 40.1 / none; SCHW 23.8 / none; COF 49.8 / none; AXP 60.0 / 0.2; AMT (REIT) 37.6 / 0. BRK-B: securities 17.1, no debt tag. The securities tags are missing for most banks: the size is a floor. Deposits and loans are the real exposure and are not tagged in this way.

## Limits
- The universe is today's S&P 100 (survivorship). One week, one curve move: a steepening at the short end and a fall at 2 to 10 years. Another move gives another sign for the cash.
- The measurement is a first order estimate (duration only, no convexity). The effects are small, so this is enough.
- Not done: a floating-rate interest effect, derivatives (swaps), preferred stock, convertible debt, operating leases, the tax effect, plan-asset allocation, no check of single company filings by hand.
- `filed` can come from a later filing (trap 4): the filter is safe, not exact. Poison and truncation tests: not run (no model code).
- Open for the Head of Fundamental Research: the model change of item 3 (bridge at market value, `long` mode). The utilities and telecoms (T, DUK, CMCSA, VZ, SO, NEE) are the first test cases. Regulated utilities pass debt cost to customers: the debt loss may not reach the equity.
