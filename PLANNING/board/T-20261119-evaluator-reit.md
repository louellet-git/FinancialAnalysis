---
id: T-20261119-evaluator-reit
title: Build the value model for REITs
area: engine
status: backlog
owner: louellet-git
branch:
created: 2026-11-19
acceptance: `fa dcf auto` gives a value range for a company of the template `reit` from a model of funds from operations, with a net-asset-value check, and a point-in-time test on the REITs in the panel reports the coverage of its bands.
---
Sequence: 3 of 5.

Why a separate model: depreciation of real estate is a large accounting cost that is
not a cash cost, and a REIT pays out most of its income. Net income and the standard
free cash flow give a wrong picture. `fa dcf auto` gives "no model".

Method (first version):

- Funds from operations (FFO) = net income + depreciation of real estate - gains on
  sales. Adjusted FFO = FFO - maintenance capex.
- A path model of adjusted FFO for each share: rental growth, margin, and the cost of
  new capital (a REIT issues shares and debt to grow), with the rate path.
- Check value: net asset value = net operating income / capitalization rate - debt.

Data in the repo now: investment property, lease income, depreciation, debt, dividends
(40 REITs in the panel). Data to check first: gains on sales and the split of capex
(maintenance against growth) are not standard variables. Mortgage REITs (NLY, AGNC)
are a different business: they have status FAIL now; decide if they go with the banks.

Test: fixed answers with all shocks off; the point-in-time test for the 40 REITs.
