---
id: T-20261121-fred-local
title: Keep the FRED files local only, with one fetch command
area: data
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: technology
order: 1
branch: claude/fred-local
created: 2026-11-21
acceptance: `fa macro get` downloads each series of `macro/fred` into `data/local/macro/fred/` (it refuses a fa-data checkout); the market measurement reads that folder; `fa bonds` writes its FRED files under `data/local/bonds/`; after the merge the store has no FRED data file, only a source note; a test for each change; the real command runs.
---
The FRED terms (read 2026-11-21) do not permit us to store, cache, archive, give to third parties or
republish FRED content without permission. The public store has 26 files in `macro/fred/` and 26
files in `bonds/` (`fred_*.csv`). The owner approved the move to local copies (2026-11-21).

Steps: (1) `fa macro get` for the series list of `macro/fred/SOURCE.md`; (2) the readers use
`data/local/`; (3) remove the files from the store, keep a source note; (4) update
`docs/DATA-SOURCES.md` and the store README; (5) tell the heads.
