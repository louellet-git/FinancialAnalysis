# Model: base rates of growth and the reality check (`fa dcf baserates`, `fa dcf attribute --reality`)

Updated: 2026-11-21. Commit: not known (branch `claude/attribution-reality`, not committed).

Backtest: none applies, because these commands measure and do not forecast. Their point-in-time check is the poison test of a later year: a window that ends after the date D must not enter. A test with fixed answers covers it (`rust/src/dcf/auto_baserate_tests.rs`, `auto_reality_tests.rs`).

## Purpose
Does an implied number make sense against what companies did? `fa dcf baserates` gives the past 10-year revenue
growth of companies, by size. `fa dcf attribute --reality` puts the implied 10-year growth of each ticker, and the
implied premium of the list, against those past numbers. Both are measurements. No input of the model reads them
(`docs/QUALITY-STANDARDS.md`, section 6). The command `attribute` is in `docs/models/implied-and-attribute.md`.

## Inputs
| Input | Source | Note |
|---|---|---|
| Revenue | the mapped variable `revenue` only (`map_inputs_pit_periods`, 40 fiscal years) | No raw tag. A missing year is not patched |
| Date | `--as-of D` (`baserates`); D2 (`attribute`) | Only facts filed on or before the date |
| List | `--tickers-file`; `--baserate-list FILE` (default: the tickers of the run before `--top`) | Today's names |
| SEC number (CIK) | the filing list of the cache | One row for each number |
| Premium history | `--premium-history FILE`, a CSV of `fa dcf premium-series` | Columns `anchor`, `s1` |
| Implied growth | the growth view of `attribute` at D2, at the model premium | One path at the means |

## Method (short)
1. Window: a base fiscal year and the year 10 years after it (`--years`). Growth = (end revenue / base revenue)^(1/10) - 1.
2. Bucket: the base-year revenue, below 1B, 1B to 10B, 10B to 50B, 50B or more (A-034).
3. Public years only, and the choice of the base year and of the last window (A-035).
4. Table: for each bucket and for all, the last window of each company and all the windows pooled. Numbers: n, median,
   75th, 90th, 95th percentile, maximum with its ticker. Also the count of companies with no base-year value.
5. `--reality`: for each ticker, the base revenue and bucket (last public year at D2), the own 5-year growth, the
   implied 10-year growth with its percentile among the last windows of its bucket, and the flag "above every case".
   A floor or ceiling of the search is marked (`<`, `>`).
6. With `--premium-history`: the list premium at D1 and at D2 against the `s1` rows before D1. The list premium is
   taken from the full list before the `--top` cut. Only an exact search result counts (A-036).
7. The path of the 8-K earnings release of the window, when the cache has it (A-037).
8. `--top N`: keep the N largest companies by market value at D1. The common change and the list premium come from
   the full list before the cut (A-036).

## Rules of the code that are being built (not in this branch yet)
- `fa dcf attribute --prior-window` (ticket item R3): with the window D1 to D2 of n days, also run D0 to D1 and D0 to
  D2. D0 is D1 less n days, moved to the last SPY trading day on or before it. The text prints the three "all" lines of
  the premium-first bridge, with the common change of each, in a block "(6) PRIOR WINDOW". No new CSV column.
  The aim: to see a late reaction (yields first, stocks later).
- `--curve auto`, the default of `attribute` (item R4): see A-027.

## Outputs
`baserates`: the text and, with `--out`, a CSV with the header `set,bucket,n,median,p75,p90,p95,max,max_ticker`
(`set`: `last`, `pooled` or `no_base_value`). `attribute --reality`: the block "(5) REALITY CHECK" and the columns
`base_revenue,base_bucket,growth5_own,growth10_implied,growth10_bound,growth10_pctile,growth10_above_all,
list_premium1,list_premium2,list_premium1_pctile,list_premium2_pctile,release_8k_path` at the END of the CSV header.
No random part. The result does not depend on `--jobs`.

## Assumptions
| Record | Name | Value | One line |
|---|---|---|---|
| A-034 | Size buckets | 1B, 10B, 50B; nominal dollars | An edge goes to the higher bucket |
| A-035 | Window | 10 years; 45 days; 548 days | Public years only; no patch |
| A-036 | Counting rules | one row for each CIK; strictly below; cut after the common change | Percentile and `--top` |
| A-037 | Release file | newest accession, newest file | A pointer, no number |
| A-016 | Share count guard | 0.5x-2x note; 0.01x-100x replace | Weight of `--top` |
| A-029 | Common change | list median | `--top` keeps the common change of the full list |

## Limits
- The list has today's names (survivorship). Companies that left it are not in the set. This moves the rates up.
- The windows of one company overlap. They are not independent. The pooled table counts each window.
- Acquired revenue is in the growth. A spin-off lowers it.
- On 2026-11-20, 103 of 500 companies of the S&P 500 list had no base-year value. The cause is a mapping gap of old
  years (ticket `T-20261121-revenue-old-tags`, Technology) and short stores. The rates are for the other 397.
- The implied growth is one path at the means, at the model premium and margin. It is not an expected value. A search
  can end at a floor or a ceiling.
- A percentile against a bucket with few cases is weak. Read the n.
- Do not read a percentile or a flag as a signal without a review (`HIDDEN-ASSUMPTIONS.md` #260).

## Code and command
| What | Path or command |
|---|---|
| Code | `rust/src/dcf/auto_baserate.rs`, `auto_reality.rs`; in `auto_attribute.rs`: `keep_top`, `release_path`, `cik_of` |
| Command | `fa dcf baserates --tickers-file FILE --as-of D [--years 10] [--out FILE]`; `fa dcf attribute --from D1 --to D2 --tickers-file FILE --reality [--top N] [--baserate-list FILE] [--premium-history FILE]` |
| Tests | `rust/src/dcf/auto_baserate_tests.rs`, `auto_reality_tests.rs` |

## Review status
Not reviewed. No skeptic, reviewer or auditor looked at the commands. First real run: S&P 100, `--top 10 --reality`
(ticket `T-20261121-dcf-engine-integration`, state note); no run of record exists.
