# Dashboard: quant (louellet-git)
What the next session of this head must know. Keep this file below 60 lines. Started 2026-11-20.
Rules: `chiefs/ORIENTATION.md`. Decisions of the person: `../RECORD.md`. Order: `PLANNING/board/EPICS.md` (E4).

## State on 2026-11-25 19:30 UTC
- Week of 2026-11-25 to 2026-12-02 (Head of Staff): Financial Analysis has 45% of the week; quant with Technology
  about 8%; volatility work and other backlog stay out of this week. Send the Head of Staff only results and questions.
- Goal of the person (2026-11-25, RECORD): paper algo trading with the broker's fills, spreads and fees. Plan:
  `../plan-paper-orders-2026-11-25.md`; facts: `../alpaca-paper-facts-2026-11-25.md`.
- Merged: PR 131 (`fa algo`, simulator) and PR 248 (step A: `--broker alpaca-paper [--send]`, 1 SPY share, limit,
  long only; gate PASS after 3 cycles). `--send` refuses in any Claude Code or cloud session (CLAUDECODE, CCR_*)
  and needs `~/.config/fa/order-host` naming the machine. Never pass --send from a session.
- Control 6 for options is on main (PR 249): long SPY call or put only, 1 contract, limit at most the ask, 7 to 45
  days, a person closes before 15:00 on the day before expiry. Spreads and condors need their own review.
- HOLD step B (1 long SPY option) until the person has run step A with --send for one day or more (Head of Staff).
- Host: `docs/guides/algo-host-gcp.md` (Technology); the binary bin/fa-linux-x86_64 in the private store.

## Next
- Step B after the first --send results; then the daily report of broker fills against simulator fills.
- `T-20261121-arb-invariants`: the continuous capture as a second service on the VM (design B), later.
- `T-20261121-quant-engine-integration`: rows 13 and 14 (run log and trial count), 18, 20a, 21; findings P5, G1.
- `T-20261121-quant-doc-findings`: B1 to B4 before any new bot result.

## Rules learned (tested)
- Run the real command myself before I pass an agent report on; check a defect report of a head against the data.
- A formula needs a fixed answer from an independent source at each branch, and a mutant that must fail.
- A new ticket goes on main before its work (`fa board check`). One branch for each ticket, from origin/main.
- Do not push to a branch in review; do not push to an open code PR (each push runs the paid check).
- A section 8 control beats my brief: read the control text before I brief an agent on order code.
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

Dream pass: 2026-11-25 19:30 UTC. Next: before the next compaction.
