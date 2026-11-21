---
id: T-20261121-fred-local
title: Replace the FRED files with their original public sources
area: data
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: technology
order: 1
branch: claude/fred-primary
created: 2026-11-21
acceptance: Each series of `macro/fred` and `bonds/fred_*` has an original source with its terms, or a recorded decision to drop it; a series from a public-domain source equals the FRED values on the overlap; the readers use the new files; then the store has no FRED file.
---
The FRED terms (read 2026-11-21) do not permit us to store, cache, archive, give to third parties or
republish FRED content without permission. The public store has 26 files in `macro/fred/` and 26
files in `bonds/` (`fred_*.csv`). The owner approved the move to local copies (2026-11-21).

Steps: (1) `fa macro get` for the series list of `macro/fred/SOURCE.md`; (2) the readers use
`data/local/`; (3) remove the files from the store, keep a source note; (4) update
`docs/DATA-SOURCES.md` and the store README; (5) tell the heads.

Change of plan (2026-11-21). The FRED site gives no reply to our honest client name (7 test
requests; the default curl name works). A different name only to pass that filter gets around a
block, so we do not do it. The FRED API terms also prohibit storage. Thus `fa macro get` (branch
claude/fred-local, not merged) cannot work. The new plan: get each series from its original source
(Treasury, Fed Board, BLS, BEA, Census, NBER, Chicago Fed). Public-domain data can stay in the store.
Moody's and ICE series have no free original source: find a substitute or drop them. Until a
series has a replacement, its FRED file stays in the private store.
