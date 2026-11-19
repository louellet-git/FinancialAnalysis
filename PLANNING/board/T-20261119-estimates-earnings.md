---
id: T-20261119-estimates-earnings
title: Get analyst estimates and earnings dates in Rust
area: engine
status: review
owner: louellet-git
branch: claude/estimates-earnings
created: 2026-11-19
acceptance: One command gets the consensus estimates, the price targets, the earnings surprises and the earnings dates of a ticker into the cache with no key, and the xlsx file and the report use them.
---
Owner request, 2026-11-19: find a way to get the estimates and the earnings dates in
Rust (Python is permitted if Rust is not possible). Rust is possible.

Tests from the cloud container on 2026-11-19 with the client name
`FinancialAnalysis fa (personal research)`: the Nasdaq site API answers with no key for

- the consensus EPS forecast of each quarter and year (with high, low, the number of
  estimates and the revisions): `/api/analyst/<T>/earnings-forecast`;
- the earnings surprises with the report dates: `/api/company/<T>/earnings-surprise`;
- the consensus against the reported EPS: `/api/quote/<T>/eps`;
- the price targets and the ratings, with history: `/api/analyst/<T>/targetprice`,
  `/api/analyst/<T>/ratings`;
- the change of the consensus: `/api/analyst/<T>/estimate-momentum`;
- the earnings calendar of a date for all companies: `/api/calendar/earnings?date=D`.

Not in this source: revenue estimates. The data vendor of the pages is Zacks. The API
is not a published API.

Second source for past earnings dates: the SEC filing list (form 8-K with item 2.02).

## State (2026-11-19, before the pull request)

Done: `fa estimates get|show`, `fa earnings calendar|dates`, `mapcheck fetch --submissions`, the cache of dated snapshots, the join with `fa analyze`
(sheets `Estimates` and `Estimates Fwd`). Real runs: 12 tickers, a 60-day calendar scan, the SEC check of IBM, JNJ and MSFT (no difference), a bulk run for the S&P 500 list.
Not done: revenue estimates from a free source (Alpha Vantage and Finnhub: the free plan is not clearly documented to give them; `docs/HIDDEN-ASSUMPTIONS.md` #216),
the growth start of `fa dcf auto` next to the consensus growth, and a revenue or estimate part in `fa report`. The next date of most companies is a guess (#222).
The token ledger row is for the orchestrator.
