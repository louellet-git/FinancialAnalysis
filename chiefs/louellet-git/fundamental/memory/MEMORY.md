# Dashboard: fundamental (louellet-git)
For the next session of this head: the judgments about value that are not in the code, and what the
person decided. Keep this file below 60 lines. Started 2026-11-20. Dream pass: 2026-11-25 16:30 UTC.

## Goal now and subsequent steps (`RECORD.md` has the words; the board and `EPICS.md` have the state)
1. Week study (E2.S1): three one-page pieces are on main and WAIT for the review of Luc; then the S&P 100 part. Do
   not start it before his review. State: page `attribution-state.md`.
2. T1 correction of the projection test (E1.S1; `T-20261121-blind-projection-test`; branch `claude/t1-revenue-band`).
   Luc lifted his pause on 2026-11-25. The rule is chosen (`band_q`, in the ticket). Next: the statistics gate of the
   Head of Economics on the rule text, then ONE score run (`--cut 2011 --base-to 2014`, never run), then the model gate.
3. E3.S5: the miner design and the page "Uranium Price Needed: NexGen" are on main (NexGen needs about US$84 a pound).
   Luc decides on the build (first company NexGen or Denison, not UEC). Then the fund look-through, R6.
4. No weekly trade: no rule passed its backtest (A-009 INVALID, PR 136); Luc: "No trades until T1 passes".

## Decisions of this head (records in `docs/assumptions/`; no reason uses the price)
- L = min(10-year zero yield, 4.0%), NOMINAL (A-001). Acquisition charge = w x expected spend (A-002).
- The value at the means is deterministic and is NOT the expected value (the mean is higher).
- Attribution: math parts (facts, time, rates); implied parts (premium, growth): common from `fa macro view`, else the list median.
- Week to 2026-11-20: a correction of expensive prices, no buy signal for the list; NVDA is consistent with its sector.
- NVDA (PR 106): Luc approved the MERGE, NOT the value. Never say that he approved the value.

## Lessons (tested)
- A one-week split between rates and premium misleads when stocks react late: run the prior window too. Two supports from one price and one model are ONE fact; give a support that the model did not make.
- A short history window overstates how rare a level is (8 years: lowest; 155 years: lowest quarter). News numbers are not ours: cite them as reported, or leave them out (the December odds, the "hot" CPI).
- The price files are split-adjusted to TODAY: a past value per share needs the share basis of the vintage. A stale close mixes dates in a sum: leave the row out and name it. The engine never patches data (PR 152).
- A backtest uses a table cut before its first year; compare a strategy with the equal weight of the SAME list.
- Commit only after the build and the tests pass in one chain; stage explicit paths; agents use the Edit tool only; `date -u`.
- A script sum needs a second calculation; a failed run must fail its test; a rule that can skip must warn (PR 213).
- T1: a change of the weight w of the growth history changes the free cash flow through A-002; the growth data have
  fat tails, so a normal draw with the correct variance is too wide at P10 and P90 (the ticket has the runs).
- A PDF: check its text with `pdftotext` (two dollar signs on a line became a formula). Read the reports of a company
  before you recommend it (UEC). A claim that Luc decided: read his words in the record (a head is not his approval).

## Standing words of the person
- Preliminary results first; a report only after his go-ahead; ask "does it make sense?" before a result goes to him. Macro "why" questions go to the Head of Economics; give Luc that answer with its author.
- Reports for him: one page, a PDF (`analysis/priors/pdf/`), by `chiefs/louellet-git/staff/memory/owner-report-style.md`
  (title, subtitle, intro of 1 to 3 lines, the answer; "Table N" with one line; "consistent with"; no "no cause found").
- What is repeatable goes into the correct engine (table on `T-20261121-dcf-engine-integration`).
- Each pause puts the state on the board. A data problem goes to its owner (Technology). A merged ticket moves to `done/`.
- "Backtesting is integral": a point-in-time backtest before each model change or strategy; trades only from it.
- Each model has a model document with a Backtest line, and each assumption a record (A-001 to A-099 are mine).
- Anthropic (ANT): numbers and words of the filing only, with the disclosure (the agents run on Anthropic models).

## Tested rules and contacts
- Heads by session title (send_message; ids from `list_sessions`, never in a file): FA-L - Head of Technology, of
  Quality and Risk, of Economics, of Quant Research, of Staff.
- Head of Staff: one short message when a result is done (it reads a result before Luc), an owner decision, a
  conflict, a long block. No status reports. A permission refusal goes to Technology.
- Tokens: all projects share one weekly limit; it resets on Wednesday 16:00 UTC. Few agents. At the close of each
  ticket: `fa usage record && fa usage show`, with the table in the done message (E6.S5).
- No push to a branch under review until the verdict. A change under `rust/` or `analysis/` starts a check run (put
  such changes in one pull request); `docs/`, `chiefs/` and `PLANNING/` do not. Run `fa selftest golden` before a merge
  that changes the output of a command (pull request 190 made a case fail on main).
- The data store is private (add_repo read, then clone). No Cboe automatic reads. A push that the permission system
  refuses: do not retry and do not work around it; tell Technology.

## Pages
- `attribution-state.md`: the week study (branches, commits, commands, numbers, open items).
- `responsibilities.md`: the area of this head and its boundary with the Head of Quant Research.
