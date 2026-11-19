---
id: T-20261119-jnj-pipeline-defects
title: Correct the importer defects that the JNJ run found
area: importer
status: review
owner: louellet-git
branch: claude/admiring-allen-f5aaeq
created: 2026-11-19
acceptance: A JNJ import has no quarterly rows marked as annual, has annual revenue from FY2016, and the JNJ backtest runs.
---
`analysis/JNJ/FIRST-PASS.md` defects 1, 2 and 3. Defect 4 is T-IMP-1 (backlog). Defect 5 (label of `run_dcf`) is corrected. Defect 6 is ticket `bridge-per-ticker-rule`.
Done on the branch, 2026-11-19, with ticket `standard-input-mapping`: the JNJ import has 12 annual rows (FY2014 to FY2025), each a full fiscal year, and `python -m scripts.backtest JNJ` runs (9 of 16 results in the band).
