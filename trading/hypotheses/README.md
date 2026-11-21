# Hypotheses

A hypothesis file states what a backtest must show, before the first run. The command
`fa portfolio backtest --hypothesis trading/hypotheses/<id>.toml` reads it. Without the file, the command
runs as an exploratory run: no run log, no variant is counted, and the result is not usable for a decision. An exploratory
run that reaches the out-of-sample window of a registered hypothesis of the same strategy is permitted, but it prints a
WARNING that names the hypothesis; a later result of that hypothesis must report it.

**Who uses it.** The harness is for a head whose result a person uses for a decision (for example a trade recommendation
of the gate of `docs/QUALITY-STANDARDS.md`). Other heads do not need it: exploration runs as before (word of the person,
2026-11-21: "The backtest harness and the hypothesis file are more for the heads that require it").

## Rules

- Register before the first run. Commit the file first: the command refuses a file that is not in HEAD or that differs from
  HEAD. The commit hash and the commit time of the file go into the run log (the proof of order).
- Do not change a file after its first run. A change is a new id (and a new file). Name the old id in `parent`.
- The file name is `<id>.toml`. The id has letters, digits, `-`, `_` and `.` only.
- `prior_variants` is needed: the number of variants that were tried before this file (exploratory runs, other start dates,
  other parameters). Write how you counted. Do not write 0 unless it is true.
- The variant count does not reset with a new id: the run log counts the variants of the id and of the family (all ids with
  the same `family`, found in the git history of all branches and in the working tree). A deleted run file stays in the count.
  The count is a warning for the reader, not a multiple-testing control: the deflated Sharpe ratio is part 2.
- The window: write `oos_start` before the first run. A child hypothesis (`parent`) uses the window of the oldest ancestor,
  also if its own `oos_start` is later. The run prints and logs `oos-read` (the run ends on or after the window start) and
  `oos_clean`. The window is not clean if `oos_prior_exposure = true`, if `oos_inherits` is set, if an earlier run of the
  family read it, or if the strategy is value-rank (the parameter table is fitted on all years through 2025).

## Form

```toml
id = "value-rank-sp500-v1"
date = "2026-11-21"            # the registration date
hypothesis = "text: what must the run show, and against which line"
strategy = "value-rank"        # value-rank or momentum
universe = "rust/mapcheck/universe/sp500.txt"   # a tickers file (path from the repository root)
oos_start = "2023-01-01"       # the first day of the out-of-sample window
prior_variants = 5             # needed: the variants tried before this file

# optional
parent = "value-rank-sp500-v0" # the id that this one changes
family = "value-rank-sp500"    # the family of the variants (default: the family of the parent, else the strategy name)
oos_prior_exposure = true      # results of the window were seen before this file
oos_inherits = "value-rank-sp500-v0"   # the window of the parent (it must be the parent)
start = "2018-01-01"           # test period; then --start and --end are taken from the file
end = "2022-12-31"
success = "text: the criterion, written before the run"

[params]                       # optional; keys: paths, seed, table, rebalance, cost_bp
paths = 300
seed = 7
rebalance = "monthly"
cost_bp = 10
```

The run uses the strategy, the params and the universe of the file. A value on the command line that differs from
the file is an error. `--start` and `--end` stay on the command line unless the file has them.

Until point-in-time index members exist, each result carries the label "universe: names of today (survivor bias)".

## The poison check

`--poison-check D1,D2` runs the strategy again with each close and each fact from D on multiplied by a factor that differs by
ticker. The target weights of each rebalance up to D (D included) and the trades before D must stay equal. D must be a
rebalance day. Use two days: one in the 10-K season (Feb to Mar) and one in the second half of the year. The check samples the given days only: a leak on another rebalance day escapes it. A look-ahead of a few days is caught only close to a given day, so give days that are near the filing seasons of the test period. It does not cover the
GSW curve, the parameter table and the tickers file.
