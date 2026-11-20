# NVDA research ledger (DRAFT, not reviewed)

Version of 2026-11-20. Step 1 of the review (analyst). Form of `analysis/IBM/research-ledger.md`, with one id for each fact.

Each outside fact has a date, a source, a tier and an expiry. Tier: PRIMARY = filing or XBRL fact of the SEC; SECONDARY = a data
vendor or a press source; REPO = a record or a table of this repo (CONVENTION or DATA of the calibration); JUDGMENT = the analyst, with
no source. "WAITS" = the number is old because a newer filing exists and is not in the data store.

Data store: `data/raw`, commit `41116d0` (2026-11-20). SEC company facts of 2026-11-19. Prices to 2026-11-18. GSW curve to
2026-11-13. Model date 2026-11-19. Filings in the store: 10-K of 2026-02-25 (accession 0001045810-26-000021), 10-Q of
2026-08-26 for the fiscal second quarter of 2027, period end 2026-07-26 (accession 0001045810-26-000075). All values below come
from `data/raw/NVDA/company_facts.json.gz` (tag names in the text) or from `mapcheck values NVDA --cache data/raw`, unless a
source is named. Nothing here comes from the market price.

## A. Reported company numbers (PRIMARY)

| Id | Fact | Value | As-of | Tier | Source | Expiry | Used in |
|---|---|---|---|---|---|---|---|
| L1 | Revenue, quarters (fiscal Q3 FY26, Q4 FY26, Q1 FY27, Q2 FY27) | 57.006, 68.127, 81.615, 96.221 $B; sum 302.969 $B | to 2026-07-26 | PRIMARY | `Revenues` of the 10-Q 0001045810-26-000075 and the 10-K 0001045810-26-000021. Q4 FY26 = fiscal year 215.938 less the first three quarters (44.062 + 46.743 + 57.006). | Next 10-Q (fiscal Q3 FY27) **WAITS** | `base_revenue`, growth start |
| L1a | Sequential growth | +21.9%, +19.5%, +19.8%, +17.9% (Q3 FY26 to Q2 FY27) | same | PRIMARY | calculated from L1 | same | growth start |
| L1b | Year over year growth | Q1 FY27 +85.2%, Q2 FY27 +105.9% | same | PRIMARY | calculated | same | context |
| L1c | Operating income, quarters | 36.010, 44.299, 53.536, 63.734 $B; sum 197.579 $B (65.21% of revenue). Q2 FY27 66.24%. | same | PRIMARY | `OperatingIncomeLoss`; Q4 FY26 = 130.387 less the first three quarters | same **WAITS** | margin start |
| L1d | Gross margin, quarters | Q1 FY26 60.52%, Q2 FY26 72.42%, Q3 FY26 73.41%, Q1 FY27 74.93%, Q2 FY27 74.98% | same | PRIMARY | `GrossProfit` / `Revenues` | same **WAITS** | margin context |
| L1e | Operating expense Q2 FY27 | 8.408 $B (8.7% of revenue); R&D 7.054 | 2026-07-26 | PRIMARY | `OperatingExpenses`, `ResearchAndDevelopmentExpense` | same | margin context |
| L2 | Results of the fiscal Q3 FY27 (8-K item 2.02 of 2026-11-17, accession 0001045810-26-000088) | NOT READ. Address: `www.sec.gov/Archives/edgar/data/0001045810/000104581026000088/q3fy27pr.htm`. The store holds only the list entry (`sec_8k_earnings.json`). | 2026-11-17 | PRIMARY | requested from the Head of Technology | **WAITS**: revenue, margins, the guidance for Q4 FY27, segment revenue, the 10-Q (cash, debt, securities, guarantees) | `base_revenue` (TTM), year 1, bridge |
| L3 | Revenue, operating margin, SBC ratio by fiscal year | FY17 6.91 $B, 28.0%; FY18 9.71, 33.0%; FY19 11.72, 32.5%; FY20 10.92 (-6.8%), 26.1%; FY21 16.68, 27.2%; FY22 26.91, 37.3%; FY23 26.97 (+0.2%), 15.7%; FY24 60.92 (+125.9%), 54.1%; FY25 130.50 (+114.2%), 62.4%; FY26 215.94 (+65.5%), 60.4%. Mean FY17 to FY22: 30.7%. Mean FY24 to FY26: 59.0%. | fiscal years to 2026-01-25 | PRIMARY | `mapcheck values NVDA --periods 20`, 10-K rows | Next 10-K (2027-02) | margin target, shock size, bust history |
| L4 | Invested capital (receivables + inventory + prepaid - payables and accrued + net property + operating-lease assets + other non-current assets) | 2024-01-28 18.74; 2025-01-26 29.98; 2025-07-27 39.61; 2026-01-25 53.44; 2026-07-26 91.44 $B (core without lease and other: 12.89, 25.15, 30.31, 42.27, 70.31) | per date | PRIMARY | balance sheet tags: `AccountsReceivableNetCurrent`, `InventoryNet`, `PrepaidExpenseAndOtherAssetsCurrent`, `AccountsPayableCurrent` + `AccruedLiabilitiesCurrent`, `PropertyPlantAndEquipmentNet`, `OperatingLeaseRightOfUseAsset`, `OtherAssetsNoncurrent`. `nested` of mapcheck checked: taxes payable, deferred revenue and accrued pay are inside accrued liabilities and are not added. | Next 10-Q **WAITS** | `s2c` |
| L4a | Incremental sales to capital | FY25 5.7 (core) / 6.2; FY26 5.0 / 3.6; Jul-2025 to Jul-2026 3.4 / 2.7. Trailing revenue to capital: 4.0 (FY26), 3.3 (Jul-2026). Receivables 63.059 $B on Q2 revenue 96.221 (60 days; 45 days in Q1 FY27); inventory 31.575 $B. | per date | PRIMARY | calculated from L1 and L4 | same | `s2c` |
| L5 | Effective tax rate | FY26 15.1% (21.383 / 141.450); quarters Q1 FY26 to Q2 FY27 14.3%, 15.3%, 15.9%, 14.8%, 16.6%, 16.5%; statutory 21% | to 2026-07-26 | PRIMARY | `IncomeTaxExpenseBenefit` / income before tax | Next 10-Q | tax (the model uses 21%, CONVENTION) |
| L6 | SBC | FY26 6.386 $B (3.0%); trailing four quarters 7.241 $B (2.39%); Q2 FY27 2.026 (2.11%) | to 2026-07-26 | PRIMARY | `ShareBasedCompensation` | Next 10-Q | SBC start |
| L7 | Purchases of businesses | FY25 1.007 $B; FY26 1.535 $B (`PaymentsToAcquireBusinessesNetOfCashAcquired`) plus 13.000 $B (`PaymentsToAcquireBusinessTwoNetOfCashAcquired`; goodwill +14.6 $B in Q4 FY26, 6.26 to 20.83); H1 FY27 0.298 $B. Five-year sum 15.9 $B over revenue 461 $B = 3.4%. | to 2026-07-26 | PRIMARY | company facts, 10-K 0001045810-26-000021 | Next 10-Q | `process.charge` |
| L8 | Beta | Regression beta 2.013 (standard error 0.132), 260 weekly returns 2021-11-29 to 2026-11-16 on SPY; shrunk 2/3 + 1/3 = 1.676 (standard error 0.088). R2 0.475. | 2026-11-16 | REPO (calculated from the price cache) | `fa dcf auto NVDA --explain`; the regression is repeated in the report | Each new price file | `discount.beta` |
| L9 | Shares | Diluted weighted 24,285M (Q2 FY27); cover page 24.1B on 2026-08-21 (rounded); repurchased 203M shares in H1 FY27 for 39.0 $B; repurchase authority left 99.3 $B | 2026-07-26 | PRIMARY | `WeightedAverageNumberOfDilutedSharesOutstanding`, `dei:EntityCommonStockSharesOutstanding`, `PaymentsForRepurchaseOfCommonStock` | Next 10-Q **WAITS** | `shares` |

## B. The bridge (PRIMARY, balance sheet of 2026-07-26)

| Id | Item | Value | In the bridge? | Reason (rules F1, F15) |
|---|---|---|---|---|
| L10 | Debt | 33.366 $B (`LongTermDebt`; current 1.000, non-current 32.366); new debt in H1 FY27 24.896 $B (`ProceedsFromDebtNetOfIssuanceCosts`); fair value 31.4 $B | Yes, book value, one time | The interest expense is not in the cash flow (option B: EBIT less tax less reinvestment), so the debt is subtracted one time (F1). |
| L10a | Cash | 22.443 $B (`CashAndCashEquivalentsAtCarryingValue`) | Yes | Interest income is not in the margin (the margin is the operating margin), so the cash is counted one time (F15). |
| L10b | Debt securities (marketable securities, current) | 34.143 $B (`DebtSecuritiesCurrent`). The total of available-for-sale debt securities is 46.9 $B (within one year 41.0, one to five years 5.9); the difference of 12.8 $B is in cash equivalents (inference from the maturity table: 41.0 less 28.2 within a year = 12.8). The balance sheet face of the 10-Q was not read. | Yes | Same as cash. `mapcheck` gives no value for `short_term_investments` after 2025-10-26 because the filer changed the tag (finding W7). |
| | Net debt used | 33.366 - 22.443 - 34.143 = **-23.220 $B** (net cash) | | |
| L11 | Equity securities, listed (fair value through income) | 42.783 $B (`EquitySecuritiesFvNi`; 12.886 on 2026-01-25) | No (sensitivity: +$3.74 a share with the item below, P50 134.63) | The gains (23.7 $B in H1 FY27, `GainLossOnInvestments`) are not in the margin, so the rule F15 would allow it. Excluded: (a) part of the stakes is in companies that buy NVDA products, so their value moves with the revenue that the model already credits; (b) the 17.5 $B paid in FY26 for these stakes is a use of cash that the growth needed (pairing, F16); (c) the terms are not read. |
| L11a | Equity securities, no readily known value | 47.898 $B (`EquitySecuritiesWithoutReadilyDeterminableFairValueAmount`; 22.251 on 2026-01-25); upward price adjustments 9.1 $B in total | No | Same reasons; the value is mostly the cost. |
| L12 | Guarantees, maximum exposure | 108.5 $B on 2026-07-26 (`GuaranteeObligationsMaximumExposure`); 0.86 $B on 2025-10-26 | No, **open** | The tag gives a maximum, not an expected loss; the terms are in a note of the 10-Q that was not read. If the whole amount were a claim: -$4.47 a share. The risk goes with the demand collapse (the failure table of the model), not with the bridge. WAITS for the note. |
| L13 | Operating lease liabilities | 5.494 $B | No | The lease cost is in the operating income (F15). Memo: -$0.23 a share. |
| L14 | Purchase obligations | `PurchaseObligation` 45.774 $B (2025-07-27, last value in the store); `UnrecordedUnconditionalPurchaseObligationBalanceSheetAmount` 22.7 $B (2026-01-25), 7.6 $B (2025-01-26) | No | They are payments for product that the company will sell: the cost is in the gross margin and in the inventory (a debt claim would count the cost two times). Rule R1: see section D. |

## C. Market and rate facts

| Id | Fact | Value | As-of | Tier | Source | Expiry |
|---|---|---|---|---|---|---|
| L15 | Last close of NVDA | $150.96 | 2026-11-18 | SECONDARY | `data/raw/NVDA/prices.json` (Nasdaq). Closes: 2026-09-30 228.38, 2026-10-30 204.64, 2026-11-03 206.69. The 52-week high is 239.24. The closes of 2026-11-19 and 2026-11-20 are NOT in the store. | Each close. Used only in "what the price implies". No input. |
| L16 | Zero-coupon curve | 10-year zero yield 5.13%; the six GSW numbers of 2026-11-13 are in the model | 2026-11-13 | PRIMARY (Federal Reserve Board, GSW) | `data/raw/rates/gsw_feds200628.csv` | Each new row |
| L17 | Equity risk premium | 4.41% (std 0.75%, lower bound 2%) | 2026-11-19 | REPO (CONVENTION; Damodaran implied premium) | `analysis/ANT/driver_model_paths.toml`, `docs/models/auto-path-model.md` | Next update of the premium |
| L18 | Long-run growth L | min(10-year zero yield 5.13%, 4.0%) = 4.0%; FOMC SEP 2026-09-16, 2.0% + 2.0% | 2026-09-16 | REPO (record A-001) | `docs/assumptions/A-001-terminal-growth.md` | Next SEP (December 2026) |
| L19 | Fade of fast growers | After a year of growth of 50% or more (all sizes, 669 company-years): median growth 30% (next year), 17% (2 years), 11% (10 years); 30 to 100 $B revenue (42 company-years): 12% (next year), -1% (2 years), 3% (3 years). Survivor bias. | 2026-11-19 | REPO (DATA, calibration table) | `analysis/priors/process_priors_2026-11-19.toml`, `[[fade]]` | Next calibration |

## D. Forward disclosures (rule R1) and consensus

| Id | Disclosure | Value | Decision | Reason |
|---|---|---|---|---|
| L20 | Remaining performance obligation | 3.2 $B (2026-07-26), 2.6 (2026-04-26), 2.3 (2026-01-25) (`RevenueRemainingPerformanceObligation`) | **Excluded** | 1.1% of trailing revenue and 0.6% of year-1 revenue of the model. It is for cloud and software contracts, not for the product orders. It does not bear on the growth driver. IBM had 105% coverage, and NVDA has 1%. |
| L21 | Purchase obligations and supply commitments | See L14 | **Excluded as a number; used as a plausibility check** | They show what NVDA bought, not what the customers ordered. The rise from 7.6 (FY25) to 22.7 $B (FY26) is in line with a start growth of +65%, but no growth number comes from it. |
| L22 | Guidance of the company for the next quarter (8-K of 2026-08-26, accession 0001045810-26-000073, and of 2026-11-17) | NOT READ | **Excluded because it is not in the store; WAITS** | The guidance for Q3 FY27 is now a past period. The guidance for Q4 FY27 is in the 8-K of 2026-11-17. It is the main test of the growth start (year 1, mean +65%, std 20%). After the filing is in the store, compare the guidance of Q4 with the model path (the average quarter of year 1 is $125B at the model mean, $500B for the year). |
| L23 | Earnings per share, reported, and consensus (Nasdaq) | Actual (non-GAAP) EPS: Q1 FY27 1.87 (consensus 1.70), Q2 2.22 (2.09), Q3 2.68 (2.47, reported 2026-11-17). Consensus for Q4 FY27 2.79 (14 analysts), Q1 FY28 3.25, Q2 3.71, Q3 4.10, Q4 4.79. Fiscal year EPS: FY27 9.48, FY28 15.60, FY29 21.65, FY30 26.61 (19, 18, 9, 3 analysts). No revenue estimates in the file. | 2026-11-19 | SECONDARY | `data/raw/NVDA/estimates/2026-11-19.json`, `earnings_surprises.json` | **Cross-check only, no input.** The consensus for the four quarters Q4 FY27 to Q3 FY28 is 2.79 + 3.25 + 3.71 + 4.10 = 13.85, against 8.34 for the last four reported quarters (Q4 FY26 1.57, Q1 1.87, Q2 2.22, Q3 2.68): +66% in earnings per share, with a window of one year that starts one quarter later than year 1 of the model. EPS growth is not revenue growth (margin, share count, tax and investment gains differ). The model mean for year 1 is revenue +65%. A consensus is made by people who see the price and the guidance; the model does not use it so that the result stays independent. |
| L24 | Price targets and ratings (Nasdaq) | Mean $232.59, low $130, high $350; 29 buy, 13 hold, 1 sell | 2026-11-19 | SECONDARY | same file | **Not used.** A target is not evidence for a driver; the rule of the ticket is that no input comes from a target. Shown in the report only as the view of others. |

## E. Items that were not read, and what they change

| Item | Needed for | Effect if different |
|---|---|---|
| 8-K of 2026-11-17 and the 10-Q of the fiscal Q3 FY27 (when filed) | Revenue of Q3, guidance for Q4, margin, cash and debt, securities, shares, guarantees, segment revenue | Moves `base_revenue` (TTM to 2026-10-25), the growth start, the margin start, the bridge, the shares; may allow two segments |
| 10-K FY26 and 10-Q notes (guarantees, purchase obligations, customer concentration, China) | L12, L14 | A guarantee claim of up to $4.47 a share |
| Balance sheet face of the 10-Q of 2026-07-26 | L10b | A different split of cash and securities |
| Closes of 2026-11-19 and 2026-11-20 | "what the price implies" | A price at a different date |

## F. Facts of the market that are not in the ledger on purpose

No fact about the market share of NVDA, about the spending of its customers, about competitors or about the rules of export is used as a number. Where the text of the model says that "customers build own chips" or that "a high margin attracts sellers", it is a JUDGMENT with no source. The skeptic should treat it so.
