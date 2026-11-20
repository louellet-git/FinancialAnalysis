---
id: T-20261119-evaluator-bank
title: Build the value model for banks
area: engine
status: backlog
owner: louellet-git
branch:
created: 2026-11-19
acceptance: `fa dcf auto` gives a value range for a company of the template `bank` from an excess-return model, and a point-in-time test on the banks in the panel reports the coverage of its bands.
---
Owner request, 2026-11-19: build the value model for each company type, one type at a
time. Sequence: 1 of 5 (bank, insurer, REIT, ETF and fund, other classes).

Why a separate model: debt is the raw material of a bank, so free cash flow to the firm
and net debt have no clear meaning. `fa dcf auto` gives "no model" for this template.

Method (first version): excess-return model on book equity.
Value of equity = book equity + present value of (ROE - cost of equity) x book equity of
each year + a terminal value. Each element is a yearly process, as in the path engine:

- ROE goes to a long-run value (persistence and shock size from the bank panel).
- Book equity grows with the retained earnings (payout ratio from the history).
- Cost of equity from the rate path and a beta.
- Option: split ROE into net interest margin, fee income, cost ratio and loan losses.

Data in the repo now: `net_interest_income`, `noninterest_income`, deposits, total
assets, equity, net income, dividends (standard variables; 48 banks in the panel).
Data to check first: regulatory capital (CET1) and loan-loss provisions are not standard
variables; decide if the first version needs them.

Test: fixed answers with all shocks off; the point-in-time test of ROE and book equity
for the 48 banks; a comparison of value with book value for a sanity check.

## Design from the study of 2026-11-20

Source: `analysis/priors/bank_utility_study_2026-11-20.md` (ticket `T-20261120-bank-utility-study`). The text
above stays; the design below replaces "data to check first".

**Set.** The template `bank` has 53 tickers in the two universe lists (not 48). Split: deposit banks 35, card and
consumer lenders 7 (AFRM and SOFI are too young), custody banks 3, brokers and investment banks 6, false members 2
(CNH, FAF). The first version values the deposit banks and the custody banks. The brokers have no ROE persistence
(phi -0.04): no value. The card lenders come in step 2.

**Model.** Excess return on common equity in the path engine (new mode `excess_return`):
`r(t) = mu + phi (r(t-1) - mu) + e`; `B(t) = B(t-1) (1 + r(t) (1 - pi) + x(t))`; `V = B(0) + sum (r(t) - k(t)) B(t-1) / prod(1+k) + TV`;
`TV = B(N) (r_inf - k) / (k - g)`. `x(t)` is the clean-surplus gap (acquisitions with shares, other comprehensive
income), drawn from the cell history. Measured for the deposit cell (328 pairs, 33 banks): ROE `phi` 0.34 (se 0.07,
FE with Nickell correction), mean 11.0%, sigma 2.8 points; ROTE `phi` 0.30, mean 14.9%, sigma 3.8 points; weight of the
own mean 0.77; payout 0.57 (median); clean-surplus gap median 1.1%, mean 5.3%. A tangible form (ROTE on TCE) is the second view.

**Registry tags to add** (scope: template `bank`; coverage in the study, section A2): `NoninterestExpense`;
provision chain `ProvisionForLoanLeaseAndOtherLosses`, `ProvisionForLoanAndLeaseLosses`, `ProvisionForLoanLossesExpensed`,
`FinancingReceivableExcludingAccruedInterestCreditLossExpenseReversal`; loans chain
`LoansAndLeasesReceivableGrossCarryingAmount`, `FinancingReceivableExcludingAccruedInterestBeforeAllowanceForCreditLoss`,
`LoansAndLeasesReceivableNetOfDeferredIncome`, `LoansAndLeasesReceivableNetReportedAmount`,
`FinancingReceivableExcludingAccruedInterestAfterAllowanceForCreditLoss`; allowance
`FinancingReceivableAllowanceForCreditLossExcludingAccruedInterest`, `FinancingReceivableAllowanceForCreditLosses`,
`LoansAndLeasesReceivableAllowance`; `Deposits`, `DepositsDomestic`; preferred equity (the largest of `PreferredStockValue`,
`PreferredStockIncludingAdditionalPaidInCapitalNetOfDiscount`, `PreferredStockValueOutstanding`,
`PreferredStockIncludingAdditionalPaidInCapital`); preferred dividends `DividendsPreferredStockCash`,
`PreferredStockDividendsIncomeStatementImpact`, `DividendsPreferredStock`, `PreferredStockDividendsAndOtherAdjustments`;
`CommonStockDividendsPerShareDeclared` / `...CashPaid` (unit USD/shares); `dei/EntityCommonStockSharesOutstanding` (51 of 53 banks;
`shares_outstanding` is missing for 18). Second version: write-offs, recoveries, nonaccrual loans, interest income on
loans, interest expense on deposits. Do not add CET1 (10 to 15 banks have it).
`dividends_paid` includes preferred dividends: subtract them.

**Tests.** Fixed answers with all shocks off (`r = k` gives `V = B`; constant `r` gives `V = B (r - g)/(k - g)`); dividend
form equals excess-return form; the new variables against the filings for JPM BAC WFC USB PNC; refusals (preferred above
equity, negative equity, fewer than 5 years, cells not in the first version); the point-in-time test with a `--template bank`
option. Result of the study for the test: ROE band P10 to P90 has 79%, 79%, 77% inside for 1, 2, 3 years (bias: the real ROE is
above the median in 63% to 67%); the equity band needs the gap shock (71%, 71%, 62% inside with it; 18% without it).

**Open decisions for the owner:** terminal ROE (persistent or fade; JPM 222 or 153 per share), the cost of equity, common or
tangible base, the cells of the first version, a fix of the detection for CNH and FAF.
