---
id: T-20261119-keyed-data-sources
title: Add data sources with a published API that give the past on request
area: engine
status: in-progress
owner: louellet-git
branch: claude/keyed-data-sources
created: 2026-11-19
acceptance: With a free key in the environment, one command gets daily and intraday bars with their history, and one command makes the option surface of a past date from the stored history of the source; each loader has tests on fixture text from the official documentation.
---
Owner request, 2026-11-19: solid data sources for all data, that a person can sync at
any time. No process that runs all the time. The past of each data set must come from
the source on request.

Research result (official pages, read 2026-11-19; file in the session scratchpad):

- Alpaca Market Data API, free key: stock bars (daily and intraday) with history; option
  chain snapshots with greeks (the "indicative" feed); historical option data from
  February 2024. Not confirmed: index options (SPX, VIX), the depth of the free history,
  and if an account with money is necessary.
- Tiingo, free key: daily prices, more than 30 years (loader exists, not tested live).
- FRED, free key: the rate and spread series that the keyless address refuses.
- Massive (the former Polygon.io), paid: option chains for index options (SPX) with 2 to
  5 years of history, intraday bars for 5 to 10 years, index levels. Options Starter is
  $29 each month and Indices Starter is $49 each month (pricing page; check again
  before a payment). This is the source for the SPX history when the owner decides to
  pay.

The site sources with no key (Nasdaq, Yahoo, Cboe delayed quotes) stay as the sources
that need no account. They are not published APIs, and two of them blocked the cloud
container on 2026-11-19.
