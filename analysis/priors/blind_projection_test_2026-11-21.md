# Blind projection test of the DCF engine (T1 to T4), one run

> **T2 to T4 INVALID (2026-11-21, CRIT F1 of the strategy gate): split look-ahead in the prices.** The rank tests
> compare a past value per share with a split-adjusted price. They run again after `T-20261121-split-basis-fix`.
> T1 (`fa dcf pit`, no price) is not touched.

Ticket `T-20261121-blind-projection-test` (design d7a6ae0, revision 1 0852d36, revision 2 1204ed1; all before
this run). Data store `1804f48`, seed 7. Branch `claude/weekly-rule-backtest`. One run of each test, 2026-11-21
04:46 to 04:49 UTC. No review yet. Rule: the price measures and ranks; no input is set from the price.

## Result

**Headline: NOT ALL PASS.** T1 fails. T2, T3 and T4 pass. By the design, the weekly trades do not start on this
basis until T1 passes (or Luc decides otherwise).

| Test | Rule (written before the run) | Result | Verdict |
|---|---|---|---|
| T1 revenue, "all" row | inside P10-P90 75-85%, each tail 5-15% | T+1 5.6 / 84.8 / 9.6; T+2 5.2 / 84.2 / 10.6; T+3 **3.5 / 88.4** / 8.1 | fail at T+3 (too wide) |
| T1 free cash flow, "all" row | the same | T+1 **17.7 / 71.2** / 11.1; T+2 **16.0 / 72.7** / 11.3; T+3 15.0 / **72.8** / 12.3 | **fail** (low tail too thin) |
| T2 rank against the 13-week outcome | mean d_w > 0, one-sided p < 0.05, block 26 | +1.79% a week, p 0.0016 (block 13: 0.0008; 52: 0.0022); 13 of 13 phases above 0 | pass |
| T3 rule A, hit against the base rate | mean > 0, p < 0.05 | +3.38 points, p 0.0056 (hit rate 50.8%, n 1,323) | pass |
| T3 rule B, "priced in" tail | below stated P10 5-15%, below P5 2-8% | 13.0% (10.5 to 15.7); 7.9% (5.9 to 9.9); 1,167 entries | pass |
| T4 at any time | 7 of 9 years, both halves, 2 of 3 periods | entries 8 of 9 years, d_w 8 of 9; halves 2 of 2; periods 3 of 3 | pass |

Commands (from `claude/weekly-rule-backtest` 1204ed1):
```bash
fa portfolio calibrate --start 2018-01-01 --tickers-file <scored.txt> --cache data/raw --table <table cut 2016, SHA-256 3a74b20f...> --ranks-out R.csv --entries-out E.csv --stress-file analysis/macro/stress-windows.csv --block-weeks 26 --boot-seed 7 --reps 10000
fa dcf pit --cache data/raw --cut 2015 --out T1.txt
```

## What the numbers say

- **T1 (the projections of the fundamentals).** Out of sample (table up to 2015, base years from 2015; 7,669
  company-years), the real free cash flow falls below the model P10 in 15% to 18% of the cases, not 10%. The
  model sees too little of the downside of free cash flow. The P50 is not biased (the median error is -0.2% to
  +1.1% of revenue). At T+3 the revenue band is too wide (88.4% inside). Large companies (above 100B) are too wide
  in both. The values of the model (P10, P50, P90 of a stock) therefore claim a narrower low side than the record.
- **T2 (the signal).** The edge is in the top of the rank: decile 9 +1.76% over 13 weeks, decile 8 +0.40%;
  deciles 0 to 7 are between -0.57% and +0.01%. The bottom decile is not worse than the middle. So the signal
  picks winners; it does not pick losers.
- **T3 (the trades).** The entries beat the base rate of the same week by 3.4 points. Their absolute hit rate is
  only 50.8%: half of the trades lose against the equal weight. The 13-week P10 of an entry is -15.2% and the P5
  is -20.7%. The tail that was stated from the earlier trades held for the later trades overall (13.0% below the
  P10).
- **T4 (at any time).** 2025 is the one year with a loss (entries -0.71%, d_w -1.33%). The last period
  2024-2026 is the weakest (d_w +0.34%). The worst 13-week spread is -10.55% (2025-04-04).
- **Stress windows (descriptive).** In the recent stress windows the trades lost, and the tail was worse than
  stated: 2024-08 (entries -12.9%, 33% below the P10, 6 entries), 2025-04 (-2.5%, 28%, 18 entries), 2026-03
  (-4.1%, 20%, 15 entries). In 2020-03 and in 2022 the trades won. So the "priced in" tail holds on average,
  but not in a sharp drop of the last two years.

## What follows (owner: the Head of Fundamental Research)

1. T1 must be fixed before the weekly trades start on this basis. The defect: the low tail of the free cash flow
   is too thin. The fix is a model change with its own test, written before its run (fit on `--half 0`, score on
   `--half 1` and `--cut 2015`). The revenue band at T+3 is a second, smaller item.
2. Each weekly write-up states: the hit rate 50.8% and +3.4 points over the base rate; the 13-week P10 -15.2% and
   P5 -20.7%; the worst year (2025) and the stress windows of 2024-2026, in which the tail was larger.

## Limits

Today's lists (survivorship); price returns; today's SIC sector; 9 years. T1 is not fully blind to the head (earlier
pit outputs were seen). The stress windows have few entries each.
