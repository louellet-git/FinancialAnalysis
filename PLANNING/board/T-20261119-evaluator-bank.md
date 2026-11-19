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
