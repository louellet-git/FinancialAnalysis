---
id: T-20261123-fund-holdings-nport
title: Get the holdings of an ETF with their date from the SEC form N-PORT
area: importer
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: technology
order: 1
story: E3.S5
branch: claude/fund-holdings-nport
created: 2026-11-23
acceptance: `fa fund holdings URA` prints the holdings and weights of URA from its latest public N-PORT filing on EDGAR, with the report date and the accession; a fixed-answer test on a small fixture; the real command runs.
---
Owner word (2026-11-23, through the Head of Staff), on the URA one-pager: "On the limitations section can we
fill that out/begin work on removing limitations." The page used a secondary site with no date, because the
fund page refused the read. The SEC form N-PORT is public, dated, and lists the holdings of each registered
fund (public for the third month of each quarter, about 60 days later). Public domain.
