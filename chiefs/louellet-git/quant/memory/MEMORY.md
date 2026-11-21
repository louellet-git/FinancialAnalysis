# Dashboard: quant (louellet-git)
What the next session of this head must know. Keep this file below 60 lines. Started 2026-11-20.
Rules: `chiefs/ORIENTATION.md`, section 8. Decisions of the person: `../RECORD.md`.

## Goal now
- Merged on 2026-11-21 (pull request 100): vol-1 (fa vol fit), vol-7 steps 2-3 (fa options archive), arb-1b,
  arb-2, arb-4 step 1. Open in my queue: `arb-4` (steps 2-3: VIX parity forward against VX, ZQ/SR3 strip;
  the two step-1 notes are in the ticket), `arb-3` (box rate, SPX against SPY, two-day `--compare`).
- Data: the Technology scheduled task copies SPX, SPY, VIX each market day from 2026-11-23 (20 days).

## Next steps
- From 2026-11-24 (two copied days): `arb-3`; the out-of-sample test of vol-1 (fit day D, price D + 1).
- Then `vol-2` (SABR, local volatility) on the new `volmodels` base.
- Now due: a test audit with `test-engineer` (several merges in my area; `chiefs/ORIENTATION.md`, section 3).
- Findings for the person: no tradable break so far; the SPX feed is indicative (not executable).

## Tested rules
- `fa recon bonds`, `fa recon explain` and `fa futures check` download each missing file (two agent downloads on
  2026-11-20). Brief: "check the cache first". Agent file `rates-credit-quant.md` changed for this.
- An agent report can be wrong on one fact (vol-1: "converged" against the program's "NOT converged"):
  run the real command myself before I pass a number on.
- A merged ticket moves to `PLANNING/board/done/` with `status: done` and `merged: <pull request>`; never delete it.
- Open a pull request only after the gate verdict, and merge `origin/main` once just before: one check run.
- 2026-11-21, the person: a separate branch for each task (`claude/quant-<ticket>`, from `origin/main`).
  `claude/upbeat-cori-a4ykf5` keeps the work that was on it before this answer.
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
- 2026-11-20: permission requests go to the Head of Technology ("If there are permission requests, they go to
  the tech head"). Send it the command, the reason and the refusal text. It does not run a refused command; it
  says if a safer form exists, and the person approves in my session. Do not run a refused command myself.
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
