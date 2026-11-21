# Dashboard: quant (louellet-git)
What the next session of this head must know. Keep this file below 60 lines. Started 2026-11-20.
Rules: `chiefs/ORIENTATION.md`. Decisions of the person: `../RECORD.md`. Order: `PLANNING/board/EPICS.md` (E4).

## State on 2026-11-21 21:00 UTC
- Token rule (Head of Staff, with the person): until Wednesday 2026-11-25 11:00 Toronto time, finish open work only;
  the algo engine after the live run of Monday; vol models and other new work after the reset.
- PR 131 (`fa algo`, branch claude/quant-algo-paper): gate passed; merge AFTER the person's live run of Monday
  2026-11-23; then the Head of Technology sets up the e2-micro. A reminder fires 2026-11-23 21:30 UTC.
- Merged today: 147, 151 (model documents, records A-100 to A-154); 148 (alpha-1 stage 1: STOP at gate 1);
  158 (SABR); 183 (arbitrage register and `fa arb run`); 187 (`fa recon history --lasting`); 189 (`fa selftest
  golden`); 196 (trade grids: stress windows from the file, --lags, --cut-scan, --price-grid).
- Ledger rows for today's runs wait until PR 131 merges (its branch has row 6; a row now makes it conflict).
- The branch claude/quant-rates-dup is dropped (the duplicate report was false): do not merge it.

## Next (after PR 131 merges; tickets on main)
- `T-20261121-arb-invariants`: the continuous capture on the e2-micro (design B in
  `analysis/priors/continuous_option_data_2026-11-21.md`; own process, a share of 60 of the 200 Alpaca requests a
  minute; raw data stays on the VM); then break persistence, then the planned relations (VIX-REPLICATION first).
- `T-20261121-algo-paper-orders`: controls 1 to 8, `--send` only on the laptop or the e2-micro, never in the cloud.
- `T-20261121-quant-engine-integration`: next rows by value: 13 and 14 (a run log and a trial count for
  `fa trade backtest`), 18 (`--seeds`), 20a (the VIX relation), 21; findings P5 and G1.
- `T-20261121-quant-doc-findings`: B1 to B4 before any new bot result.

## Rules learned (tested)
- Run the real command myself before I pass an agent report on; check a defect report of a head against the data.
- A formula needs a fixed answer from an independent source at each branch, and a mutant that must fail.
- A new ticket goes on main before its work (`fa board check`). One branch for each ticket, from origin/main.
- Do not push to a branch in review; do not push to an open code PR (each push runs the paid check).
- `fa selftest golden` before a code PR (it replaced the hand diffs against main; Linux recorder only).
- No automatic Cboe read (terms, 2026-11-21). Alpaca: personal use, no copy in git or the store; no raw quote
  in a committed file. Paper orders only, under QUALITY-STANDARDS section 8.
- A break on indicative quotes measures the data, not the market (143 SPX breaks: 59 above $1 a share).
- Messages: results, blocks over half a day, decisions of the person and conflicts go to the Head of Staff;
  permission refusals go to the Head of Technology. No status reports.

## Standing words of the person
- "You don't need me to tell you to continue unless I tell you to stop": report, then take the next ticket.
- "Before asking me anything, please look into it": research first; a question carries a recommendation.
- "for quant work, we're going to require as continuous as possible" (2026-11-21).
- "whatever is repeatable ... is built into the correct engine" (2026-11-21).
- "the agents should not be overly sophisticated just overall lean and of quality". Windows computer.

## Pages
- `../industry-2026-11-21.md`, `responsibilities.md`, `arbitrage/register/README.md`,
  `analysis/priors/arb_invariants_*.md`, `analysis/macro/2026-11-21-stress-windows.md`.

Dream pass: 2026-11-21 21:00 UTC. Next: before the next compaction.
