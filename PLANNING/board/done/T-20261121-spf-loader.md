---
id: T-20261121-spf-loader
title: Load the SPF 10-year forecasts for the Economics engine
area: data
status: done
owner: louellet-git
requested_by: louellet-git
head: technology
order: 2
branch: claude/spf-loader
created: 2026-11-21
acceptance: `fa spf get` downloads medianLevel.xlsx of the Philadelphia Fed SPF into the cache with its download date; `fa spf show` gives the 35 Q1 values of RGDP10, STOCK10, BOND10 and BILL10 with the release date of each survey (or the documented upper bound, marked as such); a fixture test; the file goes into the private store only.
merged: pull request 160 (branch claude/spf-loader)
---
The Head of Economics needs the long-run SPF medians for its engine. The site has no terms page
(footer: "All rights reserved"). The owner approved a private copy, with only results in the repo
(2026-11-21). The file is not a vintage file: a later download can revise old values.

QA gate: PASS-WITH-NOTES (2026-11-21). Open notes, for a later small change: `show` checks the stored
SHA-256 on read; an error when a sheet gives 0 rows; `fa spf show --bogus` names the wrong help; `get`
parses before it saves; check the jumps BOND10 2011-12 and 2019-20, BILL10 2019-20, STOCK10 2021-22
against the published tables once.
