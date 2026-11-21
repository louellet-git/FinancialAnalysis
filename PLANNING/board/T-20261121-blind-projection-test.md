---
id: T-20261121-blind-projection-test
title: Test the blind projections of the DCF engine on prior data before the first weekly trade
area: engine
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: fundamental
order: 0.02
branch: claude/dcf-engine-backtest
created: 2026-11-21
acceptance: Before the first weekly trade (paper included, 2026-11-27), the four tests below run once as written, on point-in-time data only, and their results (pass or fail, each with its number and command) are in a findings file that passed the gate of the Head of Quality and Risk; each weekly write-up then states the backtested chance of a loss and the size of a bad outcome for its rule, so that a failed trade is one that the model gave a chance before the trade.
---
Owner word, 2026-11-21 04:29 UTC: "Ensure that before you have trades and we begin the weekly trades, that you have
effectively back tested your future state blind projections using the DCF engine and using prior data. A
recommendation should work at any time and point and when it doesn't, the model should have already priced it in
such that it does work. You can speak w the economist about this."

Reading of the head: (1) the projections of the engine, made blind (only the facts and prices known at the date),
must be calibrated against what happened; (2) a trade can fail, but the rate and the size of the failures must
be what the backtest of the same rule said before the trade. A rule that works only in some years fails (2).

Design (written 2026-11-21 before any of these runs; review by the Head of Economics and of Quality and Risk;
data store `1804f48`; parameter table cut at 2016 for the stock tests, `--cut 2015` for the projection test):
- **T1. Projections of the fundamentals.** `fa dcf pit --cut 2015` (base years 2015 and later, table from the
  years up to 2015): free cash flow and revenue at T+1, T+2, T+3. Pass: in the "all" row the share of real values
  inside P10 to P90 is 75% to 85%, and the share below P10 and the share above P90 are each 5% to 15%. A class row
  that fails is named, and the weekly rule must exclude that class or its band must be fixed and tested again.
- **T2. The signal at every date.** Each week of 2018-01 to 2026-11, each scored ticker of the S&P scored list:
  the rank inside its sector (A-009 item 3) against the return of the next 13 weeks less that of the equal weight.
  Deciles of the rank. Pass: the top decile less the bottom decile is above 0 with a one-sided p < 0.05
  (block bootstrap by week, block of 13 weeks, seed 7), AND it is above 0 in at least 7 of the 9 calendar years.
- **T3. The trades of the rule.** The entries of R1 (A-009): the share of entries with a 13-week excess return
  above 0 (the hit rate), the mean, and the P10 and P5 of the 13-week excess return. Pass: hit rate above 50% with
  a one-sided p < 0.05 (the same bootstrap). These numbers are the "priced in" figures of each weekly write-up.
- **T4. At any time.** T3 by calendar year and by half of the list (`ticker_half`). Pass: the mean 13-week excess
  of the entries is above 0 in at least 7 of 9 years and in both halves. The worst year and its numbers are
  stated in each write-up.
If a test fails, the weekly trades do not start on its basis; the head reports the failure to Luc with the numbers.
No rule is changed after a result to make a test pass. Survivorship, price returns and today's lists stay as limits.

**Revision 1, 2026-11-21 04:31 UTC, before any run of T1 to T4** (review of the statistics by the Head of
Economics). It replaces the pass rules of T2 to T4 above. T1 does not change.
- **T2.** Each week w: d_w = the mean 13-week excess of the top decile less that of the bottom decile. The excess
  is the return less the return of the equal weight of the scored list of the same week. Primary: a one-sided
  p < 0.05 for mean(d_w) > 0 from a circular block bootstrap of the d_w series, block 26 weeks, seed 7. Checks
  that are reported: block 13 and 52, and the mean of the 13 non-overlapping phases (every 13th week). Power:
  about 34 independent 13-week periods.
- **T3.** For each week with entries: hit(entries) less the share of all scored names with an excess above 0 in
  that week. Pass: the mean over the weeks is above 0, with a one-sided p < 0.05 (the same bootstrap).
  **The "priced in" test:** from 2019-01, at each week, the stated P10 and P5 are the empirical P10 and P5 of
  the 13-week excess of all entries that closed before that week (an expanding window). Pass: the share of the
  later entries with an outcome below the stated P10 is 5% to 15%, and below the stated P5 it is 2% to 8%. The
  block bootstrap gives an interval.
- **T4.** Pass: the mean 13-week excess of the entries, and d_w of T2, are above 0 in at least 7 of 9 calendar
  years, in both halves of the list, and in at least 2 of the 3 periods 2018-2020, 2021-2023 and 2024-2026.
  Descriptive only: d_w, the entry excess and the tail coverage in the stress windows of
  `analysis/macro/2026-11-21-stress-windows.md` (on `main`), and the worst 13-week d_w with its date.
- **Headline:** T1 to T4 must all pass for the weekly trades to start on this basis.
- **Traps:**
  - Survivorship: the list is today's S&P 500, so the bottom decile looks better than it was. This bias makes T2
    harder to pass, not easier. Its size is not known.
  - The automatic model reads no consensus estimate and no price target. The store has one snapshot only
    (2026-11-19).
  - The sector is today's SIC code.

**Revision 2, before any run of T1 to T4** (choices of the build that Revision 1 does not give; the head accepts them):
- The tail test of T3 uses a week only when 30 or more entries closed before it (closed = entry day + 91 days
  before the week). A stated P10 from fewer outcomes is not a stated figure.
- The stress windows are read from `analysis/macro/stress-windows.csv`, a copy of the table of
  `analysis/macro/2026-11-21-stress-windows.md` (descriptive only).
- The last 13 weeks of the data have no outcome and are not in the statistics.
- One command runs T2 to T4: `fa portfolio calibrate --start 2018-01-01 --tickers-file <scored list> --cache
  <store 1804f48> --table <table cut 2016, SHA-256 3a74b20f...> --block-weeks 26 --boot-seed 7 --reps 10000`.
  T1 is `fa dcf pit --cut 2015` with the code of this branch (the acquisition rule of `main`; if the acquisition
  change merges before the first trade, T1 runs again once with it, and both runs are reported).
- The refactor of `fa portfolio backtest` for the shared code is checked: run D is byte-identical.

**Result of the one run (2026-11-21 04:49 UTC):** T1 FAILS (the free cash flow low tail is too thin, revenue at T+3
too wide); T2 to T4 pass (`analysis/priors/blind_projection_test_2026-11-21.md`). Owner decision, 2026-11-21
04:50 UTC: "No trades until T1 passes", paper included.

**The test of the T1 fix (written before any run of a fix):**
- Development on `fa dcf pit --cut 2015 --half 0` only. The agent may look at half 0 as often as needed. It
  chooses ONE rule for the band of the free cash flow, and if needed ONE rule for the revenue band at T+2 and T+3.
  Each rule has a reason that is not a score of half 1 (for example the size of the margin shocks seen in the
  history of half 0, or a lower tail of the margin shock). The rule and its numbers are committed before step 2.
- Score: `fa dcf pit --cut 2015 --half 1`, once. Pass: the T1 rule ("all" row, revenue and free cash flow, T+1 to
  T+3: inside P10 to P90 75% to 85%, each tail 5% to 15%) holds on half 1.
- Then T1 on the full sample (`fa dcf pit --cut 2015`), once, and T2 to T4 (`fa portfolio calibrate`, the same
  command), once, with the fixed model. All must pass. If the score of half 1 fails, the fix fails: report it
  to Luc; a second fix needs a new written test.
- `fa dcf drivers IBM` stays $83.84 / $151.84 / $258.03; a changed value gets its record (`docs/assumptions/`).

**Fix chosen (before the score of half 1).** The blind score of this fix is half 1 only (`fa dcf pit --cut 2015 --half 1`,
once). The full-sample T1 and the T2 to T4 runs after it are not blind: they were designed after the first T1 failed, and
half 0 fitted the fix.
- Rule `acq_cv` (`--rules acq_cv=1`; default off until the score of half 1): the cash paid for businesses is a draw, not a
  fixed ratio. Year 1 of the charge process is a draw with mean = the charge of A-002 (w times the mean spend ratio) and
  standard deviation = the population standard deviation of the company's own spend ratios (the last 5 years, the whole
  spend, not w times it); the shock of later years is that size over the year-1 charge, times the charge of the year (it
  falls with the fade); the floor is 0 and the cap is 1. Numbers set by a procedure: none (no tuned number).
- Reason (fact of half 0): the margin of the next year is not the cause. Its one-step z after the cut is 6.5% below -1.28
  and 7.9% above +1.28 (sd 1.07), so the margin band is not thin. The thin low tail is the acquisition outflow: the real
  free cash flow subtracts the cash paid for businesses, the model charged it as a fixed ratio. Free cash flow below P10
  by the spend of the target year (T+1): 10.9% with spend below 0.5% of revenue, 58.4% with spend above 5%. Lower side
  of the band that the real values need: 1.88 times the model band (T+1), upper side 1.06.
- Alternatives on half 0 (8 half-0 runs in all, 5 of them diagnoses): (1) shock = cv of the charge, w times smaller
  size: 17.4/73.7/8.9 at T+1; (2) whole-spend size with a floor of 0.25 on w: 15.7/75.6/8.7; (3) whole-spend size with no
  floor on w (chosen, no extra number): 15.3/75.8/8.9. A heavier tail than the normal (lumpy deals) was not tried.
- Revenue: no rule. No fact of half 0 or of the history gave a rule that is not a fit; revenue at T+3 (3.2/88.4/8.4 on
  half 0) is expected to fail on half 1 too. This is reported, not fixed.
- Half 0 before (T1 rule, `--half 0`, "all", below/inside/above at T+1, T+2, T+3):
  free cash flow 18.6/70.3/11.1, 16.4/72.4/11.3, 15.6/72.3/12.1; revenue 5.8/84.3/9.9, 5.1/84.3/10.6, 3.2/88.7/8.1.
  After (`acq_cv=1`): free cash flow 15.3/75.8/8.9, 11.8/79.0/9.2, 11.0/78.6/10.4; revenue 5.9/84.0/10.1, 5.1/84.3/10.6,
  3.2/88.4/8.4. Half 0 at T+1 still has 15.3% below P10 (rule 5 to 15).
- Files of the runs: `/tmp/claude-0/-home-user-FinancialAnalysis/5f637b6c-5bd4-5e2d-967a-4d70bf24bed7/scratchpad/t1fix/`.
