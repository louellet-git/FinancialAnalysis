---
id: T-20261118-rf-from-treasury
title: Get the risk-free rate from the U.S. Treasury for each run
area: engine
status: in-progress
owner: merlan1996
branch: rates-source
created: 2026-11-18
acceptance: `python -m scripts.verdict IBM --price <price> --rf-date today` prints the Treasury 10-year rate and its date, and the WACC uses that rate.
---
Source: the backlog item "Refresh the risk-free rate on each run" in `PLANNING/tickets.md`.
The rate moved from 4.6% to 5.1% between July and October 2026.

Scope:

1. `financial_analysis/rates.py` gets US interest rates from the publishers. These are
   the U.S. Treasury par yield curves (nominal and real) and the New York Fed reference
   rates (SOFR, EFFR). FRED is for a cross-check. No API key is necessary.
2. `scripts/verdict.py --rf-date` sets the mean of the `rf` driver to the Treasury
   10-year rate on that date. The driver models do not change.
