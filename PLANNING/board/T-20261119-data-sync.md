---
id: T-20261119-data-sync
title: Make one command that updates all data sets, and only the old ones
area: engine
status: review
owner: louellet-git
branch: claude/data-sync
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

State, 2026-11-20 (branch `claude/data-sync`, not merged): the command is built and tested with fakes (552 Rust tests, 91 in `mapcheck`). The documents are written: `docs/ARCHITECTURE.md`,
`docs/DATA-SOURCES.md`, `docs/HIDDEN-ASSUMPTIONS.md` (#245 to #259), `README.md` ("First run on your computer"), `CLAUDE.md`.

First real run (4 tickers, `SEC_USER_AGENT` not set, so the SEC data sets were skipped and not run for real): 14 min 25 s. Cause: 26 FRED series failed one after the other, each after about 32 seconds
(closed connection of the container), 13 min 50 s in one host queue. Correction: two connection failures in a row stop a host for the run; the time limit of a request is 30 seconds (60 for large files, 90 for
the Atlanta Fed). Same run after the correction, with `--force`: 4 min 46 s (the longest queue is Nasdaq: 57 requests in 4 min 16 s). Same command again with no `--force`: 1 min 07 s, no request for a current data set
(the requests went to the hosts of the items that stayed old: Cboe 403, FRED unreachable, Yahoo futures).

Not done: a real run of `sec`, `filings` and the SEC part of `earnings` (these need `SEC_USER_AGENT`; owner rule for this session: SEC requests only through the Python bridge); a run for 500 tickers;
a test on Windows. The ticket stays `in-progress` until the owner has run `fa sync` with `SEC_USER_AGENT` on a computer with a normal network.
