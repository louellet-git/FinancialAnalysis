# Point-in-time tests of `fa dcf premium-series` on the code of main (2026-11-21)

This file replaces `../premium_series_pit/result.md` for the series run of record. It answers the review of the Head of
Quality and Risk of that file (verdict REVISE, points P1 to P4).

## Result

| Test | Dates | Result |
|---|---|---|
| Truncation (all data after D removed) | 2020-03-20, 2022-06-17, 2025-04-04, 2020-04-10, 2022-07-29 | 5 of 5 byte-identical to the base |
| Poison (all data after D made false) | the same 5 dates | 5 of 5 byte-identical to the base |
| Par path: truncation and poison | 2025-04-04, with the switch week 2025-03-28 | 2 of 2 byte-identical to the base |
| Share basis of D | 2020-03-20 (18 tickers of the list have a split after it) | equal in each cell; largest difference 1.1e-16 |
| Controls (a false value on or before D) | 2025-04-04, six data paths | 6 of 6 change the output |

`cmp.txt` has the line of each comparison (`cmp` on the CSV files of `out/`). A run that failed counts as a failed test.

## Set-up

- Code: main 99c5af6. `rust/` is identical to e2557b6 (after pull requests 190, 199 and 200). Binary: the release build.
- Data store: fa-data 0340bc1, read only. List: `rust/mapcheck/universe/sp100.txt` (101 tickers).
- Command: `fa dcf premium-series --tickers-file sp100.txt --start S --end D --cache <cache> --bridge --per-ticker F --out F`.
  S is 7 days before D (14 days for the par path). Each CSV has all 18 columns; `cmp` compares every byte.
- The dates were set before the first run. The copies are made by `make_cache2.sh`; the driver is `run_all.sh`
  (one-time scripts: shell, jq, awk; they have the paths of the test machine).
- The first run of the driver failed for a path error of the driver (the runs did not find the copies). The
  comparisons of that run are not used. The driver was corrected, and a missing output now fails a test.

## The copies

- Truncation: prices after D removed; each company fact with `filed` after D removed (an amended filing after D is
  thus removed too); 8-K filings filed after D removed; GSW, Treasury and FedInvest rows after D removed; the 10-K
  filing folders removed.
- Poison: closes after D times 10; facts filed after D times 1000; GSW rows after D +5 points on each yield;
  Treasury rows after D +5 points; 8-K filings after D moved to the day after D with item 2.02.
- Par path (point P2): the GSW file ends on 2025-03-21 in the base and in both copies. The row of 2025-03-21 uses
  GSW; the rows of 2025-03-28 (the switch week) and 2025-04-04 use the par fit (`curve_source` = `par`). The
  Treasury rows after D are removed (truncation) or +5 points (poison).
- Share basis of D (point P1): a cache in the units of D. The splits with an ex-date after D are removed from
  `_corporate/splits.csv`; `last_date` of each `prices.meta.json` is the last row on or before D; each price on or
  before D is multiplied by the product of the factors of the removed splits of the ticker (AAPL 4, TSLA 15). The
  engine then reads a share basis of D. The series CSV and the per-ticker CSV are equal to the base in each cell: the
  largest absolute difference is 8.0e-17 (series) and 1.1e-16 (per ticker), which is the rounding of the price
  multiplication; no text cell differs (`share_basis_difference.txt`, `tol.awk`). So the result at D does not depend
  on a split after D, and the base run does not mix two share bases.

## Dates and what each one adds

- 2020-03-20, 2022-06-17, 2025-04-04: the dates of the first test, now on the code of main (point P3).
- 2020-04-10: a holiday Friday. The anchor is 2020-04-09, in the base and in both copies.
- 2022-07-29: a week with many 10-Q filings. `s2_facts` is +0.0103 there, so that column can move; it is identical.

## Controls (the copy reaches the engine)

Each control changes one data path on or before D = 2025-04-04, and the output must differ from the base:
the Treasury rows of the 10 days to D (par path); the GSW rows of the 10 days to D; the facts filed in the 400 days to D
(5 tickers, times 1000); the closes of the 5 days to D (5 tickers, times 2); the SPY row of D removed (the anchor
moves); the close of D removed for 5 tickers (the stale-price rule). All six outputs differ.

## Limits (point P4 and what is not covered)

- The parameter table is `process_priors_2026-11-19.toml`. It is fitted on ALL years of the cache; it is NOT cut. The
  parameters of each test run and of each series row are thus in-sample. These tests show that the DATA after D do not
  enter a row. They do not show that the PARAMETERS are point in time. The series run of record must use a table cut
  before its first year (`--table`), or its header and its document must say "parameters in-sample". No cut table
  exists today.
- The 8-K path has no control of its own: the series CSV has no 8-K column.
- A restated value is not tested: the store keeps the facts with their filing dates, and the cut uses those dates.
- The list is a list of today's names (survivorship).
