---
id: T-20261119-price-history
title: Get the daily price history of each ticker from more than one source, with a check
area: engine
status: in-progress
owner: louellet-git
branch: claude/price-history
created: 2026-11-19
acceptance: One command gets and updates the daily price history of a list of tickers into the cache from the available sources, a second command compares two sources and reports each difference, and the DCF commands use the last close when the person gives no price.
---
Owner request, 2026-11-19: the price must not come from the person. The repo needs a
solid source of past stock prices.

Tests from the cloud container on 2026-11-19, with the client name
`FinancialAnalysis fa (personal research)`:

- Yahoo chart endpoint: answers (it gave HTTP 429 earlier on the same day). No key.
  History from 1986 for MSFT, with splits and dividends. Not a published API.
- Nasdaq site API (`api.nasdaq.com/api/quote/<T>/historical`): answers. No key. 10 years
  of daily prices, not adjusted for dividends. Not a published API.
- Tiingo: a published API with a free key (more than 30 years, adjusted prices). The
  owner must register for the key.
- Stooq: no answer. Alpha Vantage and EODHD: the free limits are too small.

Design: one interface for a price source; Tiingo first when its key is in the
environment, then Yahoo, then Nasdaq; a cache that gets only the new days; a
cross-check between two sources.
