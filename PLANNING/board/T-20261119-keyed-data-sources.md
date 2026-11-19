---
id: T-20261119-keyed-data-sources
title: Add data sources with a published API that give the past on request
area: engine
status: review
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

State 2026-11-19 (code written, no loader tested against the real server: no key in the container):

- Built: Alpaca daily prices (`fa prices get --source alpaca`, many tickers in one request), Alpaca intraday bars
  (`fa prices intraday`), Alpaca option chain snapshot (`fa options snapshot --source alpaca`), the option surface of past
  dates from daily closes (`fa options history`), the FRED API with a key (`FRED_API_KEY`, first source of the FRED loaders),
  and `docs/DATA-SOURCES.md` (the table that `fa sync` will read).
- Facts that the documentation confirms and does not confirm: `docs/ARCHITECTURE.md`, section "Keyed data sources"; traps #230 to #244.
- The owner must create: an Alpaca paper account (free) and its two keys `APCA_API_KEY_ID` and `APCA_API_SECRET_KEY`; a FRED key (`FRED_API_KEY`);
  optionally a Tiingo key (`TIINGO_API_KEY`). Put them in the environment or in `.env` (names in `.env.example`; `.env` is not in git).
- First run with keys (acceptance test of this ticket): `fa prices get IBM --source alpaca`; `fa prices intraday SPY --interval 5m --from 2024-06-03 --to 2024-06-07`;
  `fa options history SPY --date 2024-06-07 --plan`, then without `--plan`; `fa rates fred --series DGS10`.

Massive (paid), for the decision of the owner. Nothing is built. Documentation read on 2026-11-19 (URLs in `docs/ARCHITECTURE.md`):

- Confirmed: option aggregates `GET /v2/aggs/ticker/{optionsTicker}/range/1/day/{from}/{to}` (one contract in each request; depth Basic and Starter 2 years, Developer 4 years,
  Advanced all; records from 2014-06-02); contract list `GET /v3/reference/options/contracts` with `as_of` and `expired=true` (limit 1,000); index aggregates `GET /v2/aggs/ticker/I:SPX/range/...`;
  flat files (S3, `https://files.massive.com`, bucket `flatfiles`, prefix `us_options_opra`, one gzip CSV for each day; Starter and higher); no request limit on a subscribed asset class.
- Not confirmed: SPX and VIX options in any of these endpoints (the pages show equity examples only); VIX options by name; the exact key of the day file; the terms for stored data.
- Cheap test before a payment: with the free Basic key, ask the contract list for `underlying_ticker=SPX`, `SPXW`, `VIX`, `VIXW` and ask the aggregates of one `O:SPXW...` contract.
- Plan for one year of SPX surfaces: flat files, 252 day files and 1 request for `I:SPX`: 253 requests. The REST way needs about 1,000 list requests and at least 100,000 aggregate requests (an estimate).
  The builder `option_history::build_range` takes a source through the trait `OptionHistorySource`; the Massive source is one `impl` (about 150 lines) after the decision.
