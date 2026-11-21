---
id: T-20261121-rates-append-dup
title: Stop the Treasury curve refresh from adding a date that the file has
area: engine
status: backlog
owner: louellet-git
requested_by: louellet-git
head: quant
order: 1.0
branch:
created: 2026-11-21
acceptance: A refresh of `rates/treasury_nominal_<year>.csv` replaces or skips a date that the file already has, a test with a file that already has the last date proves it, and `fa rates` gives the same output as before.
---
Found by Fundamental Research and reported by the Head of Technology (2026-11-21): a local refresh (`fa rates` or
`fa sync --only rates`) appended the 11/20/2026 row of `data/raw/rates/treasury_nominal_2026.csv` a second time, with
the same values. The store copy has it one time, so the defect is in the append path of `rust/src/rates.rs`. The readers
keep one row today, so no result is wrong. Agent: `rates-credit-quant`. Gate: `code-reviewer`.
