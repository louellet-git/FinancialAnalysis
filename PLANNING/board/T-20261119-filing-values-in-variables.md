---
id: T-20261119-filing-values-in-variables
title: Use the filing-level values for capex, depreciation and debt in the standard variables
area: importer
status: backlog
owner: louellet-git
branch:
created: 2026-11-19
acceptance: For the 50 tickers of `rust/mapcheck/universe/batch50.txt`, the standard variables for capex, depreciation and amortization, and total debt agree with Yahoo Finance for 90% or more, and each value records the filing lines that it contains.
---
The first part is in `main`: `financial_analysis/filing.py` and `python -m scripts.filing_tree` read the statements of the latest 10-K (calculation tree, company tags, values).

Measured on 2026-11-19 against Yahoo Finance, 50 tickers: capex 73% with the standard variables and 92% with the filing lines; total debt 80% and 90%; depreciation and amortization 73% and 81%.

Decision of the owner, 2026-11-19: capital expenditure in free cash flow is equipment and capitalized software, without purchased intangibles (`PLANNING/RULES.md` R2). The filing values must use the same definition; a company tag for software (IBM) is then included.

Next steps:
1. Replace the tag-name patterns with rules on the calculation tree (the lines under the investing total and the liabilities total), and correct the known errors (trap #43).
2. Give the three standard variables the filing value when the filing has one, with the filing lines as the source. Keep the companyfacts value as a second variable.
3. Use the calculation tree to decide `nested` and `overlap` in `mapcheck` (now a guess from the sums).
4. Read the facts with a segment (60% to 70% of the facts): segment revenue is Ticket 7 in the backlog.
5. Scale: 2,000 filings are 3 requests each and approximately 10 GB as files. Keep only the parsed values, or compress the files. Move the parser to Rust when the rules are stable.
