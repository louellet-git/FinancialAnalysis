# Token ledger: quality (louellet-git)

| Run | Date | Scope | Subagent tokens (approx) | Notes |
|---|---|---|---|---|
| sic-code gate | 2026-11-20 | `T-20261120-sic-code` (520c0fc): data-quality-analyst + code-reviewer; verdict PASS-WITH-NOTES, 4 MIN notes | about 70k (39k + 31k measured) | 10 SEC requests in a scratch cache; French table 0 differences |
| conventions gate, cycle 1 | 2026-11-20 | `T-20261120-dcf-conventions-decision` (5499292): model-validator + code-reviewer; verdict REVISE (1 MAJ: the acquisition charge is constant for heavy buyers; 10 MIN) | about 162k (124k + 38k measured) | full screen, pit and returns runs on the data store; ROP P50 checked by the head |
| yahoo-429 gate | 2026-11-20 | `T-20261120-sync-yahoo-429` (d76ad1f): code-reviewer; verdict PASS-WITH-NOTES, 5 MIN notes | about 34k (measured) | no network run (a download of many tickers is not for a QA agent) |
