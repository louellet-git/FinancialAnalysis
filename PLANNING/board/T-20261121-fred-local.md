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

Decisions of the users (2026-11-21). Fundamental: no valuation model reads the ICE or Moody's files
(fixed `kd_spread` in each model); HQM plus a fixed step for each rating is enough, with an
assumption record that gives the source of each step. Economics: drop BAA10Y from the premium design
(recorded in the design before any run).

Step 1 done (branch claude/fred-primary). `fa rates derived` writes T10Y3M, DFII10 and T10YIE from the
Treasury curves: 472, 471 and 472 days equal to FRED (2025-01-02 to 2026-11-20), no difference. The
Treasury nominal file starts in 1990, so T10Y3M 1982-1989 is not available. `fa bonds hqm-compare`
reads the Treasury HQM spot files (`.xls`, home.treasury.gov/data/treasury-coupon-issues-and-corporate-bond-yield-curve/corporate-bond-yield-curve):
154 of 154 months equal to FRED HQMCB*YR (FRED HQM is the spot rate, not the par yield).
Next: an HQM fetch and cache step; `bonds.rs` reads HQM from Treasury; the rating step table and its
record; NFCI from the Chicago Fed; then the FRED files and the FRED code leave.

Step 2 done (commit 01383b4, examined after the agent finished; tests 838+17+1, mapcheck 121).
`fa bonds hqm get` caches the 9 Treasury HQM files (data/raw/bonds/treasury_hqm/, `fa sync` key
`hqm`); `fa bonds` reads HQM from them: 514 months of 8 maturities equal to FRED; the output is the
same except the source label. NFCI: the Chicago Fed legal notice says "Users may not reproduce ...
except as expressly authorized"; its permission covers written materials, not for private gain.
So NFCI is not stored; the Head of Economics decides (drop it, or the owner asks for permission).

Steps 1 to 3 merged (2026-11-21, QA PASS-WITH-NOTES cycle 2): the Treasury derived series, the Treasury
HQM files, and the cost of debt on HQM + a Damodaran notch step (records A-200 to A-203). Open: step 4,
CPIAUCNS (BLS), TB3MS and GS10 (Fed Board H.15) for `fa dcf market`; the NBER series (the users decide:
the JST bill rate as a variant); then the removal of the FRED code and files. QA note: record the
Last-Modified of each new HQM file at `hqm get`, and check the availability rule again after three months.
