# Run logs

`fa portfolio backtest --hypothesis <file>` writes one file for each run: `trading/runs/<id>/<UTC time YYYYMMDDTHHMMSSZ>.json`.
One file for each run, so two heads never write the same file.

## Rules

- Add only. The command never creates a second file with the same name, and it changes a file only while its status is
  "started" (the same run writes its final record). Commit each run file, also a bad result and also a "started" file of a run
  that stopped: it is a variant.
- The record "started" is written before any result exists, and no result is printed before it. The final record has the
  status "done" or "failed" (a failed poison check or an error).
- The variant count is printed: "runs of this hypothesis: N (prior variants P + K logged runs); runs of family F: M". The
  logged runs are found in the working tree and in the git history of all branches (also deleted files). `prior_variants`
  of the hypothesis files is added. This count is a warning, not a multiple-testing control (the deflated Sharpe is part 2).
- A deleted file is a hidden variant. Do not delete or edit a run file. Correct an error with a new run.
- A run log is not a decision. A result is usable for a decision only after the gate of `docs/QUALITY-STANDARDS.md`.

## Content of a file

The code commit and a flag for uncommitted changes in the code, the commit of the binary (build time), the commit of the data store (null if the
cache is not a git repository; no path of a machine is logged), the SHA-256, the commit and the commit time of the hypothesis file, the full config, the
flags `oos_read` and `oos_clean` (with the reasons), the parameter table (path, SHA-256, fit window) and the SHA-256 of the tickers file, the results at 0x, 1x and 2x cost (total, annual return, volatility, maximum drawdown), the result of
the poison check if it ran, the variant counts, and the labels "universe: names of today (survivor bias)" and
"market prices (closes), not a model price".
