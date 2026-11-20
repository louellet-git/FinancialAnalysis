---
id: T-20261025-filed-date-fabrication
title: Stop filed_date fabrication in importer
area: importer
status: backlog
owner: louellet-git
head: technology
order: 4
branch:
created: 2026-10-25
acceptance: Rows without a matched revenue fact no longer get filed_date = period_end, and a test covers the backtest point-in-time logic on sparse rows.
---
Folds into T-IMP-1 (PLANNING/tickets.md).
Partly done on branch `claude/admiring-allen-f5aaeq`, 2026-11-19: a row with no revenue fact gets the latest filing date of its facts, not the period end (test in `tests/test_importer.py`). Open: the backtest test on sparse rows.
