# Dashboard: economics (louellet-git)
What the next session of this head must know. Keep this file below 60 lines. Started 2026-11-21.
Rules: `chiefs/ORIENTATION.md`, section 8. Decisions of the person: `../RECORD.md`.

## Goal now
- Long-history test of the market-level DCF (owner word 2026-11-21, via Fundamental Research). The design is
  frozen on main: `analysis/macro/2026-11-21-long-history-design.md` (2ffd392, records R1-R2 in #126).
  Fundamental Research builds the scoring in `fa dcf market` (branch `claude/dcf-market-level`). My part:
  check the output (auditor recalculation of X and P1 on sample months, via Quality and Risk), then write
  the findings file with the hash 2ffd392. Any change after a result = a new dated record (section 9).
- Done and on main: first brief (#116), stress windows (#118), design (#120), historian agent and note (#124).

## Next steps
- A new conditions brief with the published cycle models (NFCI, SAHMREALTIME, T10Y3M, payrolls), now in the
  store (`macro/fred/`, store 8952a5a). Labor and GDP are latest values, not vintages: say so.
- Scenario set for Quant Research (the stress dates are done), only if that head asks.

## Team
- `macro-economist`: briefs. `market-historian` (new, owner word 2026-11-21): precedents of an unusual
  situation, "a lens, not a rule". A new agent type loads only in a new session: until then run a
  general-purpose agent with the agent file as its brief.

## Tested rules
- Some `fa` commands download a file for a date that is not cached, and `fa rates nyfed` is a live call.
  Briefs: direct file reads; check `git -C data/raw status --short` after a run.
- A brief that allows web search lets an agent fetch data files too. Write "pages only, no data files".
- The store copy: `data/raw` has a tracked `.gitkeep`; clone to a separate folder and copy it in.
- Quality and Risk asks each time for: the main finding first, the store commit, a source column, an exact
  numbered rule, a source or "memory, not checked" for each figure. Put these in each brief.
- A design that a test uses: pre-register it (one primary test, a rejection rule, the bootstrap, the
  look-ahead rules, the sample end by a rule, a fixed Holm family). The review needs two cycles otherwise.
- Agents overrun word limits (1,300 to 2,600 against 1,000 to 1,500). Give the limit; tables are accepted.
- The head merges its own PR after the gate and a green check (orientation section 6).

## Pages
- `../industry-2026-11-21.md`: what an economics team does at other firms; three proposals (all done).
