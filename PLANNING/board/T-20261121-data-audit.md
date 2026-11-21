---
id: T-20261121-data-audit
title: Full data audit of the store (defects of the kind found on 2026-11-21), with an owner for each defect
area: importer
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: technology
order: 1
branch: claude/data-audit
created: 2026-11-21
acceptance: Each check below has a count on the store, each defect found has an owner and a ticket or a fix, and the S&P 100 defects are listed first.
---
Word of the person (2026-11-21, relayed by the Head of Fundamental Research): "Can the head of tech check if there are
other problems like those" (the split basis, the point-in-time mapping FAIL of V, BRK-B, COP and NEE, the IBM and JNJ
price update).
Checks: (1) point-in-time mapping FAIL where the normal mapping gives WARN (23 sp500 tickers on 2026-11-13; after
`T-20261121-pit-supplement-fill`); (2) price files: missing, no meta, stale; (3a) each universe ticker has a price file,
company facts, a share count at a past date and a CIK; (3b) price gaps against SPY trading days, 2 years; (3c) a one-day
jump above 40% with no split row; (3d) the model share count x price against a plausible market value (class or scale
errors); (3e) two tickers with one CIK beyond the known share classes; (3f) rates and macro files: last date, missing
days, duplicate dates; (3g) template counts ("template new", "history too short").
