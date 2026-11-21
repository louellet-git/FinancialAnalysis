# Dashboard: economics (louellet-git)
What the next session of this head must know. Keep this file below 60 lines. Started 2026-11-21.
Rules: `chiefs/ORIENTATION.md`, section 8. Decisions of the person: `../RECORD.md`.

## Goal now
- `T-20261121-economics-first-brief` (branch `econ/first-brief`): industry note and the first brief
  (`analysis/macro/2026-11-21-conditions-and-the-oct-nov-drawdown.md`). It answers the question of the
  Head of Fundamental Research (premium against AI-capex growth, 2026-10-06 to 2026-11-19), who cites it
  in the findings of `T-20261120-valuation-move-attribution`. Gate: document-reviewer and auditor,
  through the Head of Quality and Risk. After the pass: pull request, then tell Fundamental Research.

## Next steps
- Data request sent to the Head of Technology (2026-11-21): labor, GDP, PCE, NFCI, total OAS, ALFRED
  vintages, 2025 Treasury files, more VX settlement dates. Briefs stay thin until these are in the store.
- Industry note, proposal 3: offer a stress-date list and a scenario set to Quant Research, if it wants it.

## Tested rules
- Several `fa` commands download a missing file when the date is not in the cache (`fa rates inflation
  --date <2025 date>`, `fa futures positions --weeks N` with a new N). Brief: use only dates in the
  cache; check `git -C data/raw status --short` after the run.
- `fa rates nyfed --date <old date>` gives the latest value when the date is not in the cache: treat it as no data.
- The store copy: `data/raw` in a worktree has a tracked `.gitkeep`, so `git clone` into it fails.
  Clone to a separate folder and copy it in.
- An agent brief came at about 1,700 words for "2 pages": give a word limit (about 1,200) in the brief.

## Pages
- `../industry-2026-11-21.md`: what an economics team does at other firms; three proposals.
