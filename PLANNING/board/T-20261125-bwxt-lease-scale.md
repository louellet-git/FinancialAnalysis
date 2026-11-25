---
id: T-20261125-bwxt-lease-scale
title: Find the cause of the BWXT mapping FAIL (operating lease liabilities 44.221B against total liabilities 3.038B)
area: importer
status: backlog
owner: louellet-git
requested_by: louellet-git
head: technology
story: E5.S1
order: 3
branch:
created: 2026-11-25
acceptance: The cause is known (an error of scale of the filer, or a wrong tag or period in mapcheck); `mapcheck BWXT --detail` gives no FAIL, or the FAIL stays with a recorded reason; the 2 earlier fiscal years with the same FAIL are explained.
---
Found in the data audit (`T-20261121-data-audit`). `mapcheck BWXT --detail` on 2026-11-25:
`FAIL exceeds_total operating_lease_liabilities 44.221B > total_liabilities 3.038B [us-gaap/OperatingLeaseLiability]`,
and `WARN earlier_years_fail 2 of 11 earlier fiscal years`. The value is about 1000 times too large, the same kind
of defect as `T-20261121-share-count-scale-guard`. If the cause is the filer, do the two tickets with one rule.
