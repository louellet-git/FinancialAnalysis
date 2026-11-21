---
id: T-20261121-pit-supplement-fill
title: Use the 10-K supplement in a point-in-time mapping check when its filing was public on the date
area: importer
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: technology
order: 1
branch: claude/pit-supplement-fill
created: 2026-11-21
acceptance: With a date cut, `check_ticker_filed` fills from the 10-K supplement only when that filing was public on or before the cut; V, BRK-B, COP and NEE pass `fa dcf implied --as-of 2026-11-13`; a fixed-answer test shows a supplement filed after the cut gives no fill and one filed before fills.
---
Found by the Head of Fundamental Research (2026-11-21; word of the person: "If there's a problem with the data, that
needs to be fixed"). `rust/mapcheck/src/run.rs`, `check_ticker_filed`: with `filed_cut = Some(date)` it calls
`check_facts` with no fill; with no cut, `check_cached` uses `Supplement::load` (the 10-K XBRL in
`<T>/filings/<accession>`). For V (shares, sum of the class members), BRK-B (class B equivalent shares), NEE (capex,
company tags) and COP, the latest values come only from the supplement, so the vintage fails `core_missing_critical`
and the four drop out of the week attribution of the S&P 100.
