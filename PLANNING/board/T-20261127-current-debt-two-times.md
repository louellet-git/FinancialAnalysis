---
id: T-20261127-current-debt-two-times
title: The quarter balance sheet counts the current debt two times when the two tags differ by a small amount
area: importer
status: backlog
owner: louellet-git
requested_by: louellet-git
head: technology
order: 4
branch:
created: 2026-11-27
acceptance: For IBM on the balance sheet of 2026-09-30, `fa dcf auto IBM --cache data/raw --explain` shows total debt of $63.81B (not $70.83B), a test with a fixed answer covers the case, and the count of S&P 500 tickers that the change moves is in the pull request.
---
Found by the Head of Staff on 2026-11-27, in a price check of IBM for the owner.
The 10-Q of 2026-09-30 has `ShortTermBorrowings` $7.013B and `LongTermDebtAndCapitalLeaseObligationsCurrent`
$7.009B. These are one amount. The rule "same value, counted one time" needs equal values, so the two
values were added: debt $70.83B. The tag `LongTermDebtAndCapitalLeaseObligationsIncludingCurrentMaturities`
gives $63.81B. The effect on IBM is about $7 a share in the automatic model.
Second item, small: `fa verdict IBM --rf-date today` stops with an error when the BLS source refuses the
request (daily limit). The inflation lines are not necessary for the verdict. Make the verdict continue
without them and say so in the output.
