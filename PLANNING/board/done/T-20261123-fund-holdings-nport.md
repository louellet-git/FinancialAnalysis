---
id: T-20261123-fund-holdings-nport
title: Get the holdings of an ETF with their date from the SEC form N-PORT
area: importer
status: done
owner: louellet-git
requested_by: louellet-git
head: technology
order: 1
story: E3.S5
branch: claude/fund-holdings-nport
created: 2026-11-23
acceptance: `fa fund holdings URA` prints the holdings and weights of URA from its latest public N-PORT filing on EDGAR, with the report date and the accession; a fixed-answer test on a small fixture; the real command runs.
merged: pull request 226 (branch claude/fund-holdings-nport)
---
Owner word (2026-11-23, through the Head of Staff), on the URA one-pager: "On the limitations section can we
fill that out/begin work on removing limitations." The page used a secondary site with no date, because the
fund page refused the read. The SEC form N-PORT is public, dated, and lists the holdings of each registered
fund (public for the third month of each quarter, about 60 days later). Public domain.

Built (2026-11-23): `fa fund holdings TICKER [--cache DIR] [--top N]` (rust/src/market/fund_holdings.rs).
The filing list comes from the EDGAR Atom list of the series id (one request), and each filing's
seriesId is checked. URA: Global X Funds (CIK 1432353), series S000029442, report date 2026-07-31,
accession 0002048251-26-008623, filed 2026-09-29; 87 holdings; Cameco 22.18%, Sprott Physical Uranium
Trust 6.74%, Oklo 6.09%, NexGen 6.07%, Uranium Energy 4.81%. Limits: only the third month of each quarter
is public, so the holdings are 2 to 5 months old; the report date is not the filing date.

QA gate: PASS-WITH-NOTES (cycle 2). Note for later: move the form filter and the choice of the latest report into one function with a fixed-answer test (two NPORT-P filings in the wrong order).
