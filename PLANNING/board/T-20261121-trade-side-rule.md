---
id: T-20261121-trade-side-rule
title: A dated, reviewed rule for the side of a fundamental trade recommendation
area: engine
status: backlog
owner: louellet-git
requested_by: louellet-git
head: fundamental
order: 1.5
branch:
created: 2026-11-21
acceptance: A record in `docs/assumptions/` (or a rule in `PLANNING/RULES.md`) defines the band and its reason, a tie rule (the rank of the close and its interval over seeds), the sims and the seed, the statistic, the named ranges of the judgment inputs, and the frame of the list test; it passes a review and is committed before the next ticker recommendation; it is not tuned on NVDA.
---
Condition of the gate of the NVDA trade recommendation (Head of Quality and Risk, 2026-11-21, REVISE of
caa0082). The rule in `analysis/NVDA/TRADE-RECOMMENDATION.md` was written after the price was known and has no
test power. The backtest of the DCF engine (`T-20261121-dcf-engine-backtest`) is the evidence for a band: a band
that the backtest selects is measured on years after its cut, not on the years that choose it.
