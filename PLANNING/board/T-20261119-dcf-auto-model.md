---
id: T-20261119-dcf-auto-model
title: Make a DCF model for each ticker from the data, with no hand work
area: engine
status: in-progress
owner: louellet-git
branch: claude/dcf-auto
created: 2026-11-19
acceptance: One command makes and runs a path model for a ticker from the standard variables and the parameter table, one command does this for a list of tickers in seconds, and a point-in-time test on the company panel reports how often the real revenue and cash flow were inside the bands.
---
Owner request, 2026-11-19: make the DCF general (not only for ANT) and efficient.

Measured on 2026-11-19: the engine is fast (one run of 10,000 paths takes 3 ms; each
`fa dcf` command takes 4 to 150 ms). The slow part is the model of a ticker: a person
or an agent writes it by hand (ANT: three agents and about 900,000 tokens). This ticket
removes that step for a first look at a ticker.

Parts:

1. `fa dcf auto TICKER`: makes a path model from the standard variables (`rust/mapcheck`),
   the parameter table (`analysis/priors/`), and the yield curve, and runs it.
2. `fa dcf screen`: the same for a list of tickers, with one row for each ticker.
3. A point-in-time test: for each company and each past year, make the model from the
   data up to that year only, and compare its bands with the later real values.

An automatic model is a screen. It is not a reviewed valuation. A person must not use
it alone for a trade (`CLAUDE.md`, "Review of a valuation report").
