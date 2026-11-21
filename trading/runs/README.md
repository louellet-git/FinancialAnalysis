# Run logs

`fa portfolio backtest --hypothesis <file>` writes one file for each run: `trading/runs/<id>/<UTC time YYYYMMDDTHHMMSSZ>.json`.
One file for each run, so two heads never write the same file.

## Rules

- Add only. The command never replaces a file. Commit each run file, also a bad result.
- The number of run files is the variant count. The command prints it: "runs of this hypothesis: N; runs of strategy S: M"
  (N counts the files of the id, M counts the files of all ids with the same strategy; this run is included).
- A deleted file is a hidden variant. Do not delete or edit a run file. Correct an error with a new run.
- A run log is not a decision. A result is usable for a decision only after the gate of `docs/QUALITY-STANDARDS.md`.

## Content of a file

The code commit and a flag for uncommitted changes in the code (`code_dirty`), the commit of the data store (null if the
cache is not a git repository), the SHA-256, the commit and the commit time of the hypothesis file, the full config, the
flag `oos_read`, the results at 0x, 1x and 2x cost (total, annual return, volatility, maximum drawdown), the result of
the poison check if it ran, the variant counts, and the labels "universe: names of today (survivor bias)" and
"market prices (closes), not a model price".
