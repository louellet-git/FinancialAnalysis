# Dashboard: economics (louellet-git)
What the next session of this head must know. Keep this file below 60 lines. Started 2026-11-21.
Rules: `chiefs/ORIENTATION.md`, section 8. Decisions of the person: `../RECORD.md`.

## Goal now
- JST cross-country replication (15 countries; local copy only, RECORD 14:37 UTC): design frozen at 31ed025
  (#138). Fundamental Research builds the reader and the scoring. BEFORE any score: commit record R1 in
  section 9 (convention check, excluded countries, m, dropped pairs, war-list removals, reader commit).
  Local data: /home/user/jst-local (never in a repo; a new session must fetch it again from macrohistory.net).
- Brief with the published cycle models (`T-20261121-economics-brief-cycle-models`), in progress.
- Done on main: brief #116, stress windows #118, US design #120/#126, historian #124, US findings #129.
  US result: P1 and the engine pass (engine p 0.011, OOS R2 0.119), weak (about 12 windows, effect after
  1953). The stock-level DCF rule has NO edge after the split fix (Fundamental Research, 2026-11-21).

## Next steps
- Scenario set for Quant Research, only if that head asks.
- After the JST run: the findings file (name the run of record and any superseded run; R-8).

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
- A choice made after a result is seen: write it as a dated record, marked "after the run", with each
  choice listed; the auditor checks git for it. Two scored runs: name the run of record and the superseded one.
- Parallel work is allowed for independent files (orientation section 3, 2026-11-21).

## Pages
- `../industry-2026-11-21.md`: what an economics team does at other firms; three proposals (all done).
