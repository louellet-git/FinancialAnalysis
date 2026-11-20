---
id: T-20261120-dcf-engine-accuracy
title: Measure the weak points of the stochastic DCF engine and correct them one at a time
area: engine
status: backlog
owner: louellet-git
head: fundamental
order: 1.3
branch:
created: 2026-11-20
acceptance: Each part has a measurement before the change and after it (the point-in-time test, or the standard error of the P50), a test with a fixed answer, and a record of the choice; a part that gives no measured gain is not merged.
---
Owner word, 2026-11-20: "Immediate goal is basically optimizing / improving the DCF engine. It's a
stochastic engine for calculating various simulations of what a stock price or asset price should be.
We're going to stick to underlyings and not expand into options and other stuff for now."

Scope: the automatic path model and the path engine, for stocks. No option work.

Candidate parts, from the evidence that the repo has (`docs/ARCHITECTURE.md`, section "The automatic
model"; `docs/HIDDEN-ASSUMPTIONS.md`, #194 and #199; the two studies of 2026-11-20). Measure each one first.
Do them one at a time, the largest measured error first:
1. Precision of the simulation: the standard error of the P50 is 1.8% of the P50 with 4,000 paths, and the
   engine makes no antithetic numbers. Aim: the same precision with fewer paths, or a better precision.
2. Base revenue: the model starts from the latest fiscal year, which can be 11 months old. Aim: start from
   the trailing four quarters when they exist, with the point-in-time rule.
3. Bands: the bands are too wide for a company with a stable margin (coverage 95% against a target of 80%),
   and the revenue P50 is too low after a year of decline (+11% at T+3) and at 10 years (+12% to +24%).
4. Parameter cells by sector: the growth and margin cells have a size and no sector. Needs the SIC code
   (`T-20261120-sic-code`).
5. Terminal value: the margin of year 10 pays for the growth of the history, not for the terminal growth.

Rule: no part is chosen or tuned with the market price (`docs/QUALITY-STANDARDS.md`, section 6).
