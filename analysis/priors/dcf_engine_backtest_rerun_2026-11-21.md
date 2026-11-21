# The DCF engine on the backtester, rerun on corrected prices (split basis)

Ticket `T-20261121-split-basis-fix` (rerun test: items 1 to 7 of its section "Rerun test", committed 02e0be0 to
e321c34 before any rerun). This file replaces the results of `analysis/priors/dcf_engine_backtest_2026-11-21.md`
and of T2 to T4 in `analysis/priors/blind_projection_test_2026-11-21.md`, which had the split look-ahead.
Rule: the price measures and ranks; no input is set from the price. No review yet.

## Result

**The stock-level DCF signal shows no edge on corrected prices. The decision rule fails, so no weekly proposal uses
rule A-009.** T2 to T4 of the blind projection test also fail. The tail rule ("priced in") passes.

| Run (2018-01 to 2026-11, weekly or quarterly) | Annual | Equal weight | Difference a year (mean of years) | t | Years above |
|---|---|---|---|---|---|
| **R1-hold: A-009 as written, primary (harness)** | +13.84% | +14.80% | -0.97 (-1.10) | **-0.58** | 3 of 9 |
| R1-reb: A-009 with the weekly trade back to 1/39 | +16.12% | +14.80% | (+1.27) | 0.62 | 5 of 9 |
| B: top fifth, quarterly | +13.68% | +15.28% | (-1.68) | -0.72 | 4 of 9 |
| D: top fifth inside each sector, quarterly | +14.67% | +15.28% | (-0.52) | -0.26 | 5 of 9 |
| R2: bottom fifth inside each sector, quarterly | +13.80% | +15.28% | (-1.00) | -0.55 | 3 of 9 |

R1-hold by year (difference, points): 2018 -5.9; 2019 -1.4; 2020 -10.1; 2021 +1.9; 2022 +10.2; 2023 -0.2;
2024 -0.5; 2025 -4.7; 2026 +0.9. Costs at 0x / 1x / 2x: +14.12% / +13.84% / +13.55% a year.

Decision rule (item 4): a mean yearly excess above 0, a raw t of 2.0 or more, above 0 in 7 of 9 years, and T2 to T4
pass. R1-hold: -1.10, t -0.58, 3 of 9: **fails on each part**. The deflated t is not needed: the raw t is below 0.
The variant count of the harness: 20 runs of the hypothesis and of its family (19 prior, 1 logged).

| Blind projection test (corrected prices) | Rule | Value | Verdict |
|---|---|---|---|
| T2: top less bottom decile, 13 weeks | mean d_w > 0, p < 0.05 | +0.16%, p 0.43 (block 13: 0.43; 52: 0.42); 8 of 13 phases | fail |
| T3 rule A: entries less the base rate | mean > 0, p < 0.05 | +0.83 points, p 0.30; hit rate 48.2% | fail |
| T3 rule B: the stated tail | below P10 5-15%, below P5 2-8% | 11.4% (8.8 to 14.1); 5.8% (3.7 to 8.2) | pass |
| T4: at any time | 7 of 9 years, both halves, 2 of 3 periods | d_w above 0 in 4 of 9 years; half 1 -1.16% | fail |

The deciles have no order: decile 0 +0.27%, decile 1 +0.56%, decile 9 +0.41%, the middle deciles -0.44% to +0.09%.

## Checks

- The split correction: the k-test (k 2, 25, 0.1) and its control, the filing-date test and the live tests pass
  (gate PASS-WITH-NOTES, `afa1ab6`); the real-path k-test of the validator matched to 1e-15.
- The runs used the binary of 2b77809 (before the switch to `market::splits`). Rebuilt from HEAD 8a513d4 and
  run again: all nine outputs are byte-identical (`cmp`). So the code switch changes nothing.
- The truncation test at 20 rebalance Fridays (every 23rd from 2018-01-05; prices and curve kept to D, the run to the
  next trading day): 17 identical. 2018-11-23 differed, and that difference is an artefact of the test, not a
  look-ahead. `index_from_cache` gives no market index when SPY has fewer than 520 rows, and SPY cut at 2018-11-23 has
  507 (the store starts 2016-11-18), so the cut run used beta 1. With SPY cut at 2018-12-14 (521 rows) the signal of
  2018-11-23 is the same as with the full data (MCHP 1.367609). The tests `value_rank_scores_do_not_move_with_a_market_
  close_on_the_date_or_later` (SPY closes on or after D times 3: no score or weight of D moves; a control moves) and
  `a_short_spy_file_gives_no_index` are in the suite (292b6dc). 2021-12-23 and 2025-07-03 (Thursdays before a closed
  Friday) are a rebalance day only if the next trading day is in the data: calendar knowledge, not a price, but not
  point in time; stated as a limit, not changed.
- A hazard for any test that cuts SPY: the beta is dropped with no message under 520 rows.
- Store `58e54a2` (the split table of record; prices as `1804f48`). The table cut 2016 of the run code:
  `trading/hypotheses/data/process_priors_cut2016.toml`. The list: `trading/hypotheses/data/a009_list.txt`
  (426 tickers run). Excluded before the runs: GSAT, BALL, EW, EXE, GPN, HWM.
- The harness log: `trading/runs/a009-weekly-hold-v1/` (two logged runs of the same command, the second a
  reproduction on the HEAD build).

## What it means

- The earlier stock-level edge (R1 +6.56 a year, t 3.44) came from the split look-ahead. With the share counts on
  the price basis, the automatic DCF model does not rank the S&P 500 names usefully over 2018 to 2026.
- What stays: the market-level engine (`fa dcf market`, pull request 133): the premium that the index price implies
  predicts the later 10-year real excess return, 1893 to 2016 (p 0.011, OOS R2 0.119 against 0.087), with the effect
  after 1953. That is a slow index signal, not a weekly stock rule.
- The weekly trade proposals do not start on the DCF rank. A new stock rule needs a new hypothesis, written before its
  run, and its confirmation needs data that this design did not use.

## Limits

Today's lists (survivorship); price returns; 9 years; one regime. The test is on the automatic model (a screen), not
on reviewed ticker models.
