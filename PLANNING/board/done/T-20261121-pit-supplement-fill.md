---
id: T-20261121-pit-supplement-fill
title: Use the 10-K supplement in a point-in-time mapping check when its filing was public on the date
area: importer
status: done
owner: louellet-git
requested_by: louellet-git
head: technology
order: 1
branch: claude/pit-supplement-fill
created: 2026-11-21
acceptance: With a date cut, `check_ticker_filed` fills from the 10-K supplement only when that filing was public on or before the cut; V, BRK-B, COP and NEE pass `fa dcf implied --as-of 2026-11-13`; a fixed-answer test shows a supplement filed after the cut gives no fill and one filed before fills.
merged: pull request 150 (branch claude/pit-supplement-fill)
---
Found by the Head of Fundamental Research (2026-11-21; word of the person: "If there's a problem with the data, that
needs to be fixed"). `rust/mapcheck/src/run.rs`, `check_ticker_filed`: with `filed_cut = Some(date)` it calls
`check_facts` with no fill; with no cut, `check_cached` uses `Supplement::load` (the 10-K XBRL in
`<T>/filings/<accession>`). For V (shares, the largest class: class A, 1,907M), BRK-B (class B equivalent shares), NEE (capex,
company tags) and COP, the latest values come only from the supplement, so the vintage fails `core_missing_critical`
and the four drop out of the week attribution of the S&P 100.

## Real scope (measured 2026-11-21)

`fa dcf implied --tickers-file rust/mapcheck/universe/sp500.txt --as-of 2026-11-13 --cache /home/user/fa-data --out FILE`,
base binary built from `f4d74ea^` (scratch worktree, removed after), after binary from this branch:

- 23 tickers go from FAIL to a value: ABNB, APA, APO, APP, ARES, BKR, BRK-B, CI, CMS, COIN, COP, DTE, ERIE, LHX, NEE,
  PSX, STZ, TKO, TPL, URI, V, VEEV, VLO. No other refusal changes. Refused 95 -> 72; with a value 408 -> 431.
- 4 values change (`value_at_means`): IBM 171.25 -> 164.79, MA 401.39 -> 381.49, CMCSA 92.26 -> 91.95, ED 96.70 -> 99.31.
- The list premium moves, so `value_over_price_at_market_premium` changes in 422 rows (399 of the old rows and the 23 new
  ones; the QA gate counted 423).
- Cause: the Lines mode (capex, capitalized_software from the 10-K lines) now runs in the cut path too, as it already does
  with no cut. Intent: the cut path equals the no-cut path for a 10-K that is public by the cut.
- The date is parsed (YYYY-MM-DD); a filing date or a cut that is not a date gives no fill (`pit_supplement_not_used`).

## What proves the point-in-time property

The planted test and the unit tests (`supplement.rs`: cut tests; `run.rs`: the wiring test of `pit_supplement` and
`pit_fill`, which `check_ticker_filed` uses). `fa dcf pit` gives the same result only because `auto_pit.rs` does not use this
path (it reads facts with `mapper::resolve_period_with` and no supplement), so it does not cover the supplement.

## Traps found

- BRK-B: the old company_facts basic share values (2012-2014) are class A counts, about 1000 times smaller than class B
  equivalent counts. A scale trap for a long history.
- COP: capex from the 10-K lines includes "investments" (the company definition of its capex line).
