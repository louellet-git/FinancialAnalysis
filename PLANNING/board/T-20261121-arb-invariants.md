---
id: T-20261121-arb-invariants
title: A register of no-arbitrage relations and an automatic daily check of all of them
area: engine
status: in-progress
owner: louellet-git
head: quant
story: E4.S2
order: 0.9
branch: claude/quant-invariants
created: 2026-11-21
requested_by: louellet-git
acceptance: (1) A research note on how quant firms keep and monitor no-arbitrage relations, with sources. (2) A register (one file for each relation: statement, formula, data, tolerance at bid and ask, source, check command, status). (3) One command runs every relation on the newest stored data, writes one dated report, and marks each break; it runs each market day without a person. Each step is small and shows value before the next.
---
## What the break count means (first; Head of Staff question, 2026-11-21)
The 143 breaks of SPX 2026-11-20 are a measure of the quote data, not of the market. The check already uses
executable prices (it buys at the ask and sells at the bid). A cost for each break removes few of them
(`fa options check SPX --date 2026-11-20 --rate 0.04`, size bands, USD for one share): 143 with no cost; 131
above $0.10 a share ($10 a contract); 59 above $1 a share. The largest are in long-dated, deep in-the-money series
(2027 expiries, prices above $400). The quotes are Alpaca indicative, not exchange quotes (NBBO), and have no quote
time. Thus a break is a measurement of the feed. A real arbitrage claim needs exchange quotes at one moment.

Owner request, 2026-11-21 (`chiefs/louellet-git/quant/RECORD.md`): a list of arbitrage relations "that should
generally always be true", code that runs autonomously to check them, research of how quant firms do this first,
then a fit to our systems (and new parts where needed).

## Steps
1. Research (now): industry practice for no-arbitrage monitoring and market-data surveillance; the full list of
   relations that apply to our data (SPX, SPY, VIX, VX, Treasury, rates); what we check today and what we do not.
2. Register: the folder and the form; the relations that we check today move in first (the chain checks of
   `fa options check`, the futures check, the Treasury fit residuals).
3. Runner: one command over the register; a check on each new snapshot in market hours, as continuous as the
   sources permit (owner, 2026-11-21: "for quant work, we're going to require as continuous as possible"); the
   duration of each break; a continuous capture on the e2-micro (research: `analysis/priors/continuous_option_data_2026-11-21.md`).
4. New relations, one at a time, each with a fixed-answer test. First candidate: the VIX from the SPX chain
   (Cboe method) against the published VIX (owner question, 2026-11-21).

The breaks on indicative quotes are measurements, not trades (`docs/models/options-static-arbitrage-check.md`).

## Capture plan (Head of Technology, 2026-11-21; research: `analysis/priors/continuous_option_data_2026-11-21.md`)
- Design B: SPX expiries up to 45 days every 2 minutes, all of SPX, SPY and VIX every 30 minutes, one capture
  near 16:14 ET for the VIX close. Alpaca indicative REST only. No Cboe source (automatic Cboe reads are stopped,
  2026-11-21).
- On the e2-micro, in its own process, separate from `fa algo`. One request budget for the Alpaca data host: both
  processes together under 200 a minute, each with a fixed share (for example algo 120, capture 60).
- The raw data stays on the VM (Alpaca terms: personal use, no distribution). Only the derived daily reports leave
  it, into the private journal repo.
- Store gzip CSV with the quote time `t` of each contract (the parser does not keep `t` today).
- Build after PR 131 merges. A one-day smoke test on the VM first; the interval from the measured bytes.

## State (2026-11-21)
Steps 1 and 2 and step 3 part 1 (`fa arb run` on one stored snapshot) pass the gate (PASS-WITH-NOTES, 5196d9c).
Next: the continuous capture (after PR 131), break persistence (`for`), then the planned relations one at a time.
Note of Quality and Risk: a runner keeps the cache path (`compare_vx` takes a history closure).
