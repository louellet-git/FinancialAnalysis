---
id: T-20261119-data-sync
title: Make one command that updates all data sets, and only the old ones
area: engine
status: backlog
owner: louellet-git
branch:
created: 2026-11-19
acceptance: `fa sync` updates each data set that is older than its maximum age for a list of tickers, continues when one source fails, prints one summary line for each source, and a second run on the same day makes no network request.
---
Owner request, 2026-11-19: one function that imports everything, with switches, and
efficient. Start after `T-20261119-estimates-earnings` (it changes the same code).

Design:

1. One command for all sources: SEC company data and the latest 10-K files, prices,
   estimates and earnings dates, option chains, Treasury curves, the Federal Reserve
   curve, New York Fed rates, inflation data.
2. Each data set has a maximum age (prices, calendar and rates: 1 day; estimates: 1 week;
   SEC data: until the SEC filing list shows a new filing; option chains: only on
   request). The command gets a data set again only when it is older.
3. Switches `--only` and `--skip`. With no switch, the command does all parts.
4. One queue for each host, with the request limit of that host. The hosts run at the
   same time.
5. A failure of one source does not stop the others and does not change its cache.
6. `fa sync --status` shows the age of each data set with no network request.

The value commands (`fa dcf auto`, `fa verdict`) do not start a sync. They print a
warning when their data is old.

The SEC part needs the SEC contact text (open decision in `T-20261119-python-to-rust`).

Owner decision, 2026-11-19: the sync runs on request only. No watch mode in this ticket.
