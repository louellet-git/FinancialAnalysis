# Hypotheses

A hypothesis file states what a backtest must show, before the first run. The command
`fa portfolio backtest --hypothesis trading/hypotheses/<id>.toml` reads it. Without the file, the command
runs as an exploratory run: no run log, and the result is not usable for a decision.

## Rules

- Register before the first run. Commit the file first: the command refuses a file that is not in HEAD or that has a change.
  The commit hash and the commit time of the file go into the run log (the proof of order).
- Do not change a file after its first run. A change is a new id (and a new file). Name the old id in `parent`.
- The file name is `<id>.toml`. The id has letters, digits, `-`, `_` and `.` only.
- The variant count does not reset with a new id: the run log counts the runs of the id and the runs of the strategy
  (all ids). Do not hide a variant by a new id.
- Write the out-of-sample start (`oos_start`) before the first run. A run that ends after it is flagged `oos-read`.

## Form

```toml
id = "value-rank-sp500-v1"
date = "2026-11-21"            # the registration date
hypothesis = "text: what must the run show, and against which line"
strategy = "value-rank"        # value-rank or momentum
universe = "rust/mapcheck/universe/sp500.txt"   # a tickers file (path from the repository root)
oos_start = "2023-01-01"       # the first day of the out-of-sample window
parent = "value-rank-sp500-v0" # optional: the id that this one changes

[params]                       # optional; keys: paths, seed, table, rebalance, cost_bp
paths = 300
seed = 7
rebalance = "monthly"
cost_bp = 10
```

The run uses the strategy, the params and the universe of the file. A value on the command line that differs from
the file is an error. `--start` and `--end` stay on the command line (the run log keeps them).

Until point-in-time index members exist, each result carries the label "universe: names of today (survivor bias)".
