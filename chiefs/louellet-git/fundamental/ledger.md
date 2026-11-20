# Token ledger: fundamental (louellet-git)

| Run | Date | Scope | Subagent tokens (approx) | Notes |
|---|---|---|---|---|
| Engine accuracy, parts A to C | 2026-11-20 | A floor of the base-year growth (default on), B narrower margin shock (option only, refused as default), C mean of the value with its standard error; `fa dcf pit --half`, `--diag` | ~194k (1 Sonnet subagent) | orchestrator ran the tests (558 + 17 + 1), the IBM headline and the scoring half with and without the floor |
| Engine accuracy, review cycle 1 | 2026-11-20 | V1 the mean is a measurement only; V2 the floor numbers of A-003 corrected and the reason replaced; ten notes (non-finite paths, parser checks, the gap check of `--diag nu`, edge tests) | ~219k (1 Sonnet subagent, all cycles) | orchestrator ran the tests (560 + 17 + 1), the IBM headline, two floor runs of half 0 and the KO mean line |
