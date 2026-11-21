---
id: T-20261121-cache-only-switch
title: Give fa a cache-only switch, and warn when a date is not in the data
area: importer
status: backlog
owner: louellet-git
requested_by: louellet-git
head: technology
order: 2
branch:
created: 2026-11-21
acceptance: With `FA_CACHE_ONLY=1` (or `--cache-only`), `fa rates inflation --date 2025-11-20`, `fa futures positions ES --weeks 6` and `fa rates nyfed --date <old date>` send no request and write no file; a missing file is an error that names the file and the store; `fa rates nyfed --date D` warns when the value is not of date D; a test for each.
---
Found by the Head of Economics (2026-11-21). Some commands download a missing file when the date is not in
the cache: `fa rates inflation --date 2025-11-20` wrote `rates/treasury_nominal_2025.csv` and
`treasury_real_2025.csv`; `fa futures positions ES --weeks 6` wrote `futures/cftc_13874A_6.json`. Thus an
agent of a head can start a download without a request to the Head of Technology (`CLAUDE.md`, "Data").
`fa rates nyfed --date <old date>` returns the latest value with no warning.

Find each fetch-on-miss path in `rust/src/` (rates, bonds, futures, recon, options) and put it behind one
switch. Agents of the heads run with the switch on (a line in each agent file that runs these commands).
