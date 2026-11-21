---
id: T-20261119-filing-values-in-variables
title: Use the filing-level values for capex, depreciation and debt in the standard variables
area: importer
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: technology
order: 3
branch: claude/filing-capex
created: 2026-11-19
acceptance: For the 50 tickers of `rust/mapcheck/universe/batch50.txt`, the standard variables for capex, depreciation and amortization, and total debt agree with Yahoo Finance for 90% or more, and each value records the filing lines that it contains.
---
The first part is done in Rust: `rust/mapcheck/src/filing.rs` and `mapcheck filing` read the statements of the latest 10-K (calculation tree, company tags, values). `mapcheck crosscheck` compares the standard variables with the filing lines (50 tickers, 2026-11-19: capex 94%, total debt 83%, depreciation and amortization 69% agree).

Measured on 2026-11-19 against Yahoo Finance, 50 tickers: capex 73% with the standard variables and 92% with the filing lines; total debt 80% and 90%; depreciation and amortization 73% and 81%.

Decision of the owner, 2026-11-19: capital expenditure in free cash flow is equipment and capitalized software, without purchased intangibles (`PLANNING/RULES.md` R2). The filing values must use the same definition; a company tag for software (IBM) is then included.

Done on 2026-11-19 (branch `claude/admiring-allen-f5aaeq`): a critical input (revenue, capex, share count) that has no value in the company facts comes from the 10-K (`rust/mapcheck/src/supplement.rs`): company-tag capex lines, a standard tag with segment members, share classes. The filing reader keeps the three fiscal years of a 10-K and the facts with one segment member. A value that the company facts have is not replaced.

Next steps:
1. Replace the tag-name patterns with rules on the calculation tree (the lines under the investing total and the liabilities total), and correct the known errors (trap #43).
2. Give the three standard variables the filing value when the filing has one, with the filing lines as the source. Keep the companyfacts value as a second variable.
3. Use the calculation tree to decide `nested` and `overlap` in `mapcheck` (now a guess from the sums).
4. Read the facts with a segment (60% to 70% of the facts): segment revenue is Ticket 7 in the backlog.
5. Scale: 2,000 filings are 3 requests each and approximately 10 GB as files. Keep only the parsed values, or compress the files.

Part 1 (capex), 2026-11-21, branch `claude/filing-capex`: the capex lines of the latest 10-K come from the calculation tree under the investing total (the lines with the standard variable `capex` or `capitalized_software`; a line with no standard variable only if it is a payment and its tag or label names equipment, property, capital expenditure or software). For the fiscal years of that 10-K, `capex` and `capitalized_software` take the filing value with the lines as the source; the company-facts values stay in `capex_companyfacts` and `capitalized_software_companyfacts`. Crosscheck on `batch50.txt`: 92% to 94%. Largest `capex_total` changes (S&P 500, 85 tickers with a 10-K in the cache): DE -1.360B to -4.228B (equipment on operating leases), MA -0.489B to -1.215B (software), IBM -1.091B to -1.738B (software), MCD +11%. Correction (review cycle 1): `fa dcf auto` and `fa dcf screen` DO read the filing values (`import_ticker`, `rust/src/company.rs`); `fa dcf pit` does not (identical output). After the fixes of cycle 1, the sp500 screen P50 changes for URI (-11.1%), IBM (-5.6%), MA (-4.9%), ED (+2.5%), CMCSA (-0.4%); VMRK WARN to PASS. Lease equipment of a finance arm (DE, CNH) stays out until the owner decides. Open: the Yahoo comparison of the acceptance needs the network (no Yahoo fundamentals in the store); depreciation and debt are parts 2 and 3.
