# The DCF engine on the backtester: first stock-level result (2018 to 2026)

Ticket `T-20261121-dcf-engine-backtest`. Interim file: part 1 of the plan (stock level, the store window). The
market-level part (with the Head of Economics) and the longer stock window (older prices) are not done yet.
Rule: the price measures and ranks. No input of the model is set from the price, and no rule is chosen on the
years that score it (`docs/QUALITY-STANDARDS.md`, section 6). No review yet: do not use for a trade.

## Question

Does the automatic DCF model, built point in time, pick stocks that do better than the equal weight of the same
companies? Measure: the annual return of the top fifth by P50 / price, less the equal weight of the scored list,
each calendar year.

## Data and commands

Data store `1804f48` (SEC facts of 2026-11-20, prices to 2026-11-19). Most price files start on 2016-11-18, so
the first rebalance is 2018-01-02 (the vintages and a year of prices exist). Seed 7, 300 paths for each model,
quarterly rebalance, 10 bp on each traded value, price returns (no dividends). Branch
`claude/dcf-engine-backtest`.

```bash
C=data/raw; F=rust/target/release/fa; S=/tmp/bt; mkdir -p $S
# the scored list: the tickers with the standard template (432 of 503 in sp500.txt; 431 run, because one has no SEC facts)
$F dcf screen --tickers-file rust/mapcheck/universe/sp500.txt --cache $C --as-of 2026-11-19 --out $S/screen.csv
awk -F, 'NR>1 && $3=="standard"{print $1}' $S/screen.csv > $S/scored.txt
# the out-of-sample parameter table: fiscal years up to 2016 only (new option --cut)
$F dcf calibrate --cache $C --cut 2016 --no-bands --out $S/table_cut2016.toml
# the runs (A: in-sample table; B: out-of-sample table; C: B with the holdings)
$F portfolio backtest --strategy value-rank --start 2018-01-01 --rebalance quarterly --tickers-file $S/scored.txt --cache $C --out $S/a.csv
$F portfolio backtest --strategy value-rank --start 2018-01-01 --rebalance quarterly --tickers-file $S/scored.txt --cache $C --table $S/table_cut2016.toml --out $S/b.csv --holdings $S/b_h.csv
# D: the same as B with --neutral sector
$F portfolio backtest --strategy momentum   --start 2018-01-01 --rebalance quarterly --tickers-file $S/scored.txt --cache $C --out $S/m.csv
# cost: the same as B with --cost-bp 0 and --cost-bp 20; the second list: the same as B with rust/mapcheck/universe/next500.txt
```
The yearly rows come from the `--out` CSV (columns `portfolio` and `equal_weight`, value at the last close of each
year). The t value is the mean of the 9 yearly differences over its standard error (2026 is to 2026-11-19).

## Evidence

| Run | Annual return | Equal weight of the list | Difference a year | Years above | t |
|---|---|---|---|---|---|
| A. value-rank, in-sample table | +19.09% | +15.28% | +3.80 (mean of years +3.79) | 7 of 9 | 2.12 |
| **B. value-rank, table up to 2016** | **+18.20%** | +15.28% | **+2.91** (mean of years +2.85) | 7 of 9 | **1.62** |
| B at 0 bp / 20 bp | | | +3.03 / +2.80 | | |
| D. B, sector-neutral (`--neutral sector`) | +18.73% | +15.28% | +3.45 (mean of years +3.42) | 6 of 9 | 2.06 |
| Momentum (12-1), same list | +21.89% | +15.28% | +6.60 (mean of years +6.66) | 6 of 9 | 2.25 |
| B on `next500.txt` (406 scored) | +30.14% | +26.92% | +3.22 (mean of years +2.11) | 6 of 9 | 0.25 |
| SPY | +11.67% | | | | |

Run B by year (portfolio / equal weight / difference, %): 2018 -0.6 / -2.7 / +2.1; 2019 +34.1 / +32.3 / +1.9;
2020 +22.4 / +25.2 / -2.8; 2021 +38.8 / +27.9 / +10.9; 2022 -5.3 / -12.0 / +6.7; 2023 +26.3 / +23.3 / +3.0;
2024 +22.2 / +19.1 / +3.1; 2025 +9.4 / +16.0 / -6.6; 2026 +21.8 / +14.4 / +7.4.

Run B: 281 to 361 tickers scored at each rebalance (median 342), about 67 held, 210 different tickers in 9 years,
turnover 0.77 a year one way.

Sectors of run B (mean weight of the year, Fama-French 12 from today's SIC code; `fa dcf implied --out`). The
largest tilt is to Shops: 15% to 22% of the portfolio against 9% of the list. Energy is 0% to 10.5% (list 3.5%).
In 2021 and 2022, the two best years, energy is 4.9% and 1.9%, so these years are not an energy bet. Utilities fall
from 6.6% to 0%, and Money rises from 8.8% to 13.3%.

Run D keeps the sector weights of the scored list and takes the top fifth inside each Fama-French 12 sector
(`--neutral sector`; a sector with `n` scored tickers holds its top `max(1, n / 5)`). By year (difference, points):
2018 -0.4; 2019 +3.5; 2020 -1.5; 2021 +7.8; 2022 +8.4; 2023 +8.1; 2024 +4.9; 2025 -5.8; 2026 +5.7. The split
into the sector part and the stock part was planned before the run (ticket, plan item 1). It is a measurement and
does not choose the strategy.

## Judgment

- The point-in-time model value adds about 3 points a year over the equal weight of the same S&P 500 companies,
  with an out-of-sample parameter table. This is not significant at the 5% level (t 1.62 over 9 years). The sign
  is the same in 7 of 9 years. Price momentum on the same list did better (+6.6, t 2.25).
- The gain is the choice of stocks inside a sector, not the sector tilt: with the sector weights of the list
  (run D) the difference is +3.45 points a year (t 2.06), not less than run B. 2025 is a loss in both runs.
- The in-sample table adds about 0.9 points a year (A against B): a part of the earlier result was look-ahead in
  the table. From now on, a backtest of the model uses a table cut before its first year.
- The second list (`next500.txt`) cannot judge the model now. Its equal weight rose 108% in 2020: today's list of
  mid-size companies keeps the ones that grew. The yearly difference has a standard deviation of 25 points.
- The cost is not the limit: 20 bp a trade takes 0.2 points a year.

## Limits

- Survivorship: today's lists. A value strategy buys cheap stocks, and a cheap stock that failed or left the index
  is not in the list. So the bias can favor this strategy more than the equal weight. The size is not measured.
- Price returns: value stocks pay more dividends, so the price return can understate the strategy. Not measured.
- One window of 8.9 years with one regime of rates after 2017. 9 yearly points give a wide error.
- The table up to 2016 uses today's facts of those years (a later restatement stays). The vintages of the model
  itself are cut by the filing date.
- The equal-weight line and the strategy use the same scored list, the same dates and the same cost.

## Point-in-time checks

- Parameter table of runs B, D and R1: `fa dcf calibrate --cache data/raw --cut 2016 --no-bands` (fiscal years up
  to 2016 only; the first rebalance is 2018-01-02), SHA-256
  `3a74b20f452f1878d692ab92262255a1811c21f0a057657da61d03c787e3acce` (the file; it holds a `command =` line with the
  `--out` path). Without that line (`grep -v "^command = " FILE | sha256sum`):
  `41443b661454247292bfb217c65d17b214e0672b0d03d4cf2c6a136555233665`.
- Truncation test (run D, 2026-11-21): a copy of the cache with the prices of SPY and of the scored list and the
  GSW curve cut before 2022-01-31 (the facts are linked; the vintages cut them by filing date), and the run D
  command with `--end 2022-01-31`. The 1,023 holding rows of the 17 rebalances before 2022-01-31 are the same
  as in the full run (`cmp`). So the strategy reads no later close and no later curve. The poison check of the
  harness is not used: it does not catch a read of a close one day later (harness gate, cycle 1).

## Variant log (each run of the value signal, in order; outside the harness)

All on store `1804f48`, start 2018-01-01, seed 7, 300 paths. Exploratory (no committed hypothesis file). The
harness run after the harness gate counts if a number differs.
1. value-rank, quarterly, `sp500.txt` (502 tickers), in-sample table: +4.86 a year over the equal weight of the 502.
2. momentum, quarterly, `sp500.txt` (502): reference, not a variant of the value signal.
3. A: value-rank, quarterly, scored list (432 listed, 431 run), in-sample table: +3.80.
4. momentum, quarterly, scored list: reference.
5. B: value-rank, quarterly, scored list, table cut 2016: +2.91.
6. B on `next500.txt` scored list (406): +3.22 (mean of years +2.11, t 0.25).
7. B at 0 bp: +3.03. 8. B at 20 bp: +2.80.
9. B again with `--holdings` (the same values, a check).
11. D with `--end 2022-01-31` on the full cache, and on the cut cache (the truncation test; holdings only).
10. D: B with `--neutral sector`: +3.45.
Pre-registered (A-009, committed 0360941 at 2026-11-21 03:54:55 UTC, before any of its runs). Each ran once:
12. R1: `--strategy value-rank-weekly --rebalance weekly` (the A-009 rule), table cut 2016.
13. R2: `--strategy value-rank --neutral sector --pick bottom`, quarterly, table cut 2016.
14. R1 again by the head (byte-identical CSV), and R1 with the data cut at 2021-03-01 (truncation test).

## The pre-registered runs of A-009 (branch `claude/dcf-engine-backtest` c53f56d)

```bash
$F portfolio backtest --strategy value-rank-weekly --start 2018-01-01 --rebalance weekly --tickers-file $S/scored.txt --cache $C --table $S/table_cut2016.toml --out $S/R1.csv --holdings $S/R1_h.csv
$F portfolio backtest --strategy value-rank --neutral sector --pick bottom --start 2018-01-01 --rebalance quarterly --tickers-file $S/scored.txt --cache $C --table $S/table_cut2016.toml --out $S/R2.csv
```

| Run | Annual return | Equal weight | Difference a year (mean of years) | Years above | t |
|---|---|---|---|---|---|
| **R1, the A-009 rule** | +21.37% | +14.80% (weekly) | **+6.56** | 8 of 9 | **3.44** |
| R2, bottom fifth inside each sector (held long) | +11.78% | +15.28% | -3.24 | 2 of 9 | -2.71 |

R1 by year (difference, points): 2018 +1.8; 2019 +8.8; 2020 +9.3; 2021 +6.1; 2022 +11.6; 2023 +9.9; 2024 +11.1;
2025 -6.4; 2026 +6.7. First position 2018-01-05, 463 weekly rebalances, 38.3 open positions on average after week
13, turnover 1.92 a year, cost 1.07% a year (included). 9 of 450 weeks had no new entry. Of 1,362 entries, 928 are a
re-entry of a name whose 13 weeks ended that day (the rule drops first, then chooses), so a name that stays at the
top of its sector is held for more than 13 weeks. In live use a re-entry is a hold, not a trade.
R1 truncation test: the 6,108 holding rows before 2021-03-01 are byte-identical with the prices and the curve cut
at that date.

Judgment:
- R1 is the strongest result: +6.56 points a year over the equal weight of the same list, t 3.44, one run as
  pre-registered. It is larger than the quarterly top fifth (run D, +3.45). This file does not say why. Possible
  parts: the 3 names a week are the extreme ranks; the weekly trade back to 1/39 (a reversal effect, which the
  weekly equal weight also gets); the staggered entry. These are not measured, and no variant is run to find out.
- R2 meets the A-009 trigger for a short side (t -2.71 at or below -2). A short is still not used: the pilot
  limits of the Head of Quality and Risk permit long stock only until a hedging engine passes its gate, and R2
  holds the names long (no borrow cost, no short loss).
- The limits above stand: survivorship, price returns, 9 years. The weekly equal weight is also rebalanced weekly.
  A-009 item 5 data checks that the backtest cannot apply: the mapping status and the share-count guard.

## What stays open (owner: the Head of Fundamental Research)

1. Older prices (from 2009) for a window of 2010 to 2026 (request to the Head of Technology, 2026-11-21).
2. The market-level test over the longest history (with the Head of Economics).
3. Done: the sector-neutral version (run D).
4. Total return with dividends, and the delisted companies (the harness of the Head of Technology, parts 2 and 3).
5. The acquisition-charge change (`claude/dcf-acq-charge-limits`, in the gate) as a second model version, run with
   the same commands. It is a measurement and does not select the default.
