---
id: T-20261120-dcf-investigation
title: Find why the automatic DCF gives low values, and correct the defects that have a reason
area: engine
status: in-progress
owner: louellet-git
branch: claude/dcf-investigation
created: 2026-11-20
acceptance: A report gives the parts of the difference between the model value and the market price with a number for each part, each defect that was found has a test, and the point-in-time test shows the bands before and after each correction.
---
Owner request, 2026-11-20: an investigator examines the DCF, then the work continues on
the findings.

Starting fact (2026-11-19, S&P 500, 438 tickers with a value and a price): the median
of the automatic model value (P50) divided by the price is 0.48. 17% of the tickers
have a P50 above the price. 10% have a P50 of zero or less.

Rule of this ticket: the model must not be fitted to the market price. A correction
needs a reason that does not use the price: an error of logic, a count of one thing two
times, a rule that the point-in-time test of revenue and cash flow shows as wrong, or a
convention that a source supports. The price is used to measure and to rank, not to
set an input.

Steps:

1. Split the difference into parts: discount rate, growth and its fade, the horizon
   and the terminal value, margins, stock compensation, the net-debt bridge, the share
   count, and the companies that the template detection lets through by error.
2. Find defects in the code and in the rules. Each defect gets a test.
3. Measure the model against the later stock returns with the 10 years of prices.
4. Make the corrections that have a reason. Show the point-in-time bands before and
   after each one.
