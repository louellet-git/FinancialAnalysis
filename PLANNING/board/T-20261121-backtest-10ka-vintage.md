---
id: T-20261121-backtest-10ka-vintage
title: Keep the 10-K in a backtest vintage that is earlier than its 10-K/A
area: engine
status: backlog
owner: louellet-git
requested_by: louellet-git
head: technology
order: 5
branch:
created: 2026-11-21
acceptance: A vintage earlier than the filing date of a 10-K/A uses the 10-K of that year, a fixed-answer test shows it, and the changed reference results (IBM vintage 2013-03-31, IBM bias, JNJ report figure 13) are recorded again after a pass of the Head of Quality and Risk.
---
Found on 2026-11-21 (ticket `T-20261025-filed-date-fabrication`). `clean_annuals` in `rust/src/dcf/backtest.rs`
keeps the row filed last for each fiscal year, and then `vintage_periods` drops the rows filed after
`as_of`. When a 10-K/A is filed after `as_of`, the vintage loses that year. But the 10-K was
available. Example: the IBM fixture has a 10-K/A of FY2012 (filed 2013-09-01); the vintage of
2013-03-31 uses FY2011 as its base, not FY2012 (10-K filed 2013-02-15).

Fix (tested on 2026-11-21, then reverted): filter by `filed_date <= as_of` before the choice of the
row filed last. The effect on the fixed-answer tests: `backtest_parity` (IBM vintage 2013-03-31 and
the IBM bias by horizon) and `report::tests` (JNJ figure 13, 30 values of the error bars). These tests
compare with the recorded runs of the old Python code, so the new values need a review (gate: model
change) before they replace the recorded ones.
