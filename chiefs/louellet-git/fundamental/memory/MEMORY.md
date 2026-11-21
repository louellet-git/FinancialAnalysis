# Dashboard: fundamental (louellet-git)
For the next session of this head: the judgments about value that are not in the code, and what the
person decided. Keep this file below 60 lines. Started 2026-11-20. Dream pass: 2026-11-21 19:35 UTC.

## Goal now and subsequent steps (`RECORD.md` has the words; the board and `EPICS.md` have the state)
1. FIRST (order until 2026-11-25): the week report, 3 pages or less, S&P 100 then S&P 10. It WAITS for Luc's
   go-ahead (he has the preliminary results). State, branches, commands, numbers: page `attribution-state.md`.
2. Then the T1 revenue fix (`claude/t1-revenue-band`, paused; hold-out `--cut 2011 --base-to 2014` unused).
3. After the reset: the trade-level engine (`claude/trade-level-backtest`, paused), company-type models, JST reader.
4. No weekly trade: no rule has passed its backtest (PR 136); Luc: "No trades until T1 passes".

## Decisions of this head (records in `docs/assumptions/`; no reason uses the price)
- L = min(10-year zero yield, 4.0%), NOMINAL (A-001). Acquisition charge = w x expected spend (A-002).
- The value at the means is deterministic and is NOT the expected value (the mean is higher).
- Attribution: math parts (facts, time, rates) in `fa dcf attribute`; implied parts (premium, growth) split
  into common and company; the common view comes from the Economics engine (`fa macro view`), else the list median.
- Reading of the week to 2026-11-20 (preliminary): a correction of prices that were expensive against 2019-2026;
  not extreme against 1871 on (lowest quarter); no buy signal for the list. NVDA is company-specific.
- NVDA (PR 106): Luc approved the MERGE, NOT the value. Never say that he approved the value.

## Lessons (tested)
- A one-week split between rates and premium misleads when stocks react late: run the prior window too.
- Two supports from one price and one model are ONE fact; give a support that the model did not make.
- A short history window overstates how rare a level is (8 years: lowest; 155 years: lowest quarter).
- News numbers are not ours: cite them as reported, or leave them out (the December odds, the "hot" CPI).
- The price files are split-adjusted to TODAY: a past value per share needs the share basis of the vintage.
- A stale close (older than the SPY close of the date) mixes dates in a sum: leave the row out and name it.
- Point-in-time mapping needs the 10-K fill only when its filing was public (PR 152); the engine never patches data.
- A backtest uses a table cut before its first year; compare a strategy with the equal weight of the SAME list.
- Commit only after the build and the tests pass in the same command chain (3924a05 did not compile).
- Stage explicit paths while an agent works in the worktree (`git add -A` took an agent's code into a board commit).
- Agents: say "Edit tool only"; check two numbers of each agent before a gate; clock times from `date -u`.

## Standing words of the person
- Preliminary results first (chat and a results file); a report only after his go-ahead; conclusion inside 3 pages.
- Macro "why" questions go to the Head of Economics; give Luc that answer with its author.
- Each head asks "does it make sense?" before a result goes to him.
- No "possible reason, not proven": each reason is a claim with its test; the result is proven, refuted, or "tested, no cause found".
- Each document for him has a PDF (`analysis/priors/pdf/`; pandoc to HTML with `style.css`, then a headless Chromium print).
- What is repeatable goes into the correct engine (table on `T-20261121-dcf-engine-integration`).
- Each pause puts the state on the board. A data problem is fixed by its owner (Technology), not left out.
- "Backtesting is integral": a point-in-time backtest before each model change or strategy; trades only from it.
- Each model has a model document with a Backtest line, and each assumption a record (A-001 to A-099 are mine).
- The stochastic DCF engine, for stocks only. Agents lean and of quality; fewer agents (token limit, 71% used).
- A merged ticket moves to `PLANNING/board/done/`; never delete a ticket. The head merges after a gate pass.

## Tested rules and contacts
- Heads by session title (send_message; the id comes from `list_sessions`, never in a file): FA-L - Head of
  Technology, of Quality and Risk, of Economics, of Quant Research, of Staff.
- Head of Staff: one short message when a result is done (it reads a result before Luc), an owner decision, a
  conflict, a long block. No status reports. A permission refusal goes to Technology.
- No push to a branch under review until the verdict. A documents-only pull request has no check run.
- The data store is private: attach `louellet-git/fa-data` (add_repo, read), then clone. No Cboe automatic reads.
- The push of `claude/sp100-week-attribution` was refused by the permission system: do not retry; its commits
  are in `claude/premium-series`.

## Pages
- `attribution-state.md`: the week study (branches, commits, commands, numbers, open items).
- `responsibilities.md`: the area of this head and its boundary with the Head of Quant Research.
