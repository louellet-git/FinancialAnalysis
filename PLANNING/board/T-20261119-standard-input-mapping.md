---
id: T-20261119-standard-input-mapping
title: Standardize the mapping from XBRL tags to DCF input variables
area: importer
status: in-progress
owner: louellet-git
branch: claude/admiring-allen-f5aaeq
created: 2026-11-19
acceptance: On a panel of 30 US tickers, each core DCF input has a value from a recorded source for 90% or more of the non-financial companies, each annual row is a full fiscal year, and `python -m scripts.mapping_audit` prints the result.
---
Owner request, 2026-11-19: an input can have a different tag name for a different ticker. The input variables that go into the DCF must be standard, and at the most granular level possible.

Audit of the mapping on 2026-11-19 (30 tickers, latest fiscal year): `interest_expense` 8 of 30, `long_term_debt` 22 of 30, `ppe_net` 20 of 30, `da_total` 20 of 30, `operating_income` 22 of 30. Rows marked annual that contain a quarter: GE 21 of 35, CAT 12 of 32, ACN 12 of 32.

Parts:
1. Fact selection by duration (annual = a full fiscal year; quarter = one quarter).
2. One registry of standard variables: aliases in a fixed sequence, formulas as fallback, a recorded source for each value.
3. Checks: accounting identities, and a list of material tags that have no mapping.
4. Later (Ticket 7 in the backlog): filing-level XBRL for segment data and company-specific tags.
