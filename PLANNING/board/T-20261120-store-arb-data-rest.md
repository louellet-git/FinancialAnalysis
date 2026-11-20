---
id: T-20261120-store-arb-data-rest
title: Cache the NY Fed rates and the Cboe VX contract files for offline checks
area: importer
status: backlog
owner: louellet-git
requested_by: louellet-git
head: technology
order: 6
branch:
created: 2026-11-20
acceptance: `fa recon explain` and `fa futures check` run on a copy of the data store with the network blocked.
---
The rest of `PLANNING/board/done/T-20261120-store-arb-data.md`. Not needed now (the Head of Quant Research,
2026-11-20): only the carry column of `fa recon explain` reads TGCR.

1. `fa rates nyfed` answers live and writes no cache file. Add a cache file (SOFR, BGCR, TGCR, EFFR, SOMA) and a
   `fa sync` data set or an item of `rates`.
2. `fa futures check` gets one Cboe file for each VX contract (`VX_<expiry>.csv`); `futures/` of the store has
   the settlement file only. Cache the contract files.
