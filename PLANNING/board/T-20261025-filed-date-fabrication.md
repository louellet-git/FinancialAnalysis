---
id: T-20261025-filed-date-fabrication
title: Stop filed_date fabrication in importer
area: importer
status: backlog
owner: louellet-git
branch:
created: 2026-10-25
acceptance: Rows without a matched revenue fact no longer get filed_date = period_end, and a test covers the backtest point-in-time logic on sparse rows.
---
Folds into T-IMP-1 (PLANNING/tickets.md).
