---
id: T-20261125-edinet-reader
title: Read one Japanese holding of URA (ITOCHU) from EDINET into the standard variables
area: importer
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: technology
story: E3.S5
order: 1
branch: claude/edinet-reader
created: 2026-11-25
acceptance: `mapcheck` reads the EDINET CSV form (API v2, type=5) of one ITOCHU annual report into the standard variables in JPY, with fixture tests (no network); the fetch reads `EDINET_API_KEY` from the environment and never prints it; the row in docs/DATA-SOURCES.md names the source (PDL 1.0); the first real read is one command that Luc runs on the laptop.
---
Owner word (2026-11-25, through the Head of Staff): "Yes, build it" (a reader for Japan), then "Please proceed
w work". Size given: about 1.5% of the week. The key is a query parameter, so the cloud proxy cannot add it:
tests use a fixture made from the official documentation; the first real read is on the laptop. The values
stay in JPY (fa refuses a non-USD filer); the JPY/USD rate (Fed H.10) is a later part, after the first real read.
