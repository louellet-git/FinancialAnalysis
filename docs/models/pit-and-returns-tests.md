# Model: point-in-time tests of the automatic model (`fa dcf pit`, `fa dcf returns`)

Updated: 2026-11-21. Commit: 90ed737 (base of the branch). One document for two tests of measurement.

Backtest: these two commands are the point-in-time tests of the automatic model. Their own check is the truncation and poison test (skill `point-in-time-test`).

## Purpose
`fa dcf pit` asks: do the P10 to P90 bands of the model for revenue and free cash flow hold the real values of
T+1 to T+3? `fa dcf returns` asks: does the P50 / price rank the later stock returns? They are measurements of the
automatic model (`docs/models/auto-path-model.md`). They change no input of it.

## Inputs
| Input | Source | Note |
|---|---|---|
| Company facts | The cache `data/raw` | The facts of the fiscal years up to T only |
| Parameter table | `analysis/priors/process_priors_2026-11-19.toml`, or `--cut YEAR` (estimated again) | In sample unless `--cut` |
| Prices, curve, beta | Price files, GSW, SPY (`returns` only) | Price of D = end of T + lag |
| Split table | `<cache>/_corporate/splits.csv` | `returns` needs it; `pit` has no factor |

## Method (short)
1. `pit`: for each company (standard template) and each base year T with 5 fiscal years up to T and 3 after,
   build the model from the data up to T and place the real revenue and free cash flow against the P10 to P90 band.
2. Output of `pit`: coverage (target 80%) and median error for T+1 to T+3, by class (growth of the base year,
   acquisition spend, margin range). `--half H` splits the companies by the hash of the ticker. `--diag` adds
   diagnoses (`decline`, `nu`, `acq`). The free cash flow is as the model defines it (`fcff_of_rules`).
3. `returns`: the model at D = end of T + 100 days, with the curve, the beta and the price of D. The price
   return from D to D + 1 year and D + 3 years.
4. Output of `returns`: the rank correlation of P50 / price with the return (each year, pooled, mean over the
   years with its standard error; years with 20 observations or more), the mean return by quintile, by size.
5. `fa dcf assumptions --effect K=V` runs the screen and the `pit` with the default rules and with the rules
   of a candidate, and prints one block that a record can quote.

## Outputs
Text on the screen (and `--out`). Files in `analysis/priors/`: `auto_pit_*.txt`, `auto_returns_*.txt`.
Result of record for the test of 2026-11-20 (6,030 company-years): revenue coverage P10 to P90 86.1 / 83.8 / 86.9%
at T+1 / T+2 / T+3 (`auto_pit_new_2026-11-20.txt`). Returns, 1 year: mean rank correlation +0.058, standard
error 0.037 (`auto_returns_new_2026-11-20.txt`, a run before the share-basis rule).

## Assumptions
| Record | Name | Value | One line |
|---|---|---|---|
| A-014 | Test design | 5 years before, 3 after, lag 100 days, 20 per year | The 100-day lag has a stated reason; others: no source |
| A-013 | Design of the process table | see record | The table that the test uses |
| A-003 | Floor on the base-year growth | 0% | Found with the test |
| A-007 | Beta | 2/3 raw + 1/3 | Same rule in `returns` |
| A-017 | Share basis | filing date rule | `returns` needs the split table |

## Limits
- The table is in sample unless the result says OUT OF SAMPLE (`--cut`). Restated years enter the data of T.
- Survivorship: the companies exist now. The model of the test has no price, no failure and a fixed rate for `pit`.
- `pit` does not test the value (the P50). `returns` returns are price returns, with overlapping 3-year windows.
- `HIDDEN-ASSUMPTIONS.md` #199 and #286 list the known errors (a year of decline, bands for a stable margin).
- The return result of 2026-11-20 is not valid after the share-basis rule (A-017). Run it again.

## Code and command
| What | Path or command |
|---|---|
| Code | `rust/src/dcf/auto_pit.rs`, `auto_returns.rs`, `assumptions.rs` |
| Model or parameter file | `analysis/priors/process_priors_2026-11-19.toml` |
| Command | `fa dcf pit --cache data/raw`; `fa dcf returns --cache data/raw`; `fa dcf assumptions --effect K=V --tickers-file FILE` |
| Tests | `rust/src/dcf/auto_tests.rs` |

## Review status
Not reviewed. Run on 2026-11-19 and 2026-11-20 (`analysis/priors/auto_pit_*.txt`, `auto_returns_*.txt`).
