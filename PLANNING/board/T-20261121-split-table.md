---
id: T-20261121-split-table
title: Give the store a split table, so that a past price and a filed share count are on one basis
area: importer
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: technology
order: 1
branch: claude/split-table
created: 2026-11-21
acceptance: `fa splits get` writes `_corporate/splits.csv` (derived facts only, checked against the SEC split-ratio tag) for the three lists; `market::splits::split_factor` puts a share count on the basis of a later date; the trap is written; the gate passes.
---
A CRIT of the strategy gate (Head of Quality and Risk, 2026-11-21): the store prices are adjusted for splits to
today, the SEC share counts are as filed, so a past value per share is inflated by each later split (BKNG: store close
72.94 on 2018-01-05, real about 1,823; 25:1 on 2026-04-06). In the weekly strategy test this made the whole edge.

Done on the branch: `fa splits get` (Alpaca corporate actions; 41 requests for 2,003 tickers), store commit f6e66fc
(260 rows; sec_check match 153, no_sec_fact 95, mismatch 12), `rust/src/market/splits.rs` (`Split`, `load_splits`,
`split_factor`, names agreed with Fundamental Research, which corrects the models with them), the trap file, and the
`docs/DATA-SOURCES.md` rows. Public copy: a decision of the Head of Quality and Risk (derived split facts only).
Limits: no split before 2016 (Alpaca); AA has a suspect second row (2016-11-01, probably the Alcoa separation).
