---
id: T-20261120-store-arb-data
title: Add the rate, futures and option data of the arbitrage checks to the data store
area: importer
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: technology
order: 3
branch: claude/store-arb-data
created: 2026-11-20
acceptance: The store `louellet-git/fa-data` has the data sets of the list below with their dates in `SNAPSHOT.md`, and `fa recon rich-cheap`, `fa recon bills` and `fa futures check` run on the store with no request to a source.
---
Request of the Head of Quant Research (FA-L), from the owner request of 2026-11-20: find arbitrage and
reconcile our data (`T-20261120-arb-2-rates-recon`, `T-20261120-arb-4-futures-basis`).

The store has the rate curves, the FRED bond series and two option chains (IBM, JNJ). The checks also need:
1. FedInvest end-of-day Treasury prices: `prices/fedinvest_<date>.csv`, the last 60 business days.
2. The Treasury "Daily Bill Rates" file: `rates/treasury_bill_rates_<year>.csv`.
3. The New York Fed rates (SOFR, BGCR, TGCR, EFFR) and SOMA: `nyfed/`.
4. The futures files that `fa futures` reads (VX, ZQ, SR3; `futures/`) and the Cboe VIX history
   (`vix/VIX_History.csv`).
5. Later, when `fa options archive` exists (`T-20261120-vol-7-chain-copy`): the daily copies of the SPX,
   SPXW, SPY, VIX and VIXW chains with bid and ask.

Start with items 1 and 2: they unblock step 2 of `T-20261120-arb-2-rates-recon`.
