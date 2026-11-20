# Dashboard: quant (louellet-git)
What the next session of this head must know. Keep this file below 60 lines. Started 2026-11-20.
Rules: `chiefs/ORIENTATION.md`, section 8. Decisions of the person: `../RECORD.md`.

## Goal now
- Order 1 (`T-20261120-option-history-root-defect`) is merged (pull request 76, 2026-11-20). The person
  asked for arbitrage work (record, 2026-11-20): the next ticket of the queue is `arb-1-chain-check`
  (order 1.1), in progress.

## Next steps
- `arb-2-rates-recon` (1.2). `arb-3` and `arb-4` (3.1, 3.2) wait for the chain copy (`vol-7`, order 3) and
  for the data ticket of the Head of Technology (`store-arb-data`).
- Open for the person: move `vol-7-chain-copy` before `vol-1` (recommended: the arbitrage tickets need it).

## Tested rules
- 2026-11-20: this session can push only to `claude/upbeat-cori-a4ykf5`. Use it for each ticket, one
  ticket at a time, until the person gives a different branch.
- 2026-11-20: the proxy of this environment adds only the header `APCA-API-SECRET-KEY` for
  `data.alpaca.markets` (`get_environment`, network secrets). The key id (`APCA_API_KEY_ID`) is not
  in a running shell: each Alpaca request gives HTTP 401. The Head of Technology added the variable
  `APCA_API_KEY_ID` to "ALL ACCESS" (2026-11-20); it works after a restart of the session. Set
  `APCA_API_SECRET_KEY` to a dummy value; the proxy replaces it.
- A real-day run that this session cannot do (no key): ask the Head of Technology to run it on the branch.
  It did the SPX and SPY runs of pull request 76 in a few minutes.
- Two functions group option quotes: `fill_from_mids` (live index chain) and `day_snapshot` (history
  closes), in `rust/src/market/option_history.rs`. A change of the grouping changes both.

## Standing words of the person
- 2026-11-20: "You don't need me to tell you to continue unless I tell you to stop". After a ticket, report and
  take the next ticket of the queue with no wait.
- 2026-11-20, on the teams: "the agents should not be overly sophisticated just overall lean and of
  quality, not obsessive and tight fisted".
- The GitHub Actions use was too high: few pull requests, both test suites on this machine first
  (`chiefs/ORIENTATION.md`, section 6).
- The computer of the person is a Windows laptop. Give commands for PowerShell.

## Pages
- `responsibilities.md`: the split of work between Quant Research and Fundamental Research.

## Dream pass
On the word of the person ("clean up memory" or "dream"). Last pass: none.
