---
id: T-20261119-bond-arbitrage-research
title: Find the cause of each bond price difference and list rich and cheap Treasury securities
area: engine
status: in-progress
owner: louellet-git
branch: claude/bond-arb
created: 2026-11-19
acceptance: One command gives, for a date, each Treasury security with its difference from the fitted curve, the cause group of the difference, and a list of rich and cheap securities after the bid-ask cost.
---
This ticket continues `T-20261119-bond-recon` (`fa recon bonds`). The branch starts
from `claude/bond-recon` with `main` merged, because the comparison code is not in
`main` (the owner holds that merge).

Goal of the owner: a small quantitative research desk that looks for arbitrage in the
difference between market bond prices and the prices from the rates that we get.

Steps:

1. Explain the differences that have one sign for a full group (bills: about -6.8 bp
   on three dates). Find if the cause is a convention (yield basis, settlement, curve
   input) or a market effect.
2. Fit a curve to the market prices of the coupon securities. Give the difference of
   each security from this curve (rich or cheap), in price and in yield.
3. Keep the differences for a sequence of dates. Measure if a difference stays or
   goes back to zero.
4. List the securities whose difference is larger than the bid-ask cost. Say what the
   data cannot show (financing rates, on-the-run premium, tax effects).

The repo gives analysis only. It sends no orders.

## State (2026-11-19)

Built (crate `rust/`; modules `fit.rs`, `richcheap.rs`, `history.rs`, `billdiag.rs`; method in `docs/ARCHITECTURE.md`):

1. `fa recon bills`: tests each cause of the bill difference. Result: no convention error. The
   Treasury par curve has bill nodes (1M to 3M) that are 8 to 11 bp above the Treasury's own bill-rate
   table. With the nodes from that table, the mean bill difference goes from about -6 bp to about -2 bp
   (#100, #101).
2. `fa recon rich-cheap`: a Nelson-Siegel-Svensson curve fitted to the notes and bonds of a date
   (fit error 1.5 to 2.5 bp; within 1 bp of the GSW curve from 1 to 20 years). It lists each security
   with residual, z-score, cost and group, and the candidates after the cost.
3. `fa recon history`: 50 business days (2026-09-08 to 2026-11-18). The residual of a security is
   persistent; its deviation from its own mean has a half-life of 1 to 2 days (#105).

Open:
- The Rust download of the price file. The cause is known (#69); the fix is not made. The cache is filled with `curl`.
- Cause of the difference between the Treasury par curve bill nodes and the Treasury bill-rate table (#100).
- The cost has no repo rate, no short-sale cost and no balance-sheet cost (#106). Nothing is called arbitrage.
- Owner to confirm the use of the FedInvest data for 50 dates (#67).
