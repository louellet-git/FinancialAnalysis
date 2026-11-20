---
id: T-20261120-vol-7-chain-copy
title: Copy the option chains each day to keep bid and ask
area: engine
status: in-progress
owner: louellet-git
head: quant
order: 3
branch: claude/upbeat-cori-a4ykf5
created: 2026-11-20
acceptance: One command `fa options archive` copies the SPX, SPXW, SPY, VIX and VIXW chains with the snapshot time and the root of each quote, a day that was copied can be read back by `fa vol`, and the roots SPX and SPXW of one date have separate forwards (with a test).
---
Support ticket for steps 1 to 6 of `analysis/priors/volatility_study_2026-11-20.md`. Depends on: nothing. The owner put the continuous collection on hold (decision of 2026-11-19: nothing runs between two commands, #245). This ticket states the cost of the hold and gives the cheapest way to pay it.

## Why the copy is needed (the cost of not doing it)

- Alpaca gives the history of CLOSES only: there is no page for past bid and ask (`docs/DATA-SOURCES.md`). A day that nobody copied has no bid and no ask for ever.
- The study measured what closes cost: on the two active days 24% to 46% of the wide-strike monotony and butterfly tests fail beyond one tick (live: 0.02% and 4.7%), 25% to 34% of the calendar tests fail (live: 0%), the parity forward is wrong by 20 to 130 points, the history gives a VIX that is 1.3 to 14 points too low, and the ATM skew has an error of 0.05 to 0.10 in its exponent (study A.3, A.4, A.5).
- The steps of the sequence that need many days of bid and ask are the stability of the parameters over the days and the test "fit day D, price day D + 1" (the validation of tickets 1 to 6). With closes they are not valid. Without the copy these tests can be done only on the days when a person runs the live command by hand.
- The history of closes (since February 2024) does not need a copy. It is the wrong data for these tests, not a missing data set.

## What to copy and the cost

| Chain | Command today | Requests | Size of the file |
|---|---|---|---|
| SPX and SPXW | `fa options snapshot SPX --source alpaca` | 33 | 10.4 MB (32,046 quotes) |
| SPY | `fa options snapshot SPY --source alpaca` | 14 | 4.0 MB |
| VIX and VIXW | `fa options snapshot VIX --source alpaca` | 3 | 0.5 MB |
| VX settlement | `fa futures curve VX` (Cboe file; the weekly lines carry the monthly price) | 1 (Cboe) | 5 KB |
| VIX close | the Cboe VIX history file (`fa sync --only vix`) of the next day | 1 (Cboe) | 0.5 MB |

About 50 Alpaca requests a day (limit 200 each minute, the program waits 0.4 second between requests). 15 MB a day in JSON, 1.5 to 2 MB with gzip (estimate: JSON of this form compresses by 8 to 10): 250 market days are about 0.5 GB.
`data/` is not committed (the repo is public and the terms of the data are not confirmed, #243).

## How to run it (the owner decides)

The sync does not run by itself. Options, from the cheapest:
1. The person runs `fa options archive` once on each market day between 15:30 and 15:55 ET (the last quotes before the close) and once after 16:15 ET (VX settlement). The lost days are lost.
2. A scheduled task of the Claude Code service that runs the command each market day and saves the files in the cloud session (the files must then be moved to the owner's machine; not tested).
3. A cron job on a machine of the owner (not in the cloud container: the Cboe host refused the container in the past). No such machine is known in the repo.
This ticket builds the command and the rules. The owner decides the way to run it.

## Method (small steps; one part at a time)

1. Correct the defect found in the study: `fill_from_mids` (`rust/src/market/option_history.rs`) groups the quotes by the expiry date only; the 2026 chains list the roots SPX (AM) and SPXW (PM) on the same date for 6 dates, and the second root replaces the first. Group by (root, expiry); keep the root in the quote (the file has it only in the contract symbol). A test with two roots on one date and different prices checks that the forwards are different.
2. Add the snapshot time (UTC, to the minute) to the file and a new file name `<date>T<hhmm>.json` (the old `<date>.json` name stays readable). The rule of #240 (a live snapshot is never replaced by closes) stays.
3. `fa options archive`: for the list of symbols (default SPX, SPY, VIX): snapshot with the time, then a gzip of the file; a line for each symbol with the quotes, the requests and the time; the Cboe VX file; a check that the file has more than 80% of the quotes of the last file (a half chain is not saved).
4. `fa sync`: a data set `chains` with a maximum age of 1 market day, only for the symbols named; the sync prints "not copied: the day is lost" when it is late for the close.

## Tests with fixed answers

1. Two roots on one date with different prices give two series with different forwards (fixture of 6 quotes).
2. The file name and the time round-trip; an old file without the time is still read.
3. A snapshot with less than 80% of the last quotes is not written (fixture).
4. The gzip file has the same quotes as the JSON file.
5. No key is in any file or line of output (the tests of #230 apply).

## Validation on real data

Run the command on 5 market days in a row (one person, by hand). Report: quotes per symbol and day, the time of the run, the requests, the size, and the check of the snapshot time against the file time. Then the first `fa vol` use: the fit error of ticket 1 on day D and D + 1.

## Commands

`fa options archive [SYMBOLS] [--keep-days N]`, `fa sync --only chains`.
