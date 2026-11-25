---
id: T-20261123-nasdaq-etf-retry
title: Ask the Nasdaq site again as an ETF when the stock class answers "Symbol not exists"
area: importer
status: done
owner: louellet-git
requested_by: louellet-git
head: technology
order: 1
story: E3.S5
branch: claude/nasdaq-etf-retry
created: 2026-11-23
acceptance: URA, URNM and SMH download from the Nasdaq site; a fixed-answer test; the old paths do not change.
merged: pull request 220 (branch claude/nasdaq-etf-retry)
---
Found on 2026-11-23 for the URA page of the Head of Economics: the Nasdaq site answers "Symbol not exists"
(an error, not an empty reply) for an ETF asked as a stock, so the code never asked again as an ETF. The
ticket was written after the fix (a small fix found in the work; the rule "ticket first" was not kept).
QA gate: PASS. Note for later: the code finds the error by its text; a typed error variant is safer.
