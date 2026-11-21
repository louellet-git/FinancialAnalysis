# NVDA research ledger (DRAFT, not reviewed)

Version 2 of 2026-11-21. Step 1 of the review (analyst). Version 1 (commit 9a2e6f0) had the base of 2026-07-26. Form of
`analysis/IBM/research-ledger.md`, with one id for each fact.

Each outside fact has a date, a source, a tier and an expiry. Tier: PRIMARY = filing or XBRL fact of the SEC; COMPANY 8-K = the
press release of the company, filed as an 8-K, unaudited; SECONDARY = a data vendor or a press source; REPO = a record or a table of
this repo (CONVENTION or DATA of the calibration); JUDGMENT = the analyst, with no source. "WAITS" = the number is old because a
newer filing exists or will exist, and it is not in the data store.

Data store: `data/raw`, commit `a1dffb5` (2026-11-21). SEC company facts of 2026-11-20. NVDA prices to 2026-11-19 (close $145.98).
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
| L1a | Sequential growth | +21.9% (Q3 FY26), +19.5% (Q4), +19.8% (Q1 FY27), +17.9% (Q2), +20.4% (Q3 FY27, with L2a) | to 2026-10-25 | PRIMARY + 8-K | calculated | same | growth start |
| L1c | Operating income and margin | Q4 FY26 44.299, Q1 53.536, Q2 63.734 $B (66.24% of revenue in Q2); with Q3 (L2a) the trailing four quarters are 239.089 $B, 66.08% | same | PRIMARY + 8-K | `OperatingIncomeLoss`; Q4 FY26 = 130.387 less the first three quarters | same | margin start |
| L1d | Gross margin | Q1 FY26 60.52%, Q2 FY26 72.42%, Q3 FY26 73.41%, Q1 FY27 74.93%, Q2 FY27 74.98%, Q3 FY27 74.8% (8-K) | same | PRIMARY + 8-K | `GrossProfit` / `Revenues` | same | margin context |
| L3 | Revenue, operating margin by fiscal year | FY17 6.91 $B, 28.0%; FY18 9.71, 33.0%; FY19 11.72, 32.5%; FY20 10.92 (-6.8%), 26.1%; FY21 16.68, 27.2%; FY22 26.91, 37.3%; FY23 26.97 (+0.2%), 15.7%; FY24 60.92 (+125.9%), 54.1%; FY25 130.50 (+114.2%), 62.4%; FY26 215.94 (+65.5%), 60.4%. Mean FY17 to FY22: 30.7%. Mean FY24 to FY26: 59.0%. | to 2026-01-25 | PRIMARY | `mapcheck values NVDA --periods 20`, 10-K rows | Next 10-K (2027-02) | margin target, shock size, bust history |
| L4 | Invested capital (receivables + inventory + prepaid - payables and accrued + net property + operating-lease assets + other non-current assets) | 2024-01-28 18.74; 2025-01-26 29.98; 2025-07-27 39.61; 2026-01-25 53.44; 2026-07-26 91.44 $B (core without lease and other: 12.89, 25.15, 30.31, 42.27, 70.31) | per date | PRIMARY | balance sheet tags (`AccountsReceivableNetCurrent`, `InventoryNet`, `PrepaidExpenseAndOtherAssetsCurrent`, `AccountsPayableCurrent` + `AccruedLiabilitiesCurrent`, `PropertyPlantAndEquipmentNet`, `OperatingLeaseRightOfUseAsset`, `OtherAssetsNoncurrent`). `nested` of mapcheck checked: taxes payable, deferred revenue and accrued pay are inside accrued liabilities and are not added. | Next 10-Q **WAITS**: the press release has no receivables, inventory or payables | `s2c` (not updated) |
| L4a | Incremental sales to capital | FY25 5.7 (core) / 6.2; FY26 5.0 / 3.6; Jul-2025 to Jul-2026 3.4 / 2.7. Trailing revenue to capital 4.0 (FY26), 3.3 (Jul-2026). Receivables 63.059 $B on Q2 revenue 96.221 (60 days; 45 days in Q1 FY27); inventory 31.575 $B. | per date | PRIMARY | calculated from L1 and L4 | same | `s2c` |
| L5 | Effective tax rate | FY24 12.0%; FY25 13.3%; FY26 15.1% (21.383 / 141.450); quarters Q1 FY26 to Q2 FY27 14.3%, 15.3%, 15.9%, 14.8%, 16.6%, 16.5%; Q3 FY27 17.4% (13.730 / 79.017, L2b); statutory 21% | to 2026-10-25 | PRIMARY + 8-K | `IncomeTaxExpenseBenefit` / income before tax | Next 10-Q | tax (the model keeps 21%, CONVENTION, with the reason in the model file) |
| L6 | SBC | FY26 6.386 $B (3.0%); Q4 FY26 1.633, Q1 1.928, Q2 2.026 $B (2.11%); the three quarters: 5.587 / 245.963 = 2.27%. Q3 FY27: **not in the press release; WAITS.** | to 2026-07-26 | PRIMARY | `ShareBasedCompensation` | Next 10-Q **WAITS** | SBC start |
| L7 | Purchases of businesses | FY25 1.007 $B; FY26 1.535 (`PaymentsToAcquireBusinessesNetOfCashAcquired`) plus 13.000 (`PaymentsToAcquireBusinessTwoNetOfCashAcquired`; goodwill +14.6 in Q4 FY26); H1 FY27 0.298. Five-year sum to FY26 15.9 / 461 = 3.4% of revenue. For the nine months of FY27 see L2e. | to 2026-07-26 | PRIMARY | company facts, 10-K 0001045810-26-000021 | Next 10-Q | `process.charge` |
| L8 | Beta | Regression beta 2.021 (standard error 0.132), 260 weekly returns 2021-11-29 to 2026-11-16 on SPY (SPY ends 2026-11-18 in the store; the weekly returns end 2026-11-16 in `fa dcf auto`); shrunk 2/3 + 1/3 = 1.681 (standard error 0.088). R2 0.477. Version 1: 2.013, 1.676. | 2026-11-16 | REPO (calculated from the price cache) | `fa dcf auto NVDA --cache data/raw --as-of 2026-11-20 --explain` | Each new price file | `discount.beta` |

## A2. The 8-K press release of 2026-11-17 (fiscal Q3 FY27, quarter end 2026-10-25)

Source for all rows of this section: 8-K item 2.02, exhibit 99.1, accession 0001045810-26-000088,
`https://www.sec.gov/Archives/edgar/data/0001045810/000104581026000088/q3fy27pr.htm`. It was read on 2026-11-21 by a fetch tool
(HTTP 200) and the numbers were given to the analyst by the orchestrator; the analyst did not fetch the page. Tier: company 8-K,
unaudited. Expiry: when the 10-Q of Q3 FY27 is filed (then check each row against it). Amounts in $ millions in the source.

| Id | Fact | Value | Used in |
|---|---|---|---|
| L2 | Status | The 10-Q of Q3 FY27 is not filed. The store has the list entry of the 8-K only (`sec_8k_earnings.json`). | all rows below |
| L2a | Revenue Q3 FY27 | 115,844 (Q2 FY27 96,221; Q3 FY26 57,006): +20.4% q/q, +103% y/y. Data Center 107.6B (+21% q/q, +110% y/y). Edge Computing 8.2B (+14% q/q, +42% y/y). No other segment split is given and there is no history of the two segments. Trailing four quarters to 2026-10-25: 68.127 + 81.615 + 96.221 + 115.844 = **361.807 $B**. | base revenue |
| L2b | Income Q3 FY27, GAAP | Gross margin 74.8% (Q2 75.0%). Operating expenses 9,156. Operating income 77,520 (Q2 63,734): 66.92% of revenue. Other income, net 1,497. Income before tax 79,017. Tax 13,730 (17.4%). Net income 65,287. Diluted EPS $2.70. | margin start, tax |
| L2c | Cash flow Q3 FY27 (nine months) | Operating 48,253 (122,675). Purchases of property, equipment and intangibles (4,422) ((8,856)). Repurchases (22,122) ((61,166)). Dividends (6,026) ((12,316)). Debt issued, net, 24,896 for the nine months. With Q4 FY26 (OCF 36.188, capex 1.284 from L1 tags) the trailing free cash flow is 158.863 - 10.140 = 148.723 $B, 41.1% of trailing revenue. | cross-check of the margin |
| L2d | Outlook Q4 FY27 (**forward disclosure, rule R1: used**) | Quarter of 14 weeks ("includes an additional week"). Revenue "$130.0 billion, plus or minus 3%". "NVIDIA is not assuming any Data Center compute revenue from China in its outlook." GAAP gross margins 71.0% to 72.0%. GAAP operating expenses about $11.0 billion. Full-year FY27 GAAP tax rate 16.0% to 18.0%. In 13 weeks: 130.0 x 13 / 14 = 120.7 $B, +4.2% on Q3 (reported +12.2%). | growth start, margin start, tax |
| L2e | Purchases of businesses and stakes | Acquisitions, net of cash acquired and other: 2,718 in Q3 and (3,031) in nine months. "Groq, Inc.": (2,944) in nine months (the two lines may overlap). **Purchases of equity securities: 15,696 in Q3.** Pending acquisition of Hugging Face (no amount). | charge; bridge decision |
| L2f | Balance sheet 2026-10-25 | Cash and cash equivalents 29,089. Marketable debt securities 24,423. Marketable equity securities 44,219. Non-marketable securities 65,528. Short-term debt: none. Long-term debt 32,371. Total assets 372,188. Shareholders' equity 266,666. Diluted weighted shares Q3: 24,186 million. Non-GAAP figures no longer exclude SBC from Q1 FY27. | bridge, shares |

The quote of the press release names Colette Kress as interim CEO and CFO. No model input comes from it.

## B. The bridge (balance sheet of 2026-10-25, L2f)

| Id | Item | Value | In the bridge? | Reason (rules F1, F15, F16) |
|---|---|---|---|---|
| L10 | Debt | 32.371 $B long-term; no short-term debt (07-26: 33.366). Debt issued in the nine months 24.896 $B. | Yes, book value, one time | Interest expense is not in the cash flow (option B: EBIT less tax less reinvestment), so the debt is subtracted one time (F1). |
| L10a | Cash | 29.089 $B | Yes | Interest income is not in the margin (the margin is the operating margin), so the cash is counted one time (F15). |
| L10b | Marketable debt securities | 24.423 $B (07-26: 34.143 `DebtSecuritiesCurrent`) | Yes | Cash-like; same as cash. `mapcheck` gives no value for `short_term_investments` after 2025-10-26 (filer tag change, finding W7). |
| | **Net debt used** | 32.371 - 29.089 - 24.423 = **-21.141 $B** (net cash; $0.87 a share) | | Version 1: -23.220 |
| L11 | Marketable equity securities | 44.219 $B (07-26: 42.783) | **No** (base) | Decision below. |
| L11a | Non-marketable securities | 65.528 $B (07-26: 47.898; 10-K: 22.251) | **No** (base) | Decision below. |
| | Equity securities together | 109.747 $B = $4.54 a share | Sensitivity: P50 +$4.54 if in the bridge at book | The stock-and-flow rule (F15) would allow them: the gains (23.7 $B in H1 FY27, `GainLossOnInvestments`) are not in the margin. The base case excludes them for three reasons. (a) The purchases are large and rising: 15.696 $B in Q3 alone, 13.5% of revenue; they are a use of cash that may finance buyers of the product (a discount on the price in another form). Counting the stake at book and also charging the future purchases would be needed for a pair; the model has no evidence on the terms. (b) Part of the stakes is in companies whose purchases are already in the revenue that the model credits, so the stake and the revenue are one bet counted twice. (c) The terms and the carrying value basis are in the 10-Q notes, not read. Treatment of the purchases in the model: past purchases are paid (the cash is gone and the stake has value zero in the model); future purchases are NOT charged except in the 1.5% charge of businesses. If they are a financing of customers the cost is larger. Sensitivity: charge 5% of revenue gives P50 $91.01; stakes in the bridge give $102.19. |
| L12 | Guarantees, maximum exposure | 108.5 $B on 2026-07-26 (`GuaranteeObligationsMaximumExposure`); 0.86 $B on 2025-10-26. The 2026-10-25 value is not in the press release. | No, **open** | The tag gives a maximum, not an expected loss. If the whole amount were a claim: -$4.49 a share (24.186B shares). The note of the 10-Q WAITS. The risk goes with the demand collapse of the model (failure table), not with the bridge. |
| L13 | Operating lease liabilities | 5.494 $B (2026-07-26) | No | The lease cost is in the operating income (F15). Memo: -$0.23 a share. |
| L14 | Purchase obligations | `PurchaseObligation` 45.774 $B (2025-07-27, last value in the store); `UnrecordedUnconditionalPurchaseObligationBalanceSheetAmount` 22.7 $B (2026-01-25), 7.6 $B (2025-01-26) | No | Payments for product that the company will sell: the cost is in the gross margin and the inventory (a debt claim would count it two times). Rule R1: section D. |
| L9 | Shares | Diluted weighted 24,186M (Q3 FY27, L2f); 24,285M in Q2. Repurchased 203M shares in H1 FY27 for 39.0 $B (`PaymentsForRepurchaseOfCommonStock`); 61.166 $B in nine months (L2c). | Yes, diluted count | Existing awards are in the count; new awards are the SBC cost (F5). Cover page count (10-Q Q2) 24.1B on 2026-08-21. |

## C. Market and rate facts

| Id | Fact | Value | As-of | Tier | Source | Expiry |
|---|---|---|---|---|---|---|
| L15 | Last close of NVDA in the store | **$145.98** (open 152.915, high 158.13, low 145.71, volume 322.9M). Earlier closes: 2026-11-18 150.96; 2026-11-17 155.46; 2026-11-03 206.69; 2026-10-30 204.64; 2026-09-30 228.38. The 52-week high is 239.24. **The close of 2026-11-20 is not in the store.** | 2026-11-19 | SECONDARY | `data/raw/NVDA/prices.json` (Nasdaq; `prices.meta.json` last date 2026-11-19) | Each close. Used only in "what the price implies". No input. |
| L16 | Zero-coupon curve | 10-year zero yield 5.13%; the six GSW numbers of 2026-11-13 are in the model | 2026-11-13 | PRIMARY (Federal Reserve Board, GSW) | `data/raw/rates/gsw_feds200628.csv` | Each new row |
| L17 | Equity risk premium | 4.41% (std 0.75%, lower bound 2%) | 2026-11-19 | REPO (CONVENTION; Damodaran implied premium) | `analysis/ANT/driver_model_paths.toml`, `docs/models/auto-path-model.md` | Next update of the premium |
| L18 | Long-run growth L | min(10-year zero yield 5.13%, 4.0%) = 4.0%; FOMC SEP 2026-09-16, 2.0% + 2.0% | 2026-09-16 | REPO (record A-001) | `docs/assumptions/A-001-terminal-growth.md` | Next SEP (December 2026) |
| L19 | Fade of fast growers | After a year of growth of 50% or more (all sizes, 669 company-years): median growth 30% (next year), 17% (2 years), 11% (10 years); 30 to 100 $B revenue (42 company-years): 12% (next year), -1% (2 years), 3% (3 years). Survivor bias. | 2026-11-19 | REPO (DATA, calibration table) | `analysis/priors/process_priors_2026-11-19.toml`, `[[fade]]` | Next calibration |

## D. Forward disclosures (rule R1) and consensus

| Id | Disclosure | Value | Decision | Reason |
|---|---|---|---|---|
| L20 | Remaining performance obligation | 3.2 $B (2026-07-26), 2.6 (2026-04-26), 2.3 (2026-01-25) (`RevenueRemainingPerformanceObligation`). The value at 2026-10-25 is not in the press release. | **Excluded** | 1.1% of the trailing revenue of version 1, under 1% now. It is for cloud and software contracts, not for the product orders. It does not bear on the growth driver. IBM had 105% coverage, and NVDA has about 1%. |
| L21 | Purchase obligations and supply commitments | See L14 | **Excluded as a number; plausibility check only** | They show what NVDA bought, not what the customers ordered. |
| L22 | Guidance of the company: outlook for Q4 FY27 (L2d) | Revenue $130.0B +-3% (14 weeks, 120.7 in 13 weeks); GAAP gross margin 71.0% to 72.0%; GAAP opex about $11.0B; tax 16% to 18% for FY27; no China data center compute revenue assumed | **USED** (version 2) | Evidence for the growth start (first quarter of year 1: +4.2% on Q3 in 13 weeks; reported +12.2%), for the margin start (GAAP operating margin about 63.0% in the guided quarter) and for the half-life (the sequential growth falls from +20% to +4% in one quarter). It sets no number alone: the model mean is 40% for year 1 and the guide is one quarter of four. The guidance of the earlier quarters and the revenue of the company against them are NOT in the store, so a habit of beating the guide is not measured. The EPS beats of L23 (6% to 10%) suggest that a guide can be low; this is not used. |
| L23 | EPS reported and consensus (Nasdaq) | Actual (non-GAAP; SBC is no longer excluded from Q1 FY27) EPS: Q4 FY26 1.57 (consensus 1.45), Q1 FY27 1.87 (1.70), Q2 2.22 (2.09), Q3 2.68 (2.47, reported 2026-11-17). GAAP diluted EPS of Q3 is 2.70 (L2b). Consensus of 2026-11-20: Q4 FY27 2.84 (15 analysts), Q1 FY28 3.27, Q2 3.77, Q3 4.33, Q4 4.92; fiscal year EPS FY27 9.60, FY28 16.09, FY29 22.18, FY30 26.90 (18, 18, 9, 3 analysts). No revenue estimates in the file. | **Cross-check only, no input.** The consensus for the four quarters Q4 FY27 to Q3 FY28 is 2.84 + 3.27 + 3.77 + 4.33 = 14.21, against 8.34 for the last four reported quarters: +70% in EPS. The model has revenue +40% for year 1 with a margin that falls. The street therefore expects a growth of the quarters after Q4 that is far above the +4% of the guide. The model does not follow it: the only evidence in the store for those quarters is the guide. This is the largest open difference with the street. |
| L24 | Price targets and ratings (Nasdaq) | Mean $232.59, low $130, high $350; 29 buy, 13 hold, 1 sell | 2026-11-20 | SECONDARY | `data/raw/NVDA/estimates/2026-11-20.json` | **Not used.** A target is not evidence for a driver. Shown only as the view of others. |

## E. Items that were not read, and what they change

| Item | Needed for | Effect if different |
|---|---|---|
| 10-Q of Q3 FY27 (when filed) | SBC of Q3, receivables, inventory, payables (`s2c`), guarantees note, purchase obligations, RPO, share count at the cover date, the face of the balance sheet | Moves the SBC start, `s2c`, the bridge (guarantees), the shares |
| Press release text itself | All of section A2: the analyst received the numbers from the orchestrator and did not read the page | A wrong number in section A2 would move the base, the growth start and the bridge |
| Guidance history (past outlooks against results) | Whether the guide for Q4 is low | The start growth could be higher |
| 10-K FY26 and 10-Q notes (customer concentration, China, terms of the stakes) | L11, L12, L14 | A claim of up to $4.49 a share; the stakes at a different value |
| Closes of 2026-11-20 and later | "what the price implies" | A price at a different date |

## F. Facts of the market that are not in the ledger on purpose

No fact about the market share of NVDA, about the spending of its customers, about competitors or about the rules of export is used as a number. Where the text of the model says that "customers build own chips" or that "a high margin attracts sellers", it is a JUDGMENT with no source. The skeptic should treat it so. The remark in the press release on China ("not assuming any Data Center compute revenue from China") is used only as a text in the reason of the growth start.
