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

Study (2026-11-21, 14 test requests with the honest name). Hosts that answer: federalreserve.gov,
home.treasury.gov, chicagofed.org, oui.doleta.gov. BLS: the daily limit of the shared address. BEA:
a key is necessary. NBER: bots are not permitted on the data files.

| Series | Use | Replacement |
|---|---|---|
| T10Y3M, DFII10, T10YIE | Economics design | Compute from the Treasury nominal and real curves (equal on 3 dates) |
| HQMCB* (8) | `bonds.rs` | Treasury HQM file (public domain; monthly, from 1984) |
| NFCI | Economics design | Chicago Fed CSV, after a read of its terms |
| CPIAUCNS, PAYEMS, UNRATE, CES0500000003 | CPI cross-check; others not used | BLS (loader exists) |
| SAHMREALTIME | not used | Compute from UNRATE |
| TB3MS, GS10 | history notes | Fed Board H.15 (Data Download Program) |
| GDP, PCE series (6), RSAFS, ICSA | not used | BEA, Census, DOL, only when a model needs them |
| ICE BofA (14), Moody's (6), STLFSI4, NBER (3) | `bonds.rs` (ICE OAS, Aaa/Baa); BAA10Y in the Economics design | No public original source. Drop, or find a substitute: the users decide |

Order: (1) Treasury computed series and HQM; (2) NFCI; (3) BLS; (4) H.15; (5) the rest only on
need. The ICE and Moody's drop changes the cost of debt by rating (`--rating`): the Fundamental and
Economics heads decide the substitute.
