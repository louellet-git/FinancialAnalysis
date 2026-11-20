# Token ledger: fundamental (louellet-git)

| Run | Date | Scope | Subagent tokens (approx) | Notes |
|---|---|---|---|---|
| Conventions decision, part 1 | 2026-11-20 | two new defaults of the automatic model (`long_run = 0.040`, `acq_w = 1`), tests, records A-001 and A-002, model document; no full data run (partial cache) | ~90k (1 Sonnet subagent) + ~28k (fetch of the FOMC table) | orchestrator read the diff, ran both checks (554 + 17 + 1 tests, IBM headline); not reviewed, not merged |
| Market-implied command, parts 1 to 3 | 2026-11-20 | `fa dcf implied`: value at the means, three searches, group quartiles, list premium; 10 new tests; run on 218 tickers of the partial cache | ~161k (1 Sonnet subagent) | orchestrator ran the tests (564 + 17 + 1), the IBM headline and the command two times (same bytes); not reviewed, not merged |
| Conventions decision, part 2 | 2026-11-20 | full runs on the data store (data of 2026-11-19): screen of `sp500.txt`, point-in-time test, return test, old and new defaults; numbers in A-001, A-002, the model document and the architecture page | ~122k (1 Sonnet subagent) | orchestrator recomputed the two screen medians and ran each default alone (0.5605, 0.382) |
