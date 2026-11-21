---
id: T-20261121-dcf-engine-integration
title: Put the new DCF work of 2026-11-21 into the engine, so that it runs as one set of commands, fast and with tests
area: engine
status: open
owner: louellet-git
requested_by: louellet-git
head: fundamental
order: 0.008
branch: (to set at start)
created: 2026-11-21
acceptance: Each item below is Rust code in `fa dcf` with a fixed-answer test, and no result of a study comes from a shell loop or an awk calculation; the full S&P 500 premium series runs in a stated time that is lower than now (about 45 to 70 minutes).
---
Owner words, 2026-11-21 about 19:30 UTC (question "Is all DCF work fitting to the DCF engine?", then): "It depends, we
need some way for this all to run efficiently/smoothly. What I meant was more so that."

Already in the engine: `attribute` (par curve, full bridge, stale price), `implied` and `premium-series` (one list-premium
function), the curve loaders (`auto.rs`), the data fixes (one mapping program, `mapcheck`).

Items (each with a test):
1. One curve rule for all commands (GSW when at most 3 days old, else the par fit), in one function; `attribute`
   uses it by default (now its default is GSW only).
2. `premium-series --per-ticker`: the bridge line of each company each week, so the 2-month S&P 10 table comes from one
   command (now a shell loop of `attribute`).
3. The common part and the company part of the premium change in Rust (now `awk`), in the output of `attribute` and
   `premium-series` (with `T-20261121-premium-company-part`).
4. The balance-sheet effect of the curve: if `T-20261121-rates-balance-sheet` finds it material, in the bridge of the
   engine (new SEC variables through `financial_analysis/standard_variables.toml`), not a side calculation.
5. Speed: cache the point-in-time mapping of each ticker by filing vintage (the facts change only at a filing), reuse it
   across dates; measure the time of the full S&P 500 series before and after.
6. The model documents and records (branch `claude/fundamental-model-docs`, in progress).
Not in scope: one engine for all company models (owner: "It depends"); the driver engine and the classic engine stay.
