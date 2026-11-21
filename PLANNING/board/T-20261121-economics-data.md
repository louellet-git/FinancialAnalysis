---
id: T-20261121-economics-data
title: Put the macro series of the Head of Economics in the data store
area: importer
status: backlog
owner: louellet-git
requested_by: louellet-git
head: technology
order: 6
branch:
created: 2026-11-21
acceptance: The store has the FRED series below with their dates, the ALFRED vintages of the labor, GDP and PCE series, EFFR and SOFR of 2025, and the Cboe VX settlements of 2026-08-20 and 2026-10-06; `SNAPSHOT.md` lists them; the Head of Economics reads them with no request.
---
Request of the Head of Economics (2026-11-21, not urgent).
- FRED: PAYEMS, UNRATE, ICSA, CES0500000003, GDPC1, PCEPI, PCEPILFE, PCEC96, RSAFS, NFCI, STLFSI4,
  SAHMREALTIME, T10Y3M, BAMLC0A0CM, BAMLH0A0HYM2. ALFRED vintages of the labor, GDP and PCE series.
  FRED needs a key. In a cloud session the key goes through the secure credential form (never a file or a
  chat); ask the person first. Check the FRED terms for a public copy of each series (some ICE series have
  limits), and keep a series out of the public store if the terms do not permit it.
- EFFR and SOFR of 2025: with item 1 of `T-20261120-store-arb-data-rest` (a cache file for `fa rates nyfed`).
- Cboe VX settlement files of more dates (at least 2026-08-20 and 2026-10-06): with item 2 of the same ticket.
- Done on 2026-11-21: the Treasury nominal and real files of 2025 (store commit 33d673b).
