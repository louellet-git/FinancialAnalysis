---
id: T-20261120-sync-chains
title: Add the chain copy to fa sync as the data set chains
area: importer
status: backlog
owner: louellet-git
requested_by: louellet-git
head: technology
order: 3
branch:
created: 2026-11-20
acceptance: `fa sync --only chains` copies the SPX, SPY and VIX chains with `fa options archive` code when the last copy is older than 1 market day, and prints "not copied: the day is lost" when it runs after the close of a day that has no copy.
---
Step 4 of `T-20261120-vol-7-chain-copy` (the Head of Quant Research built steps 2 and 3: `fa options archive`).
It waits for the answer of the person to `T-20261120-ask-chain-copy-schedule`.

What the code needs (from the author of steps 2 and 3):
1. A data set `Chains` in `rust/src/sync/`: maximum age 1 market day, only for the symbols named; default SPX,
   SPY, VIX.
2. The job: `market::options_cli::fetch_alpaca(repo, sym, today, None, rate, &stamp)`, then
   `market::options_archive::store(cache, &snap)`, with `options::utc_stamp(alpaca::now_utc())` as the stamp.
3. The age: `sync/fresh.rs` (near line 182) uses `options::list_snapshots(c, key).pop()`, which gives dates. For
   `Chains`, use the last stem of `options::stems_of_date` (the file name has the time: `<date>T<hhmm>.json`).
4. A late run prints "not copied: the day is lost". This needs a market-calendar check, which does not exist yet.
5. A `TooSmall` outcome (less than 80% of the quotes of the last copy) prints its line and counts as not copied.
   An Alpaca error text goes through the scrub of `Client` (it is applied inside the client).
6. Trap: the file date is the UTC date of the run; a run after 20:00 ET gets the next date
   (`docs/traps/2026-11-20-option-file-names-and-the-last-file.md`).
