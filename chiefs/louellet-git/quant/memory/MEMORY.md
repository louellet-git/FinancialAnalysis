# Dashboard: quant (louellet-git)
What the next session of this head must know. Keep this file below 60 lines. Started 2026-11-20.
Rules: `chiefs/ORIENTATION.md` (sections 3 to 8). Decisions of the person: `../RECORD.md`.

## State on 2026-11-21 15:00 UTC (before a compaction)
- PR 131 (`fa algo`, branch claude/quant-algo-paper): gate passed; merge AFTER the person's first live run on the laptop,
  Monday 2026-11-23; then send the merge commit to the Head of Technology (Google e2-micro guide, the person said yes).
  Do not push to an open code PR (any push reruns the check). A reminder fires 2026-11-23 21:30 UTC.
- alpha-1 (branch claude/quant-alpha-1, commit 23bd711): stage 1 says STOP (s05 t 1.35; premium t 1.97 in the
  gate sample); in review by Quality and Risk; no push until the verdict. Then measure model vs real option prices.
- Person, 2026-11-21: research before a question; questions go through the Head of Technology. Three-stage method:
  isolation, bigger data, paper, then improve (each change a counted trial).
- Split look-ahead is fixed in the automatic model (Fundamental PR 136); a value factor has low priority.

## Goal now
- Find and measure arbitrage and reconcile our data (person, 2026-11-20). Done and merged: SPX/SPXW apart (76),
  `fa options check` (84, 100), rates reconciliation steps 1-2 (84, 100), `fa vol fit` Heston/Bates (100),
  `fa options archive` (100), parser and bond tests (111), limits of use (115). No tradable break so far.
- Data: the Technology scheduled task copies SPX, SPY, VIX (Alpaca indicative) each market day from 2026-11-23,
  20 days (to about 2026-12-22). The Cboe host refuses the cloud (HTTP 403): no NBBO copy.

## Next steps (queue `head: quant`)
- From 2026-11-24 (two copied days): `arb-3` (box rate by the NY Fed regression, SPX against SPY, two-day
  `--compare`); the out-of-sample test of vol-1 (fit day D, price D + 1).
- `arb-4` steps 2-3 (VIX parity forward against VX; ZQ/SR3 strip): needs the VIX copies and NY Fed rates.
- Then `vol-2` (SABR, local volatility) on the `volmodels` base.
- About 2026-12-22: the Head of Technology asks me what the 20 days showed; then the person answers whether to
  get real quotes (laptop at 15:45 ET or a paid source). My recommendation so far: no.
- Test audit: next after several more merges (last 2026-11-21: speed proposals refused, suite ~10 s).

## Lessons from this session (tested)
- Reviews found what I missed in each code ticket: a fit that hid non-convergence (vol-1), an 80% check that
  could lock the archive for good (vol-7). For any fit or automatic job: print the status and the way to recover.
- An agent report can be wrong on one fact (vol-1 "converged"): run the real command myself before I pass it on.
- `fa recon bonds`, `fa recon explain`, `fa futures check` download each missing file (two agent downloads,
  2026-11-20): the brief says "check the cache first"; `rates-credit-quant.md` says it too.
- One branch for each ticket (person, 2026-11-21): `claude/quant-<ticket>` from `origin/main`. Parallel agents
  get worktrees from the branch; I bring their work in with a patch or a cherry-pick and test it myself.
- A value factor for a past date: `fa dcf implied --as-of D` or `fa dcf attribute`; `fa dcf auto --as-of` and
  `fa dcf screen --as-of` read later filings (look-ahead; Fundamental, PR 107).
- Alpaca: the proxy adds only the secret header. `APCA_API_KEY_ID` is in the environment "ALL ACCESS" from a
  restart; set `APCA_API_SECRET_KEY` to a dummy value. Without it, the Head of Technology runs a real-day test.
- A backtest of momentum on today's index list has survivorship bias (`fa portfolio backtest` warns of it);
  no point-in-time member list exists in the store.

## Standing words of the person
- 2026-11-20: "You don't need me to tell you to continue unless I tell you to stop": report, then take the next
  ticket with no wait.
- 2026-11-20: permission requests and decisions go to the Head of Technology (`chiefs/ORIENTATION.md`, section 5).
- 2026-11-20: "the agents should not be overly sophisticated just overall lean and of quality".
- The check runs cost money (`chiefs/ORIENTATION.md`, section 6). The person's computer is Windows (PowerShell).

## Pages
- `../industry-2026-11-21.md`: industry practice (SR 26-2, overfitting bar, NBBO data, box rate method).
  Its three changes: real quotes (refused by Cboe; the ask waits), the strategy bar (in
  `strategy-researcher.md`), limits of use (done for `heston-bates.md`; the other models wait for their documents).
- `responsibilities.md`: the split of work between Quant Research and Fundamental Research.
- Stress windows for backtests: `analysis/macro/2026-11-21-stress-windows.md` (Head of Economics; 13 windows).

## Dream pass
Last pass: 2026-11-21 (context at 82% of the compaction threshold). Next: before the next compaction.
