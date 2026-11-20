---
id: T-20261119-filing-level-xbrl
title: Read the calculation tree of a filing (first part of filing-level XBRL)
area: importer
status: done
owner: louellet-git
branch:
created: 2026-11-19
acceptance: For a ticker, one command prints the lines of the three statements as the filer structures them (parents, parts, company-specific tags, values) from the latest 10-K, and shows the capex, depreciation and debt lines.
---
Owner approval, 2026-11-19 (item 4 of the mapping assessment). This is the first part of Ticket 7 in the backlog.

Why: the companyfacts source does not say which lines are parts of which line, and it has no company-specific tags. The results are: 15 of 312 companies have no capex value, depreciation and amortization agrees with an independent source for only 50 of 67 companies, total debt for 55 of 69, and for 166 of 312 companies a variable is a detail inside a different line.
