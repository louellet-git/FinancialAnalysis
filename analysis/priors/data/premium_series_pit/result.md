# Point-in-time test of `fa dcf premium-series` (2026-11-21)

## Result

All six tests PASS. Each test CSV file is byte-identical to its baseline. This applies to the
two rows (the Friday before D and D) and to all 18 columns.

| D | Truncation | Poison |
|---|---|---|
| 2020-03-20 | PASS | PASS |
| 2022-06-17 | PASS | PASS |
| 2025-04-04 | PASS | PASS |

The S1 values of the D rows are 0.06547 (2020), 0.04947 (2022) and 0.03869 (2025). All six rows
use the GSW curve. No run has a switch week.

## Set-up

- Binary: `/home/user/wt-premium/rust/target/release/fa` (branch `claude/premium-series`, a993e68). We did not rebuild it.
- Store: `/home/user/fa-data-split` (0340bc1), read only. List: `sp100.txt` (101 tickers).
- Command: `fa dcf premium-series --tickers-file sp100.txt --start <D-7> --end D --cache <cache> --bridge --out <file>`.
- We set the dates before the first run. We ran the command from `/home/user/wt-premium`.
- We used `strace` on the three baseline runs to find the files that the command reads. They are:
  `prices.json`, `prices.meta.json`, `company_facts.json.gz`, `sec_8k_earnings.json` and `filings/` of each
  ticker; SPY prices; `_corporate/splits.csv`; `rates/gsw_feds200628.csv`; the parameter table in the repo.
- The copies are made by `make_cache.sh` (shell, jq, awk, gzip) in this folder.

## Truncation copy (all data after D removed)

- Prices of the 101 tickers and SPY: rows dated after D removed.
- Company facts: each fact with `filed` after D removed. The JSON structure stays.
- 8-K list: filings filed after D removed; `latest_periodic` removed when it is after D.
- GSW rows and Treasury par, real and bill rows after D removed. FedInvest files after D removed.
- 10-K filing folders removed. All of them are filed after the three dates (the latest is 2025-12-18).
- `prices.meta.json` and `_corporate/splits.csv` are copied as they are (recorded trap: the split basis reads the original last date).

## Poison copy (data after D made false)

- Closes after D times 10. Facts filed after D: values times 1000.
- GSW rows after D: BETA0 and each yield and rate column +5 points. Treasury rows after D: +5 points.
- 8-K filings after D: `filed` and `report_date` set to the day after D (the boundary). `latest_periodic` the same.
- 10-K filing folders are not changed (their dates block them).

## Control

We put the poison one day before D (2025-04-03) and ran again. The D row changed: S1 became empty,
`ret_w` 8.05 and `zero10` 0.0904. Thus the copies reach the engine, and the tests are not empty.
The diff is in `out/control_diff.txt`.

## Limits

- The list has today's names (survivorship).
- `filed` can come from a later filing (trap #4). The filter is conservative but not exact.
- The par-fit path is not tested: GSW has a row for each anchor.
- `splits.csv` and `prices.meta.json` hold dates after D. The tests do not cut them, by design.

## Files

The cache copies are deleted after the runs (`make_cache.sh` makes them again). `out/base_*.csv`, `out/trunc_*.csv`, `out/poison_*.csv`, `out/diff.txt`, `out/*.log`, `strace_*.txt`, `make_cache.sh`.
