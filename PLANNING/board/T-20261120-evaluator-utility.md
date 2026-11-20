---
id: T-20261120-evaluator-utility
title: Build the value model for utilities
area: engine
status: backlog
owner: louellet-git
branch:
created: 2026-11-20
acceptance: `fa dcf auto` gives a value range above zero for each of the 27 utilities that the free-cash-flow model values at or below zero, from an excess-return model with an allowed-return ROE, and a point-in-time test on the 34 regulated utilities reports the coverage of its bands.
---
Owner request, 2026-11-20 (investigation of the automatic model). Sequence: 1b of 5. Start after
`T-20261119-evaluator-bank`: the model uses the same engine (mode `excess_return`). It comes before the insurer ticket.

Why a separate model: a utility invests more than its operating cash flow for years because its rate base grows and earns
a regulated return, and debt is part of the business. For DUK, ES and FE the free cash flow is below 0 in 9, 9 and 7 of 10
years (10-year sums -$20B, -$10B, -$5B). A free-cash-flow model with the history margin gives a value at or below zero.
Source: `analysis/priors/bank_utility_study_2026-11-20.md`, Part B.

**Set.** The template `utility` (new, in `filer_template`): the company has any of `PublicUtilitiesPropertyPlantAndEquipmentNet`,
`...PlantInService`, `PublicUtilitiesAllowanceForFundsUsedDuringConstruction...`, `UtilitiesOperatingExpenseMaintenanceAndOperations`,
or (regulatory assets + regulatory liabilities) / assets is at least 7%. It finds 36 tickers in the two universe lists
with no false member (29 in `sp500.txt`, 7 in `next500.txt`). 34 are regulated utilities (AEE AEP ATO AWK CMS CNP D DTE DUK ED
EIX ES ETR EVRG EXC FE IDA LNT NEE NFG NI OGE PCG PEG PNW PPL SO SRE SWX TXNM UGI WEC WTRG XEL). CEG and NRG are producers with
no rate base (regulatory share 0%); AES and VST are missed by the rule. Decide the class of the producers.

**Model (first version): excess return on common equity.** Same equations as the bank ticket.
`r(t) = mu + phi (r(t-1) - mu) + e`; `B(t) = B(t-1) (1 + r(t) (1 - pi_net))` with `pi_net` = (dividends + buybacks - share issues) / net income.
Measured (323 pairs, 34 utilities): ROE `phi` 0.20 (se 0.09), mean 10.2%, median 10.1%, sigma 5.8 points; the true spread of the
company means is 0.7 points, so the weight of the own mean is 0.10 (all utilities go to about 10%, the allowed return);
net payout median 0.57; growth of PP&E median 7.0%; growth of equity median 5.6%; share issues median 1.6% of equity;
capex / depreciation median 2.5; debt / PP&E median 0.54. First calculation (DUK, ES, FE, ETR, SO): value 1.46 to 1.70 times
book, 8% to 49% below the close (comparison only); one point of the cost of equity moves the value by 15% to 20%.
The raw betas are 0.20 to 0.42 (Ke 7.2% to 7.8%): decide a floor.

Not used in the first version: the dividend discount model (no stable growth input: the Gordon value of ES is 2.6 times the
close); the free-cash-flow model with growth capex separated (XBRL has no split of capex; depreciation as the proxy for
maintenance capex gives growth capex of 47% to 67% of capex). Step 2: the rate base form with `PublicUtilities...Net`,
plant in service, construction in progress, regulatory assets and liabilities, and the allowed return (manual input).

**Registry changes** (scope: template `utility`): `ppe_net` alternative `us-gaap/PublicUtilitiesPropertyPlantAndEquipmentNet`
(use the larger value; ES has none now, ETR has $0.48B for a $52.9B plant, FE has none after 2023);
capex chain `PaymentsToAcquirePropertyPlantAndEquipment`, `PaymentsForCapitalImprovements` (ETR 2007 to 2023: the
registry capex is 0.0 to 0.3 $B in 2017 to 2023), with a check that a capex below 2% of PP&E is not a capex (NEE has no capex tag for
2016 to 2022); `RegulatoryAssets` (else noncurrent + current), `RegulatoryLiabilities` (else noncurrent + current),
`ConstructionInProgressGross`, `PublicUtilitiesPropertyPlantAndEquipmentPlantInService`,
`PublicUtilitiesAllowanceForFundsUsedDuringConstructionCapitalizedCostOfEquity`, `CommonStockDividendsPerShareDeclared`.
Build `rust/mapcheck`, run both `cargo test` commands after the change.

**Tests.** Fixed answers with all shocks off (as for banks); the mapping tests (ETR capex 2017 to 2023 above $4B; ES `ppe_net`
$45.9B); a refusal for a company with a regulatory share below 1.5% and no `PublicUtilities` tag; the point-in-time test of ROE
and equity growth for the 34 utilities; the acceptance test: a value above zero for the 27 utilities. The price is never an input.

**Open decisions for the owner:** the class of the producers (CEG, NRG, AES, VST, TLN); the beta floor and the cost of equity;
the terminal ROE (persistent or fade); whether the allowed return is a manual input in step 2.
