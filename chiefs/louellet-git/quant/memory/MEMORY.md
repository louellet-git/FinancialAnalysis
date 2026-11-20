# Dashboard: quant (louellet-git)
What the next session of this head must know. Keep this file below 60 lines. Started 2026-11-20.
Rules: `chiefs/ORIENTATION.md`, section 8. Decisions of the person: `../RECORD.md`.

## Goal now
- Ticket `T-20261120-option-history-root-defect` (order 1). The code is on the branch
  `claude/upbeat-cori-a4ykf5` (commit c6ad2c4). Open: the real-day smoke test (see "What stops
  work"), the gate pass of the Head of Quality and Risk (request sent 2026-11-20), the pull request.

## Next steps
- Order 2 `vol-1-heston-bates`, order 3 `vol-7-chain-copy`. The data store has no option history:
  send the data request to the Head of Technology when a ticket needs it.

## Tested rules
- 2026-11-20: this session can push only to `claude/upbeat-cori-a4ykf5`. Use it for each ticket, one
  ticket at a time, until the person gives a different branch.
- 2026-11-20: the proxy of this environment adds only the header `APCA-API-SECRET-KEY` for
  `data.alpaca.markets` (`get_environment`, network secrets). The key id (`APCA_API_KEY_ID`) is not
  in the environment, so each Alpaca request gives HTTP 401. A dummy key id does not work.
- Two functions group option quotes: `fill_from_mids` (live index chain) and `day_snapshot` (history
  closes), in `rust/src/market/option_history.rs`. A change of the grouping changes both.

## Standing words of the person
- 2026-11-20, on the teams: "the agents should not be overly sophisticated just overall lean and of
  quality, not obsessive and tight fisted".
- The GitHub Actions use was too high: few pull requests, both test suites on this machine first
  (`chiefs/ORIENTATION.md`, section 6).
- The computer of the person is a Windows laptop. Give commands for PowerShell.

## Pages
- No page.

## Dream pass
On the word of the person ("clean up memory" or "dream"). Last pass: none.
