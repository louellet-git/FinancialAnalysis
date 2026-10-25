# IBM v4.1 — July-22 watch-list grades (bet ledger, entry 1)

**As of 2026-10-22** (latest filing used: Q3 10-Q, 0000051143-26-000095). Source claims: `analysis/IBM/report/IBM-valuation-2026-07.md` §5, lines 144-148 (frozen, not edited).
Grades: CONFIRMED = stated threshold held; BROKEN = breached; UNRESOLVED = not decidable from filings to date (resolves with the FY2026 10-K, expected ~Feb 2027, unless noted).
All figures re-fetched from SEC on 2026-10-25 (GET data.sec.gov / www.sec.gov). XBRL = `data.sec.gov/api/xbrl/companyfacts/CIK0000051143.json`; Ex.99.1 = `www.sec.gov/Archives/edgar/data/51143/<accession, no dashes>/ibm-<date>xex991.htm`.

Filings: Q2 8-K 0000051143-26-000077 (filed 2026-07-22), Q2 10-Q -000078 (2026-07-23), Q3 8-K -000093 (2026-10-21), Q3 10-Q -000095 (2026-10-22); bases from Q4-25 8-K -000004 (2026-01-28) and FY25 10-K -000010 (2026-02-24).

## 1. Did the slipped deals close? — **CONFIRMED** (partial, qualitative)
Claim (l.144): "Closure is the bull gate; failure to close converts 'timing' into demand loss." No numeric threshold; graded on verbatim filing text only.
- Q2 10-Q -000078, MD&A p.47: "In the final weeks of June, we saw a shift in client spending priorities that resulted in numerous large deals failing to close within the expected timelines, driving the majority of the shortfall."
- Q3 10-Q -000095, MD&A p.52 (Software): Q3 Software revenue $7,804M, +8.2%, "led by the combination of strong new business performance, including Confluent, and pipeline closure rates as we completed a significant portion of large deals that failed to close within the expected timelines from the second quarter." Same page, Transaction Processing +9.3%: "strong execution against our pipeline, including certain large deals that failed to close from the second quarter."
- Reason: the filing says a "significant portion" closed in Q3 (bull gate met). Not quantified, not "all"; IBM Z revenue still fell 19.6% in Q3 (10-Q -000095, Infrastructure table: "reflecting product cycle dynamics"), so the Infrastructure leg of the slip is not shown closed.

## 2. FY2026 FCF guide versus ~$15.7B — **CONFIRMED** (on guidance; FY actual UNRESOLVED until FY2026 10-K)
Claim (l.145): FY2025 FCF $14.7B + "about $1B" = ~$15.7B; "A guide materially below ~$15.7B quantifies the damage" (bear).
- Base: FY2025 FCF $14,734M (Q4-25 8-K -000004, Ex.99.1 FCF reconciliation, year ended 2025-12-31). Bar = 14,734 + ~1,000 = ~$15,734M.
- Q2 8-K -000077, Ex.99.1 "Full-Year 2026 Expectations": "The company continues to expect full-year free cash flow to increase by about $1 billion year-over-year."
- Q3 8-K -000093, Ex.99.1: "The company now expects full-year free cash flow to increase by more than $1 billion year-over-year" (latest guidance, used for the grade).
- YTD (not the threshold): FCF 9M-2026 $7,787M vs $7,181M 9M-2025 (Q3 8-K -000093 reconciliation); H1 $4,760M vs $4,808M (Q2 8-K -000077). Seasonality: Q4-2025 alone was $7,553M of the $14,734M, so YTD says little about the full year.
- Reason: guide held at the bar in July and was raised above it in October; the bear condition (materially below) did not occur.

## 3. Margin at the FY2025 endpoint (19.4%) — **CONFIRMED** (YTD, same-period y/y; FY2026 margin UNRESOLVED until 10-K)
Claim (l.146): core margin 19.4% "pre-SBC, pre-M&A basis"; "July 22 tests whether even that holds."
- 19.4% is the model's own metric (`analysis/IBM/driver_model.py:122`, std 0.020), not a reported figure. It is derivable from filed XBRL: README v3 FCFF = OCF + 0.79 × (InterestExpense − InterestIncomeOther) − capex − SBC (`dcf_scenarios.py:45-48`); `core_margin` is that margin before the SBC charge (sbc_ratio 2.54% = 1,715/67,535 is charged separately), so
  **core margin = [OCF + 0.79 × (InterestExpense − InterestIncomeOther) − PaymentsToAcquirePropertyPlantAndEquipment] / Revenues.**
- Check, FY2025 (10-K -000010): [13,193 + 0.79 × (1,935 − 645) − 1,091] / 67,535 = 13,121.1 / 67,535 = **19.43%** (post-SBC 16.89%, matching README 16.9%).

| Period (XBRL, duration) | OCF | IntExp | IntInc | Capex | Revenue | Core margin |
|---|---|---|---|---|---|---|
| 6M-2026, 10-Q -000078 | 7,766 | 959 | 238 | 461 | 33,079 | 7,874.6 / 33,079 = **23.81%** |
| 6M-2025, 10-Q -000078 comparative | 6,071 | 965 | 363 | 454 | 31,519 | 6,092.6 / 31,519 = 19.33% |
| 9M-2026, 10-Q -000095 | 11,791 | 1,451 | 325 | 793 | 50,182 | 11,887.5 / 50,182 = **23.69%** |
| 9M-2025, 10-Q -000095 comparative | 9,153 | 1,457 | 513 | 709 | 47,849 | 9,189.8 / 47,849 = 19.21% |

($M. Tags: NetCashProvidedByUsedInOperatingActivities, InterestExpense, InterestIncomeOther, PaymentsToAcquirePropertyPlantAndEquipment, Revenues.)
- Seasonality caveat: YTD is not the annual endpoint. In 2025, 9M (19.21%) sat 0.22pt below FY (19.43%), but that is a coincidence of offsets: Q4-2025 carried a $4,063M financing-receivables outflow inside OCF (Q4-25 8-K -000004, Ex.99.1 FCF reconciliation) alongside strong underlying OCF. The primary comparison is therefore same-period y/y: +4.5pt at 6M and +4.5pt at 9M.
- Definition caveat: the model keeps consolidated OCF including IBM Financing receivables. The 9M-2026 OCF gain (+$2,639M y/y) includes a $1.9B y/y swing in cash from financing receivables (10-Q -000095, MD&A cash flow: "an increase in cash provided by financing receivables of $1.9 billion"). Excluding it (OCF ex-IBM-Financing-receivables from Ex.99.1 reconciliations: 9M-2026 8,986 vs 9M-2025 8,248; 6M 5,503 vs 5,465): 9M 18.10% vs 17.31% (+0.8pt); 6M 16.96% vs 17.41% (−0.5pt).
- Reason: on the model's own definition the margin is above the endpoint and prior-year YTD; on the receivables-free basis it is within ±1pt of prior year, inside the driver's 2.0pt std. No evidence of the feared decline; magnitude of the gain is largely receivables timing, so do not read 23.7% as a new level.

## 4. RPO in the Q2 10-Q — **BROKEN** (coverage below 105%; y/y change positive)
Claim (l.147): FY2025 RPO $71.0B = 105.1% of revenue; "coverage holds at or above 105% and ... the year-over-year change stays positive — read net of acquisition effects."
- Coverage = RPO / trailing-twelve-month revenue; TTM = FY2025 revenue + YTD-2026 − YTD-2025 (XBRL `Revenues`: FY2025 67,535 [10-K -000010]; 6M 33,079 / 31,519 [-000078]; 9M 50,182 / 47,849 [-000095]). RPO = XBRL `RevenueRemainingPerformanceObligation` (instant; rounded to $1B), matching the 10-Q RPO note.

| Date (filing) | RPO | TTM revenue | Coverage | RPO y/y | Prior-year same-date coverage |
|---|---|---|---|---|---|
| 2025-12-31 (10-K -000010) | $71B | 67,535 | 105.1% | +12.7% (report) | — |
| 2026-06-30 (10-Q -000078) | $68B | 67,535 + 33,079 − 31,519 = 69,095 | 98.4% | 68/66 − 1 = **+3.0%** | 66 / (62,753 + 31,519 − 30,231 = 64,041) = 103.1% |
| 2026-09-30 (10-Q -000095) | $69B | 67,535 + 50,182 − 47,849 = 69,868 | **98.8%** | 69/64 − 1 = **+7.8%** | 64 / (62,753 + 47,849 − 45,199 = 65,403) = 97.9% |

(2025 RPO: $66B at 06-30 [10-Q 0000051143-25-000052], $64B at 09-30 [10-Q 0000051143-25-000064]; 2024 revenue: 10-K -000010 / 10-Qs -24-000039, -24-000049.)
- Text (10-Q notes): 06-30 "approximately 69 percent ... in the subsequent two years" (report: ~67% at FY2025); 09-30 "approximately 70 percent".
- Rounding: $1B rounding moves coverage by about ±0.7pt; the 6-7pt shortfall to 105% exceeds it.
- Acquisition effects: the RPO notes state none. Confluent closed 2026-03-17 (10-Q -000095, note 5) and its obligations, if any, sit in the 06-30 and 09-30 RPO unquantified, so the y/y gain cannot be shown organic.
- Reason: the coverage leg is breached at both interim dates (98.4%, 98.8%). Seasonality: RPO peaks at year-end (Sep-25 coverage was 97.9%, Dec-25 105.1%), so a 105% test at an interim date is harsh; y/y the Q3 coverage is +0.9pt, Q2 is −4.7pt. The y/y-change leg held (positive, gross of acquisitions). Conjunctive claim, one leg breached: BROKEN.

## 5. Segment actuals versus the year-1 pins — **UNRESOLVED** (full-year pins; YTD run-rate favorable on all three, resolves at FY2026 10-K)
Claim (l.148): "Software +5%, Consulting flat, Infrastructure ~$14.6B implied for the year. Software materially below +5% undercuts the g_software mean from above; Infrastructure above the pin suggests the cycle haircut was too harsh."
- Pins: Software 29,962 × 1.05 = 31,460; Consulting 21,055 flat; Infrastructure 15,718 × 0.93 = 14,618 (`driver_model.py:62, 67`). FY2025 bases: Q4-25 8-K -000004, Ex.99.1 segment table (Software 29,962; Consulting 21,055; Infrastructure 15,718; Q4-25: 9,031 / 5,349 / 5,132).
- Filed ($M; 10-Q MD&A segment tables; reported growth):

| Segment | Q2-26 (-000078) | H1-26 | Q3-26 (-000095) | 9M-26 (9M-25) | 9M y/y | Q4 needed to hit pin (vs Q4-25) |
|---|---|---|---|---|---|---|
| Software | 7,761 (+5.1%) | 14,813 (+7.9%) | 7,804 (+8.2%) | 22,617 (20,932) | +8.1% | 31,460 − 22,617 = 8,843 (−2.1%) |
| Consulting | 5,327 (+0.2%) | 10,599 (+2.1%) | 5,335 (+0.2%) | 15,934 (15,706) | +1.5% | 21,055 − 15,934 = 5,121 (−4.3%) |
| Infrastructure | 3,835 (−7.4%) | 7,161 (+1.9%) | 3,707 (+4.2%) | 10,868 (10,586) | +2.7% | 14,618 − 10,868 = 3,750 (−26.9%) |

- Sub-grades on the YTD run-rate (not full-year): Software CONFIRMED (+8.1% vs +5% pin; the bear reading "materially below +5%" not met); Consulting CONFIRMED (+1.5% vs flat); Infrastructure above pin pace (74.4% of the pin level through Q3; the §5 reading "haircut too harsh" is pointing that way, but Q4-25 was a $5,132M quarter, and the FY level is not set).
- Item grade UNRESOLVED because all three pins are full-year and Q4 is unreported; the Q4 column shows how far Q4 would have to fall to reach each pin. Manager may split the item into three entries if the ledger wants one grade per pin.

## Summary

| Item | Grade | Figure (latest filing) |
|---|---|---|
| 1 Slipped deals | CONFIRMED | Q3 10-Q: "completed a significant portion of large deals that failed to close ... from the second quarter" |
| 2 FY26 FCF guide | CONFIRMED | Q3 8-K: "more than $1 billion" increase vs $14,734M FY25 (bar ~$15.7B) |
| 3 Core margin 19.4% | CONFIRMED | 9M-26 23.69% vs 9M-25 19.21% (model definition; 18.10% vs 17.31% ex-financing receivables) |
| 4 RPO coverage | BROKEN | 98.8% ($69B / $69,868M TTM) at 09-30 vs 105% bar; y/y +7.8% |
| 5 Segment pins | UNRESOLVED | 9M: Software +8.1%, Consulting +1.5%, Infrastructure $10,868M (74% of $14,618M pin) |
