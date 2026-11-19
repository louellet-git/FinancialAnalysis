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
