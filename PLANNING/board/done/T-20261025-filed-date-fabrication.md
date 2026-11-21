---
id: T-20261025-filed-date-fabrication
title: Stop filed_date fabrication in importer
area: importer
status: done
owner: louellet-git
requested_by: louellet-git
head: technology
order: 4
branch: claude/filed-date-sparse
created: 2026-10-25
acceptance: Rows without a matched revenue fact no longer get filed_date = period_end, and a test covers the backtest point-in-time logic on sparse rows.
merged: pull request 101 (branch claude/filed-date-sparse)
---
Folds into T-IMP-1 (PLANNING/tickets.md).
Partly done on branch `claude/admiring-allen-f5aaeq`, 2026-11-19: a row with no revenue fact gets the latest filing date of its facts, not the period end (test in `tests/test_importer.py`). Open: the backtest test on sparse rows.

Done on 2026-11-21: the importer part is in `rust/mapcheck/src/mapper.rs` (a row with no revenue fact gets the latest filing date of its facts; test `filed_date_comes_from_the_revenue_fact`). The backtest test on sparse rows: `a_sparse_row_is_visible_only_from_its_filing_date` in `rust/src/dcf/tests.rs`. Found: a 10-K/A filed after a vintage hides its 10-K (ticket `T-20261121-backtest-10ka-vintage`).
