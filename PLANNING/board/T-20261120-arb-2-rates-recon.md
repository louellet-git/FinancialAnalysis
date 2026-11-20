---
id: T-20261120-arb-2-rates-recon
title: Reconcile the rate curves and the bond series of the data store
area: engine
status: backlog
owner: louellet-git
requested_by: louellet-git
head: quant
order: 1.2
branch:
created: 2026-11-20
acceptance: A findings note with an evidence table gives the differences between the GSW curve and the Treasury par curve, and between the corporate curves (HQM, ICE BofA) and the Treasury zero curve plus the spread, for the dates of the store, with the command that gives each number, and the auditor passes it.
---
Owner request, 2026-11-20: start to find arbitrage and to reconcile our data.

The checks exist: `fa forwards check` (GSW against Treasury par), `fa recon bonds` (FedInvest prices against
the Treasury curve; curve level: HQM against Treasury zeros plus the mean ICE OAS), `fa recon bills`,
`fa recon rich-cheap`, `fa recon history`, `fa recon explain`. No note gives their result on our data.

Step 1 (the data of the store now): `rates/gsw_feds200628.csv`, `rates/treasury_nominal_2026.csv`, `bonds/`
(26 FRED series). Run the checks that need only these files. Give each difference in basis points and say
which source is wrong, or that the difference is a convention (day count, compounding, par against zero).
Step 2 (after `T-20261120-store-arb-data`): FedInvest prices, the bill rate file and the New York Fed rates.
Run rich and cheap and its persistence (`fa recon history`). A residual that lasts and that is larger than
the bid-ask cost is a candidate, not an arbitrage: the repo rate and the borrow are not known (`docs/HIDDEN-ASSUMPTIONS.md` #112).

Agent: `rates-credit-quant`. Form: the skill `investigation-report`. Gate: `auditor`.
