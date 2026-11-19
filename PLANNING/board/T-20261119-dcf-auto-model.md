---
id: T-20261119-dcf-auto-model
title: Make a DCF model for each ticker from the data, with no hand work
area: engine
status: review
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

## State, 2026-11-19 (branch `claude/dcf-auto`)

Parts 1 to 3 run on the branch. The commands are `fa dcf auto TICKER`, `fa dcf screen` and `fa dcf pit`. The rules, the files and the traps are
in `docs/ARCHITECTURE.md` (section "the automatic model") and `docs/HIDDEN-ASSUMPTIONS.md` (#185 to #199).

- Part 1: the model of a ticker has one rule for each number. `--explain` prints the rules. `--write FILE` writes a model that `fa dcf drivers TICKER --model FILE` runs.
- Part 2: the screen maps, builds and simulates 1,003 tickers. The time of each stage is in the output. The result does not depend on the number of threads.
- Part 3: the point-in-time test. Result files: `analysis/priors/auto_pit_2026-11-19.txt` (table from all years) and `analysis/priors/auto_pit_cut2015_2026-11-19.txt` (table from the years up to 2015).
- The fade table is now in `analysis/priors/process_priors_2026-11-19.toml` (`[[fade]]`). The AR(1) part of the file did not change.

Next steps (each is a ticket if a person wants it):

1. Templates: a model for a bank (excess return), an insurer, a REIT (FFO) and a fund. Today these give "no model for this template".
2. A better default when no price is known. Without a price the equity weight is 1.0 and beta is 1.0: a company with much debt (a utility) gets a negative value. A bulk price source is needed.
3. A charge for acquisitions. The hand models of IBM (3.55% of revenue) and JNJ (about 5.8%) have one; the automatic model has none, and its growth includes growth that was bought.
4. A rule for the companies with a decline in the base year (the point-in-time test shows a real revenue 8% to 11% above the P50 at T+2 and T+3).
5. A test of the value against later stock returns (needs prices).
6. The trailing four quarters as the base, and a check of the template for banks and REITs that the mapping calls `standard` (PNFP, GBCI, VICI and OMF pass as `standard`).
