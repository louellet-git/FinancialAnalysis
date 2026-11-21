---
id: T-20261121-arb-invariants
title: A register of no-arbitrage relations and an automatic daily check of all of them
area: engine
status: in-progress
owner: louellet-git
head: quant
order: 0.9
branch: claude/quant-invariants
created: 2026-11-21
requested_by: louellet-git
acceptance: (1) A research note on how quant firms keep and monitor no-arbitrage relations, with sources. (2) A register (one file for each relation: statement, formula, data, tolerance at bid and ask, source, check command, status). (3) One command runs every relation on the newest stored data, writes one dated report, and marks each break; it runs each market day without a person. Each step is small and shows value before the next.
---
Owner request, 2026-11-21 (`chiefs/louellet-git/quant/RECORD.md`): a list of arbitrage relations "that should
generally always be true", code that runs autonomously to check them, research of how quant firms do this first,
then a fit to our systems (and new parts where needed).

## Steps
1. Research (now): industry practice for no-arbitrage monitoring and market-data surveillance; the full list of
   relations that apply to our data (SPX, SPY, VIX, VX, Treasury, rates); what we check today and what we do not.
2. Register: the folder and the form; the relations that we check today move in first (the chain checks of
   `fa options check`, the futures check, the Treasury fit residuals).
3. Runner: one command over the register; one report for each day; a daily run with the chain copies.
4. New relations, one at a time, each with a fixed-answer test. First candidate: the VIX from the SPX chain
   (Cboe method) against the published VIX (owner question, 2026-11-21).

The breaks on indicative quotes are measurements, not trades (`docs/models/options-static-arbitrage-check.md`).
