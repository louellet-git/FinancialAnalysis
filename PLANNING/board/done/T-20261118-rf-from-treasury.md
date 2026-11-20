---
id: T-20261118-rf-from-treasury
title: Get the risk-free rate and inflation from the publishers for each run
area: engine
status: done
owner: merlan1996
branch: rates-source
created: 2026-11-18
acceptance: `python -m scripts.verdict IBM --price <price> --rf-date today` prints the Treasury 10-year rate, its date and its inflation part, and the WACC uses that rate.
---
Source: the backlog item "Refresh the risk-free rate on each run" in `PLANNING/tickets.md`.
The rate moved from 4.6% to 5.1% between July and October 2026.

Scope:

1. `financial_analysis/rates.py` gets US rates and inflation from the publishers. These
   are the U.S. Treasury par yield curves (nominal and real), the New York Fed reference
   rates (SOFR, EFFR) and BLS CPI-U. It calculates breakeven inflation and 5y5y forward
   inflation. FRED is for a cross-check. No API key is necessary.
2. `scripts/verdict.py --rf-date` sets the mean of the `rf` driver to the Treasury
   10-year rate on that date. It prints the real and inflation parts of the rate and a
   terminal-growth check. The driver models do not change.
3. `rust/` calls the Python code with `--json` and gives typed results. The `fa` command
   and the Rust library can call each Python entry point.
