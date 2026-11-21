---
id: T-20261121-dcf-engine-integration
title: Put the new DCF work of 2026-11-21 into the engine, so that it runs as one set of commands, fast and with tests
area: engine
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: fundamental
order: 0.008
branch: claude/attribution-reality
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

**Owner, 2026-11-21 about 18:50 UTC: "make sure that whatever is repeatable/should be expected of future work like this is
built into the correct engine." The repeatable steps of the week study, each with its engine (written before the code).**
Status of the ticket: in-progress, branch `claude/attribution-reality`.

| Step of the study | How it was done | Where it goes | Owner |
|---|---|---|---|
| Bridge of a change, common and company parts, 8-K flag, stale and left-out names, par curve | `fa dcf attribute` | done (at the gate) | Fundamental |
| The 10 largest of a list | a list file made by hand | `fa dcf attribute --top N` (by market value at D1, one weight for each company) | Fundamental |
| Growth base rates by size | an agent with jq and awk | `fa dcf baserates` (item R1 below) | Fundamental |
| Implied growth against the base rate; implied premium against its history | by hand | `fa dcf attribute --reality` (item R2) | Fundamental |
| The bridge week by week for each company | a shell loop | `fa dcf premium-series --per-ticker` (item 2 above) | Fundamental |
| One curve rule (GSW, else par) | two code paths | one function (item 1 above) | Fundamental |
| Balance-sheet effect of the curve | an agent with jq and awk | the bridge of the engine, after its backtest (item 4 above) | Fundamental |
| The view on the premium and the growth | none yet | the view engine, file `--common-file` | Economics |
| Facts of an earnings release in the window | an agent read the 8-K | the path of the release in the output (R2); a parser is a data task | Technology |
| Poison and truncation test of a series | a shell script | `analysis/priors/data/premium_series_pit/make_cache.sh`; the skill `point-in-time-test` names it | Fundamental |
| Missing mapped revenue of old years (51 tickers) | patched by hand in the study | the mapping (`mapcheck`); the engine never patches | Technology |

R1. `fa dcf baserates --tickers-file F --as-of D [--years 10] --out FILE`: for each company of the list, each window of
`--years` fiscal years whose end year was filed on or before D (point in time), from the mapped variable `revenue` only (no
raw tag); the compound growth of each window; the size bucket by the revenue of the base year (below 1B, 1B to 10B, 10B to
50B, 50B or more; nominal dollars; fixed now). Output: for each bucket, the last window of each company and the pooled
windows: n, median, 75th, 90th, 95th, maximum (with the ticker), and the count of companies with no base-year value. Limits
printed: today's list (survivorship), overlapping windows, acquired revenue.
R2. `fa dcf attribute --reality [--premium-history FILE]`: a block "(5) REALITY CHECK" and CSV columns at the end: the base
revenue and its bucket, the own 5-year growth, the implied 10-year growth at D2 with its percentile in the bucket of the base
rates of D2 (the last windows), a flag when it is above every case; with `--premium-history` (a CSV of `fa dcf
premium-series`), the percentile of the list premium of D1 and of D2 in the rows before D1; the path of the 8-K release of
the window when the cache has it. A measurement: no input of the model is set from it. An assumption record for the buckets
and the window; the model documents updated; tests with fixed answers; a poison test (a window that ends after D does not
enter).

**State (2026-11-21 about 20:10 UTC):** R1, R2 and `--top N` are built (44205b7; suites 885 + 17 + 1; not at the gate yet).
Real run, S&P 100 `--top 10 --reality`: the list premium 1.21% at D1 (percentile 0 of the 410 weeks before) and 1.67% at
D2 (percentile 2); bucket 50B or more: n 46, median 3.9%, 90th 10.0%, maximum 21.0%; 103 of 500 companies have no
base-year value (the mapping gap of old years, ticket `T-20261121-revenue-old-tags` of Technology, and short stores).

**More repeatable steps (design before code, 2026-11-21):**
R3. The late-reaction check. In the study, the premium rise of one week mostly reversed the week before (the yields rose
first; the stocks followed). `fa dcf attribute --prior-window`: with the window D1 to D2 of n days, also run D0 to D1 and
D0 to D2 (D0 = D1 less n days, moved to the last SPY trading day on or before it), and print the three "all" lines of the
premium-first bridge with the common change of each, in a block "(6) PRIOR WINDOW". No new CSV column.
R4. One curve rule: `--curve auto`, the default of `fa dcf attribute`: GSW at both dates when its row is at most 3 days
older than each date; else the par fit at both dates (one source at both dates); the text names the source. With GSW at
both dates the output is the same as `--curve gsw`. `fa dcf premium-series` keeps its rule for each anchor (A-027).
R5. `fa dcf premium-series --per-ticker FILE`: a second CSV with one row for each week and ticker (the bridge line of the
ticker in points of the list, the price change, the weight), so the weekly table of a small list comes from one command.
The main CSV contract with Economics does not change.
