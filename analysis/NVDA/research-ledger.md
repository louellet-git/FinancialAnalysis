# NVDA research ledger (DRAFT, not reviewed)

Version 5 of 2026-11-21. Step 1 of the review (analyst). Version 1 (commit 9a2e6f0) had the base of 2026-07-26. Version 2 added the 8-K. Versions 3 and 4 changed the bridge and the charge. Form of `analysis/IBM/research-ledger.md`, with one id for each fact.
`analysis/IBM/research-ledger.md`, with one id for each fact.

Each outside fact has a date, a source, a tier and an expiry. Tier: PRIMARY = filing or XBRL fact of the SEC; COMPANY 8-K = the
press release of the company, filed as an 8-K, unaudited; SECONDARY = a data vendor or a press source; REPO = a record or a table of
this repo (CONVENTION or DATA of the calibration); JUDGMENT = the analyst, with no source. "WAITS" = the number is old because a
newer filing exists or will exist, and it is not in the data store.

Data store: `data/raw`, commit `1804f48` (2026-11-21; the NVDA facts, prices and estimates are those of `a1dffb5`; the store adds the 8-K earnings releases of 21 quarters, Q4 FY22 to Q3 FY27, in `NVDA/filings/8k/`).
SPY prices to 2026-11-18. GSW curve to 2026-11-13. The close of 2026-11-20 is NOT in the store (the source lags one day). Model
date 2026-11-20. Filings in the store: 10-K of 2026-02-25 (accession 0001045810-26-000021) and the 10-Q of 2026-08-26 for the fiscal
second quarter of 2027, period end 2026-07-26 (accession 0001045810-26-000075). **The 10-Q of the fiscal third quarter of 2027
(quarter end 2026-10-25) is NOT filed.** Its numbers come from the 8-K press release of 2026-11-17 (section A2).

Values in sections A, B, C come from `data/raw/NVDA/company_facts.json.gz` (tag names in the text) or from `mapcheck values NVDA
--cache data/raw`, unless a source is named. Nothing here comes from the market price.

## A. Reported company numbers, filed (PRIMARY, to 2026-07-26)

| Id | Fact | Value | As-of | Tier | Source | Expiry | Used in |
|---|---|---|---|---|---|---|---|
| L1 | Revenue, quarters | Q4 FY26 68.127, Q1 FY27 81.615, Q2 FY27 96.221 $B (Q3 FY26 57.006). Q4 FY26 = fiscal year 215.938 less the first three quarters (44.062 + 46.743 + 57.006). | to 2026-07-26 | PRIMARY | `Revenues` of the 10-Q 0001045810-26-000075 and the 10-K 0001045810-26-000021 | Next 10-Q (Q3 FY27) | base revenue (with L2a) |
| L1a | Sequential growth | +22.0% (Q3 FY26), +19.5% (Q4), +19.8% (Q1 FY27), +17.9% (Q2), +20.4% (Q3 FY27, with L2a) | to 2026-10-25 | PRIMARY + 8-K | calculated | same | growth start |
| L1c | Operating income and margin | Q4 FY26 44.299, Q1 53.536, Q2 63.734 $B (66.24% of revenue in Q2); with Q3 (L2a) the trailing four quarters are 239.089 $B, 66.08% | same | PRIMARY + 8-K | `OperatingIncomeLoss`; Q4 FY26 = 130.387 less the first three quarters | same | margin start |
| L1d | Gross margin | Q1 FY26 60.52%, Q2 FY26 72.42%, Q3 FY26 73.41%, Q1 FY27 74.93%, Q2 FY27 74.98%, Q3 FY27 74.8% (8-K) | same | PRIMARY + 8-K | `GrossProfit` / `Revenues` | same | margin context |
| L3 | Revenue, operating margin by fiscal year | FY17 6.91 $B, 28.0%; FY18 9.71, 33.0%; FY19 11.72, 32.5%; FY20 10.92 (-6.8%), 26.1%; FY21 16.68, 27.2%; FY22 26.91, 37.3%; FY23 26.97 (+0.2%), 15.7%; FY24 60.92 (+125.9%), 54.1%; FY25 130.50 (+114.2%), 62.4%; FY26 215.94 (+65.5%), 60.4%. Mean FY17 to FY22: 30.7%. Mean FY24 to FY26: 59.0%. | to 2026-01-25 | PRIMARY | `mapcheck values NVDA --periods 20`, 10-K rows | Next 10-K (2027-02) | margin target, shock size, bust history |
| L4 | Invested capital (receivables + inventory + prepaid - payables and accrued + net property + operating-lease assets + other non-current assets) | 2024-01-28 18.74; 2025-01-26 29.98; 2025-07-27 39.61; 2026-01-25 53.44; 2026-07-26 91.44 $B (core without lease and other: 12.89, 25.15, 30.31, 42.27, 70.31) | per date | PRIMARY | balance sheet tags (`AccountsReceivableNetCurrent`, `InventoryNet`, `PrepaidExpenseAndOtherAssetsCurrent`, `AccountsPayableCurrent` + `AccruedLiabilitiesCurrent`, `PropertyPlantAndEquipmentNet`, `OperatingLeaseRightOfUseAsset`, `OtherAssetsNoncurrent`). `nested` of mapcheck checked: taxes payable, deferred revenue and accrued pay are inside accrued liabilities and are not added. | Next 10-Q. The page has the balance sheet of 2026-10-25 (L2f): invested capital 112.7 $B (core 89.1); incremental ratio 2025-10-26 to 2026-10-25: 3.0 (3.6 core); trailing revenue to capital 3.2; receivables 63.7 days of Q3 revenue | `s2c` (start 3.0 confirmed) |
| L4a | Incremental sales to capital | FY25 5.7 (core) / 6.2; FY26 5.0 / 3.6; Jul-2025 to Jul-2026 3.4 / 2.7. Trailing revenue to capital 4.0 (FY26), 3.3 (Jul-2026). Receivables 63.059 $B on Q2 revenue 96.221 (60 days; 45 days in Q1 FY27); inventory 31.575 $B. | per date | PRIMARY | calculated from L1 and L4 | same | `s2c` |
| L5 | Effective tax rate | FY24 12.0%; FY25 13.3%; FY26 15.1% (21.383 / 141.450); quarters Q1 FY26 to Q2 FY27 14.3%, 15.3%, 15.9%, 14.8%, 16.6%, 16.5%; Q3 FY27 17.4% (13.730 / 79.017, L2b); statutory 21% | to 2026-10-25 | PRIMARY + 8-K | `IncomeTaxExpenseBenefit` / income before tax | Next 10-Q | tax (the model keeps 21%, CONVENTION, with the reason in the model file) |
| L6 | SBC | FY26 6.386 $B (3.0%); Q4 FY26 1.633, Q1 1.928, Q2 2.026 $B (2.11%); the three quarters: 5.587 / 245.963 = 2.27%. Q3 FY27: 2.088 (L2c; page). Trailing four quarters 7.675 / 361.807 = 2.12%; Q3 alone 1.80%. | to 2026-10-25 | PRIMARY + 8-K | `ShareBasedCompensation`; page, cash flow statement | Next 10-Q | SBC start |
| L7 | Purchases of businesses | FY25 1.007 $B; FY26 1.535 (`PaymentsToAcquireBusinessesNetOfCashAcquired`) plus 13.000 (`PaymentsToAcquireBusinessTwoNetOfCashAcquired`; goodwill +14.6 in Q4 FY26); H1 FY27 0.298. Five-year sum to FY26 15.9 / 461 = 3.4% of revenue. For the nine months of FY27 see L2e. | to 2026-07-26 | PRIMARY | company facts, 10-K 0001045810-26-000021 | Next 10-Q | `process.charge` |
| L8 | Beta | Regression beta 2.021 (standard error 0.132), 260 weekly returns 2021-11-29 to 2026-11-16 on SPY (SPY ends 2026-11-18 in the store; the weekly returns end 2026-11-16 in `fa dcf auto`); shrunk 2/3 + 1/3 = 1.681 (standard error 0.088). R2 0.477. Version 1: 2.013, 1.676. | 2026-11-16 | REPO (calculated from the price cache) | `fa dcf auto NVDA --cache data/raw --as-of 2026-11-20 --explain` | Each new price file | `discount.beta` |

## A2. The 8-K press release of 2026-11-17 (fiscal Q3 FY27, quarter end 2026-10-25)

Source for all rows of this section: 8-K item 2.02, exhibit 99.1, accession 0001045810-26-000088,
`https://www.sec.gov/Archives/edgar/data/1045810/000104581026000088/q3fy27pr.htm`. **Version 3: the analyst fetched the page
(one `curl` request, HTTP 200, 348,168 bytes, 2026-11-21, User-Agent of `sec_contact.toml`; the file is in the scratchpad of the
session and is not in the repo) and traced each row to the table or line of the page.** Tier: company 8-K, unaudited. Expiry: when
the 10-Q of Q3 FY27 is filed (then check each row against it). Amounts in $ millions in the source. Result of the trace: every
number of version 2 is on the page and agrees. One description was wrong and is corrected: the line "Groq, Inc." is in the
FINANCING section of the cash flow statement (nine months only, none in Q3), not an acquisition line of the investing section.
New numbers taken from the page: SBC of Q3, the balance sheet lines for `s2c`, the purchases of equity securities (nine months),
the proceeds from sales of equity securities, the weighted shares, the repurchase authority, the basic shares.

| Id | Fact | Value | Used in |
|---|---|---|---|
| L2 | Status | The 10-Q of Q3 FY27 is not filed. The store has the list entry of the 8-K only (`sec_8k_earnings.json`). | all rows below |
| L2a | Revenue Q3 FY27 | 115,844 (Q2 FY27 96,221; Q3 FY26 57,006): +20% q/q, +103% y/y (calculated +20.4%). Data Center 107.6B (+21% q/q, +110% y/y). Edge Computing 8.2B (+14% q/q, +42% y/y). No other segment split and no history of the two segments. Trailing four quarters to 2026-10-25: 68.127 + 81.615 + 96.221 + 115.844 = **361.807 $B**. Nine months FY27 293,681 (FY26 147,811). **Traced:** page, "Condensed consolidated statements of income", line Revenue (three months 115,844 / 57,006; nine months 293,681 / 147,811); summary table (Q2 $96.2B); bullets "Highlights", Data Center and Edge Computing. | base revenue |
| L2b | Income Q3 FY27, GAAP | Gross margin 74.8% (Q2 75.0%). Operating expenses 9,156 (R&D 7,454; SG&A 1,702). Operating income 77,520 (Q2 63,734): 66.92% of revenue. Other income, net 1,497. Income before tax 79,017. Tax 13,730 (17.4%). Net income 65,287. Diluted EPS GAAP $2.70 (non-GAAP $2.68; Q2 GAAP $2.46, non-GAAP $2.22; Q3 FY26 GAAP $1.30). Cost of revenue 29,168. **Traced:** page, income statement (all lines), summary tables, GAAP to non-GAAP reconciliation. | margin start, tax |
| L2c | Cash flow Q3 FY27 (nine months) | Operating 48,253 (122,675). Purchases related to property and equipment and intangible assets (4,422) ((8,856)). Repurchases of common stock (22,122) ((61,166)). Dividends (6,026) ((12,316)). Proceeds related to issuance of debt, net: 24,896 for the nine months, none in Q3. Repayment of debt (1,000) in Q3. SBC 2,088 (6,042). Depreciation and amortization 1,254 (3,378). Free cash flow as the company defines it: 43,777 (113,672). The company returned $28.1B to holders in Q3. With Q4 FY26 (OCF 36.188, capex 1.284 from L1 tags) the trailing free cash flow is 158.863 - 10.140 = 148.723 $B, 41.1% of trailing revenue. **Traced:** page, cash flow statement (lines named) and the reconciliation table (free cash flow). | margin cross-check; SBC |
| L2d | Outlook Q4 FY27 (**forward disclosure, rule R1: used**) | Quarter of 14 weeks ("includes an additional week"). Revenue "$130.0 billion, plus or minus 3%". "NVIDIA is not assuming any Data Center compute revenue from China in its outlook." GAAP and non-GAAP gross margins 71.0% to 72.0%. GAAP operating expenses about $11.0 billion (non-GAAP $10.7B). Full-year FY27 GAAP and non-GAAP tax rates 16.0% to 18.0%, excluding discrete items. In 13 weeks: 130.0 x 13 / 14 = 120.7 $B, +4.2% on Q3 (reported +12.2%). **Traced:** page, section "Outlook" (four bullets) and the table "Reconciliation of GAAP to non-GAAP outlook". | growth start, margin start, tax |
| L2e | Purchases of businesses and stakes | Investing activities: "Acquisitions, net of cash acquired and other" (2,718) in Q3 and (3,031) in nine months (nine months FY26 (1,370)). **Financing activities: "Groq, Inc." (2,944) in nine months, none in Q3** (corrected from version 2). Purchases of equity securities: **(15,696) in Q3, (58,100) in nine months** (nine months FY26 (4,952)). Proceeds from sales of equity securities 2,253 in Q3, 9,493 in nine months. Purchases of debt securities (33,191) nine months; proceeds from sales and maturities of debt securities 47,821. Gains from equity securities, net (1,767) in Q3, (25,474) in nine months (non-cash, removed from operating cash flow). Pending acquisition of Hugging Face ("Highlights"; no amount). Four-quarter purchases of equity securities: Q4 FY26 12.80 (company facts: fiscal 2026 17.502 less nine months 4.702; the page shows 4.952 for the same nine months, a basis difference of 0.25) + 58.100 = 70.90 $B = 19.6% of trailing revenue. **Traced:** page, cash flow statement, lines named. | charge; bridge decision |
| L2f | Balance sheet 2026-10-25 | Cash and cash equivalents 29,089. Marketable debt securities 24,423. Marketable equity securities 44,219. Non-marketable securities 65,528. Accounts receivable 81,033. Inventories 39,724. Prepaid and other current assets 3,506. Property and equipment, net 19,227. Operating lease assets 6,383. Goodwill 26,451. Intangible assets 4,056. Deferred income tax assets 11,383. Other assets 17,166. Total assets 372,188. Accounts payable 21,074. Accrued and other current liabilities 33,287. Short-term debt: none (999 at 2026-01-25). Long-term debt 32,371. Long-term operating lease liabilities 5,777. Other long-term liabilities 13,013. Total liabilities 105,522. Shareholders' equity 266,666. Weighted diluted shares Q3 24,186 million (basic 24,106; Q2 diluted 24,285). Repurchase authority left about $227.3B after an increase of $150B in September 2026. Non-GAAP figures no longer exclude SBC from Q1 FY27. **Traced:** page, "Condensed consolidated balance sheets" (all lines), income statement (shares), text paragraph on the repurchase authorization, and the sentence "Beginning in the first quarter of fiscal 2027, NVIDIA's non-GAAP financial measures no longer exclude stock-based compensation expense" (section "Non-GAAP Measures"). **INFERENCE 1 (not on the page):** the line "Marketable debt securities" (24,423) is the same item as the XBRL tag `DebtSecuritiesCurrent` (34,143 on 2026-07-26) and is cash-like; the page does not say so. | bridge, shares, `s2c` |
| L2g | H20 charges | "Includes H20 charges/(releases), net, which were $4.3 billion and insignificant for the first nine months of fiscal years 2026 and 2027". The footnote gives the nine-month totals only. **INFERENCE 2 (not on the page):** that the $4.3B is the cause of the gross margin of 60.5% in Q1 FY26 (L1d); the page does not present Q1 FY26. **Traced:** page, footnote of the reconciliation (the totals). | context |
| L2i | Items for the 10-Q (open) | Footnote (B) of the reconciliation: "Comprised of impact related to equity derivatives, equity method investments, convertible debt, dividend income, and interest expense related to accrued purchase consideration." So the company has **convertible debt** and **accrued purchase consideration** (a liability for acquisitions, probably in "Other long-term liabilities" 13,013 or in accrued liabilities). "Losses from derivatives, net" 492 in Q3 (829 in nine months). The balance sheet of the page has no line for them. **Open for the 10-Q:** the amounts, and whether the debt of 32,371 includes the convertible debt. **Traced:** page, footnote (B) and the cash flow line. | bridge (possible claims not in net debt) |
| L2h | Governance items on the page (not in the model) | The forward-looking statements name: "NVIDIA's leadership transition and the search for a new chief executive officer", and "the ongoing investigation by an independent committee of NVIDIA's board of directors". Colette Kress is interim CEO and CFO. The text gives no detail. **Traced:** page, forward-looking statements. | weak point only; no input |

No model input comes from the quote of the press release.

## B. The bridge (balance sheet of 2026-10-25, L2f)

| Id | Item | Value | In the bridge? | Reason (rules F1, F15, F16) |
|---|---|---|---|---|
| L10 | Debt | 32.371 $B long-term; no short-term debt (07-26: 33.366). Debt issued in the nine months 24.896 $B. | Yes, book value, one time | Interest expense is not in the cash flow (option B: EBIT less tax less reinvestment), so the debt is subtracted one time (F1). |
| L10a | Cash | 29.089 $B | Yes | Interest income is not in the margin (the margin is the operating margin), so the cash is counted one time (F15). |
| L10b | Marketable debt securities | 24.423 $B (07-26: 34.143 `DebtSecuritiesCurrent`) | Yes | Cash-like; same as cash. `mapcheck` gives no value for `short_term_investments` after 2025-10-26 (filer tag change, finding W7). |
| | **Net debt used** | 32.371 - 29.089 - 24.423 = **-21.141 $B** (net cash; $0.87 a share) | | Version 1: -23.220 |
| L11 | Marketable equity securities | 44.219 $B (07-26: 42.783) | **Yes, at FULL fair value** | Decision of the Head of Fundamental Research: they have observable marks and net gains of $25.5B in nine months (`Gains from equity securities, net` (25,474), page cash flow statement) and no impairment; r does not apply to them. Gains are not in the margin (F15). |
| L11a | Non-marketable securities | 65.528 $B (07-26: 47.898; 10-K: 22.251) | **Yes, at r x carrying value, r = 0.5** | Decision of the Head of Fundamental Research: ONE r for the non-marketable stakes and for the future purchases. A purchase is partly an investment that keeps its value (share r) and partly a subsidy to buyers that buys credited revenue (share 1 - r). The carrying value is mostly the cost (upward adjustments 9.1 $B on 2026-07-26, `EquitySecuritiesWithoutReadilyDeterminableFairValueUpwardPriceAdjustmentCumulativeAmount`). The terms are in the 10-Q notes, not read. |
| | Equity securities together | 109.747 $B ($4.54 a share at r = 1) | **In the bridge at 44.219 + r x 65.528 = 77.0 $B at r = 0.5 ($3.18 a share).** Net debt used: -21.141 - 44.219 - 32.764 = **-98.124 $B** | The future purchases are charged at (1 - r) of the four-quarter rate (process.charge, part A), with the fade of the growth (same half-life as D3). There are no terms and no data for r: The value 0.5 means that the value is not known. The range of r carries the uncertainty (report section 1: r = 0, 0.25, 0.5, 0.75, 1). r = 1: all stakes in at full value, no equity charge. r = 0: non-marketable stakes out, full charge. Earlier versions: version 2 r = 0 for the bridge and no charge; version 3 r = 1 for the bridge and r = 0 for the charge; version 4 r on all stakes (net debt -76.015). **The engine cannot impair the bridge on a failed path**, so the collapse paths keep the stakes at $3.18 a share (the P10 is biased up by up to about $3.2). |
| L12 | Guarantees, maximum exposure | 108.5 $B on 2026-07-26 (`GuaranteeObligationsMaximumExposure`); 0.86 $B on 2025-10-26. The 2026-10-25 value is not in the press release. | No, **open** | The tag gives a maximum, not an expected loss. If the whole amount were a claim: -$4.49 a share (24.186B shares). The note of the 10-Q WAITS. The risk goes with the demand collapse of the model (failure table), not with the bridge. |
| L13 | Operating lease liabilities | 5.494 $B (2026-07-26) | No | The lease cost is in the operating income (F15). Memo: -$0.23 a share. |
| L14 | Purchase obligations | `PurchaseObligation` 45.774 $B (2025-07-27, last value in the store); `UnrecordedUnconditionalPurchaseObligationBalanceSheetAmount` 22.7 $B (2026-01-25), 7.6 $B (2025-01-26) | No | Payments for product that the company will sell: the cost is in the gross margin and the inventory (a debt claim would count it two times). Rule R1: section D. |
| L9 | Shares | Diluted weighted 24,186M (Q3 FY27, L2f); 24,285M in Q2. Repurchased 203M shares in H1 FY27 for 39.0 $B (`PaymentsForRepurchaseOfCommonStock`); 61.166 $B in nine months (L2c). | Yes, diluted count | Existing awards are in the count; new awards are the SBC cost (F5). Cover page count (10-Q Q2) 24.1B on 2026-08-21. The period-end count is lower than the Q3 weighted count after the buyback of $22.1B in Q3 (under 1%; the weighted count is the conservative choice). Repurchase authority left $227.3B (page). |

## C. Market and rate facts

| Id | Fact | Value | As-of | Tier | Source | Expiry |
|---|---|---|---|---|---|---|
| L15 | Last close of NVDA in the store | **$145.98** (open 152.915, high 158.13, low 145.71, volume 322.9M). Earlier closes: 2026-11-18 150.96; 2026-11-17 155.46; 2026-11-03 206.69; 2026-10-30 204.64; 2026-09-30 228.38. The 52-week high close is 239.24 (2026-10-06); the intraday high is 243.37 on the same day. **The close of 2026-11-20 is not in the store.** | 2026-11-19 | SECONDARY | `data/raw/NVDA/prices.json` (Nasdaq; `prices.meta.json` last date 2026-11-19) | Each close. Used only in "what the price implies". No input. |
| L16 | Zero-coupon curve | 10-year zero yield 5.13%; the six GSW numbers of 2026-11-13 are in the model | 2026-11-13 | PRIMARY (Federal Reserve Board, GSW) | `data/raw/rates/gsw_feds200628.csv` | Each new row |
| L17 | Equity risk premium | 4.41% (std 0.75%, lower bound 2%) | 2026-11-19 | REPO (CONVENTION; Damodaran implied premium) | `analysis/ANT/driver_model_paths.toml`, `docs/models/auto-path-model.md` | Next update of the premium |
| L18 | Long-run growth L | min(10-year zero yield 5.13%, 4.0%) = 4.0%; FOMC SEP 2026-09-16, 2.0% + 2.0% | 2026-09-16 | REPO (record A-001) | `docs/assumptions/A-001-terminal-growth.md` | Next SEP (December 2026) |
| L19 | Fade of fast growers | After a year of growth of 50% or more (all sizes, 669 company-years): median growth 30% (next year), 17% (2 years), 11% (10 years); 30 to 100 $B revenue (42 company-years): 12% (next year), -1% (2 years), 3% (3 years). Survivor bias. | 2026-11-19 | REPO (DATA, calibration table) | `analysis/priors/process_priors_2026-11-19.toml`, `[[fade]]` | Next calibration |

## D. Forward disclosures (rule R1) and consensus

| Id | Disclosure | Value | Decision | Reason |
|---|---|---|---|---|
| L20 | Remaining performance obligation | 3.2 $B (2026-07-26), 2.6 (2026-04-26), 2.3 (2026-01-25) (`RevenueRemainingPerformanceObligation`). The value at 2026-10-25 is not in the press release. | **Excluded** | 1.1% of the trailing revenue of version 1, under 1% now. It is for cloud and software contracts, not for the product orders. It does not bear on the growth driver. IBM had 105% coverage, and NVDA has about 1%. |
| L21 | Purchase obligations and supply commitments | See L14 | **Excluded as a number; plausibility check only** | They show what NVDA bought, not what the customers ordered. |
| L22 | Guidance of the company: outlook for Q4 FY27 (L2d) | Revenue $130.0B +-3% (14 weeks, 120.7 in 13 weeks); GAAP gross margin 71.0% to 72.0%; GAAP opex about $11.0B; tax 16% to 18% for FY27; no China data center compute revenue assumed | **USED** | Evidence for D2, D3 and D6. The notes are below the table. |
| L23 | EPS reported and consensus (Nasdaq) | Actual (non-GAAP; SBC is no longer excluded from Q1 FY27) EPS: Q4 FY26 1.57 (consensus 1.45), Q1 FY27 1.87 (1.70), Q2 2.22 (2.09), Q3 2.68 (2.47, reported 2026-11-17). GAAP diluted EPS of Q3 is 2.70 (L2b). Consensus of 2026-11-20: Q4 FY27 2.84 (15 analysts), Q1 FY28 3.27, Q2 3.77, Q3 4.33, Q4 4.92; fiscal year EPS FY27 9.60, FY28 16.09, FY29 22.18, FY30 26.90 (18, 18, 9, 3 analysts). No revenue estimates in the file. | **Cross-check only, no input.** The consensus for the four quarters Q4 FY27 to Q3 FY28 is 2.84 + 3.27 + 3.77 + 4.33 = 14.21, against 8.34 for the last four reported quarters: +70% in EPS. The model has revenue +40% for year 1 with a margin that falls. The street therefore expects a growth of the quarters after Q4 that is far above the +4% of the guide. The model does not follow it: the only evidence in the store for those quarters is the guide. This is the largest open difference with the street. |
| L24 | Price targets and ratings (Nasdaq) | Mean $232.59, low $130, high $350; 29 buy, 13 hold, 1 sell | 2026-11-20 | SECONDARY | `data/raw/NVDA/estimates/2026-11-20.json` | **Not used.** A target is not evidence for a driver. Shown only as the view of others. |

Notes on L22 (the guide is used):
- The guide is evidence for the growth start (+4.2% on Q3 in 13 weeks; reported +12.2%), the margin start (GAAP operating margin about 63.0%) and the half-life (the sequential growth falls from +20% to +4%).
- The guide sets no number alone.
- The base of year-1 growth is 44.9%: the guide (41.0%) plus half the median beat of all pairs, by the rule of L26.
- The record of the guide (L26): 18 beats in 19 quarters, median +5.57%, range -17.2% to +22.8%.
- Sensitivity (100,000 paths, P50): the guide $98.60; the base $103.78; the full median (48.9%) $109.23; a miss like Q2 FY23 (16.7%) $72.86.
- For the order of the decision and the result see weak point W1 of the report.

### L26. Guide against result: the 19 pairs (revenue, $ billions)

Source: the 8-K earnings releases (EX-99.1) of 21 quarters in the store (store `1804f48`; `data/raw/NVDA/filings/8k/<accession>/q<N>fy<YY>pr.htm`; tier company 8-K, unaudited; expiry: when the next quarter is reported, add a pair). Guide = the sentence "Revenue is expected to be $X billion, plus or minus Y%" (the oldest releases say "Y percent") in the "Outlook" of the release of the quarter before; reported = the line "Revenue" of the income statement of the release of the quarter itself. Beat = reported / guide - 1. A pair needs a guide and the next result: the store has 20 final releases with a guide (Q4 FY22 to Q3 FY27), the guide of the last one (Q4 FY27) has no result, so there are **19 pairs** (the coordinator counted 20 and 8 before Q4 FY24; the store gives 19 and 7). Method: `COMMANDS.md`, section 3.

**The preliminary release of 2022-08-08** (`0001045810-22-000133/pressrelease.htm`) is a PRELIMINARY result of Q2 FY23 ("approximately $6.70 billion, down 19% sequentially ... weaker than forecasted Gaming revenue", and about $1.32B of charges for inventory). It is NOT used. The reported value of Q2 FY23 is from the final release of 2022-08-24 (`0001045810-22-000136/q2fy23pr.htm`, $6.704B). The guide of that quarter was $8.10B in the release of Q1 FY23 (`0001045810-22-000073`). The miss was pre-announced.

| Quarter | Guide in the release of | Guide $B (+-2%) | Reported $B (release) | Beat |
|---|---|---|---|---|
| Q1 FY23 | Q4 FY22 (`0001045810-22-000008`) | 8.10 | 8.288 (`0001045810-22-000073`) | +2.32% |
| Q2 FY23 | Q1 FY23 (`0001045810-22-000073`) | 8.10 | 6.704 (`0001045810-22-000136`; preliminary 6.70 on 2022-08-08 not used) | **-17.23%** |
| Q3 FY23 | Q2 FY23 (`0001045810-22-000136`) | 5.90 | 5.931 (`0001045810-22-000163`) | +0.53% |
| Q4 FY23 | Q3 FY23 (`0001045810-22-000163`) | 6.00 | 6.051 (`0001045810-23-000014`) | +0.85% |
| Q1 FY24 | Q4 FY23 (`0001045810-23-000014`) | 6.50 | 7.192 (`0001045810-23-000087`) | +10.65% |
| Q2 FY24 | Q1 FY24 (`0001045810-23-000087`) | 11.00 | 13.507 (`0001045810-23-000171`) | +22.79% |
| Q3 FY24 | Q2 FY24 (`0001045810-23-000171`) | 16.00 | 18.120 (`0001045810-23-000225`) | +13.25% |
| Q4 FY24 | Q3 FY24 (`0001045810-23-000225`) | 20.00 | 22.103 (`0001045810-24-000028`) | +10.52% |
| Q1 FY25 | Q4 FY24 (`0001045810-24-000028`) | 24.0 | 26.044 (`0001045810-24-000113`) | +8.52% |
| Q2 FY25 | Q1 FY25 (`0001045810-24-000113`) | 28.0 | 30.040 (`0001045810-24-000262`) | +7.29% |
| Q3 FY25 | Q2 FY25 (`0001045810-24-000262`) | 32.5 | 35.082 (`0001045810-24-000315`) | +7.94% |
| Q4 FY25 | Q3 FY25 (`0001045810-24-000315`) | 37.5 | 39.331 (`0001045810-25-000021`) | +4.88% |
| Q1 FY26 | Q4 FY25 (`0001045810-25-000021`) | 43.0 | 44.062 (`0001045810-25-000115`) | +2.47% |
| Q2 FY26 | Q1 FY26 (`0001045810-25-000115`) | 45.0 (the guide reflects a loss of H20 revenue of about $8.0B) | 46.743 (`0001045810-25-000207`) | +3.87% |
| Q3 FY26 | Q2 FY26 (`0001045810-25-000207`) | 54.0 | 57.006 (`0001045810-25-000228`) | +5.57% |
| Q4 FY26 | Q3 FY26 (`0001045810-25-000228`) | 65.0 | 68.127 (`0001045810-26-000019`) | +4.81% |
| Q1 FY27 | Q4 FY26 (`0001045810-26-000019`) | 78.0 | 81.615 (`0001045810-26-000051`) | +4.63% |
| Q2 FY27 | Q1 FY27 (`0001045810-26-000051`) | 91.0 | 96.221 (`0001045810-26-000073`) | +5.74% |
| Q3 FY27 | Q2 FY27 (`0001045810-26-000073`) | 108.0 | 115.844 (`0001045810-26-000088`) | +7.26% |
| Q4 FY27 | Q3 FY27 (`0001045810-26-000088`) | **130.0 (+-3%)**, 14 weeks | not reported | |

| Window | Pairs | Beats | Median | Mean | Minimum | Maximum |
|---|---|---|---|---|---|---|
| All pairs (Q1 FY23 to Q3 FY27) | 19 | 18 | +5.57% | +5.61% | -17.23% | +22.79% |
| Before Q4 FY24 (Q1 FY23 to Q3 FY24) | 7 | 6 | +2.32% | +4.74% | -17.23% | +22.79% |
| Q4 FY24 to Q3 FY27 (the window of the first version) | 12 | 12 | +5.65% | +6.12% | +2.47% | +10.52% |

**The rule for the base** (decision of the Head of Fundamental Research, 2026-11-21; also in the model file, D2, and in the report, section 3; for the order see weak point W1 of the report).
- The base of year-1 growth is the guide. The guide is +41.0%: 121 + 125 + 130 + 134 = 510 over 361.8. Quarters 2 to 4 are a judgment of about +3.3%, +4.0% and +3.1% a quarter.
- If (a) the median of all pairs is above zero and (b) at least 15 of 19 pairs (79%) are beats, the base adds half the median beat.
- Year-1 growth = 1.41 x (1 + beat) - 1.
- Result: (a) the median is +5.57%, so (a) holds. (b) 18 of 19 are beats (94.7%), so (b) holds. The beat is +2.78%. The base is 1.41 x 1.0278 - 1 = **44.9%**.
- Half is a cut that keeps the base low against the company's record. From the fourth pair on, the actual beat was at or above half of the earlier median in 15 of 16 quarters. It was at or above the full earlier median in only 9 of 16 (checked by the analyst).
- Sensitivity rows (100,000 paths, P50): a miss like Q2 FY23 (-17.2%: 16.7%) $72.86; the guide (41.0%) $98.60; a constant +4.2% a quarter after Q4 (42.1%) $100.04; the base (44.9%) $103.78; the full median (+5.57%: 48.9%) $109.23.
- Limits: one company; one upswing and one bust. The guide of Q4 FY27 has a band of plus or minus 3% (all earlier ones 2%) and 14 weeks. The beat is applied to the level of year 1, because each guide starts from the last result.

### L27. Brief of the Head of Fundamental Research: the downside of the cycle

| Date | Source | Tier | Expiry |
|---|---|---|---|
| 2026-11-20, about 23:25 UTC, before the first run | Brief of the Head of Fundamental Research in the session; not in git | Instruction of the Head of Fundamental Research (not a fact of the market) | None |

The words of the brief: "a downside of the cycle (customers cut their capital spend: a regime or a failure-like hazard of a demand drop, if the engine supports it; else a wider growth spread with a reason)".

Use: the collapse table (D14). The ticket (`PLANNING/board/T-20261120-nvda-valuation.md`, line 21) also names the cycle of the capital spend of the customers as a driver.

## E. Items that were not read, and what they change

| Item | Needed for | Effect if different |
|---|---|---|
| 10-Q of Q3 FY27 (when filed) | Guarantees note, purchase obligations, RPO, share count at the cover date, terms of the equity securities, the investigation and the leadership change (L2h) | The bridge (guarantees: up to -$4.49 a share), the stake value |
| Press release page | Read and traced in version 3 (section A2); two inferences are marked (L2f, L2g) | None; check again against the 10-Q |
| Convertible debt and accrued purchase consideration (footnote B, L2i) | The 10-Q | A claim that is not in net debt |
| Guidance history | Measured in L26 (19 pairs; Q1 FY23 to Q3 FY27); the store has no release before Q4 FY22 | The record before FY23 (not in the store) |
| 10-K FY26 and 10-Q notes (customer concentration, China, terms of the stakes) | L11, L12, L14 | A claim of up to $4.49 a share; the stakes at a different value |
| Closes of 2026-11-20 and later | "what the price implies" | A price at a different date |

## F. Facts of the market that are not in the ledger on purpose

No fact about the market share of NVDA is used as a number. The same is true for the spending of its customers, for competitors and for the rules of export. The text of the model says that "customers build own chips" and that "a high margin attracts sellers". Treat these as JUDGMENT with no source. The remark in the press release on China ("not assuming any Data Center compute revenue from China") is used only as a text in the reason of the growth start.
