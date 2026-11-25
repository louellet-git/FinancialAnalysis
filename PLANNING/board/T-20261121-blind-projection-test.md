---
id: T-20261121-blind-projection-test
title: Test the blind projections of the DCF engine on prior data before the first weekly trade
area: engine
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: fundamental
order: 0.02
branch: claude/t1-revenue-band
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

**Blindness of the T1 re-run (2026-11-21, before any score run of a fix; condition of the Head of Quality and Risk):**
choice (a). The blind score of the fix is `fa dcf pit --cut 2015 --half 1` only (the fix is fitted and chosen on half
0). The store has no year after the last year of the T1 data, so no fresher hold-out exists now. The full-sample T1
and the T2 to T4 runs after the fix are reported as "not blind (designed after the first T1 failed; half 0 fitted
the fix)", with the count of tries. The fix is a model change: it takes the gate of a model change and the
Backtest row. The first live weeks give the fresh hold-out.

**Result of the T1 fix (2026-11-21, `claude/t1-fcf-band` 49a1d82, A-010):** the blind score on half 1 FAILS on
revenue only. Free cash flow passes at T+1 to T+3 (13.9 / 76.9 / 9.2; 11.2 / 79.6 / 9.3; 10.6 / 78.3 / 11.1).
Revenue at T+3 fails (3.8 / 87.6 / 8.6; the band is too wide, an error on the safe side). Half 1 is now seen.

**The test of a revenue fix (written before any run of it):** develop on `fa dcf pit --cut 2015 --half 0` only,
one rule with a reason that is a fact of half 0. Score once on a hold-out that no run has scored alone: the earlier
base years, `fa dcf pit --cut 2011` limited to base years 2011 to 2014 (a new option `--base-to 2014`), with
`acq_cv=1` and the revenue rule, both halves. Label it "partly blind": earlier full-sample runs mixed these years
with later years. Pass: the T1 rule on the "all" rows, revenue and free cash flow, T+1 to T+3. No fully blind
fundamentals data exists in the store; the next 10-K filings are the only fresh data.

**Status at the pause (2026-11-21 15:15 UTC; owner word: "hold off on other work for now", for the S&P 100 week
attribution).** The work of the agent is committed as is, with no review (branch `claude/t1-revenue-band`): step 0 (the
port of `acq_cv` from 49a1d82 by hand, with A-010 and the register row), step 1 (`fa dcf pit --base-to YEAR`) and a
revenue diagnosis `--diag rev` (step 2, development on `--cut 2015 --half 0` only). Runs made: four half-0 runs (base,
`--diag fcf`, two `--diag rev`) and one half-1 run with `acq_cv=1` (the port check of step 0, asked by the brief).
No revenue rule is chosen, and the score run (`--cut 2011 --base-to 2014`) is NOT made: that hold-out is still unused.
Not done: the fixed-answer test of `--base-to`, the build and test check of this commit, step 3 (write the rule into
this ticket before the score). Next step: `cargo test`, then step 2 from the `--diag rev` output.

**Board note (2026-11-21 about 20:50 UTC):** paused by the owner word "hold off on other work for now" (the week attribution is first); back to the backlog, with the work kept on its branch (see the status at the pause above).

**Restart (16:29 UTC, 2026-11-25).** In progress again (louellet-git, branch claude/t1-revenue-band). The person agreed to the
week of 2026-11-25 with this row in it ("Agreed to the week ... Proceed", record of the Head of Staff, 16:13 UTC); this
lifts his pause of 2026-11-21. Share of the week for this ticket: about 6% of the token limit; no agent is planned.
The branch is merged with main 0e817ff (three text conflicts; tests 1061 + 17 + 1 pass). No pit run was made after the pause.

**Revision 3, written before any run after the restart.** It changes no pass rule. It names what moved since the design.
- Data store: the design named store 1804f48. The runs after the restart use store 2d4619d. Code: this branch with main
  0e817ff in it. Reason: main now has the fix of the old revenue tags (pull request 200), which fills empty revenue periods
  before 2018 and adds company-years, most of them in the early base years. The fix must hold on the data and the code
  that the engine uses now.
- Effect on the hold-out. The score of the revenue fix is still `fa dcf pit --cut 2011 --base-to 2014` with `acq_cv=1`
  and the revenue rule, both halves, one time. No run has scored these base years alone, before or after the restart.
  The label stays "partly blind". The new company-years of pull request 200 are in the hold-out; this head did not look
  at them.
- Development stays on `--cut 2015 --half 0` only. The first run after the restart is the half-0 base run on the new
  store and code (with `acq_cv=1`), because the numbers of the old store are not the base of the new one. It is a
  development run, not a score. Each half-0 run is counted below.
- The rule for the revenue band is written in this ticket, with its reason from half 0 and its numbers, and pushed,
  before the score run. One rule only. If the score fails, the fix fails and the person gets the numbers; a second fix
  needs a new written test.
- After a pass: T1 on the full sample one time, and T2 to T4 one time, both labelled "not blind", on the new store.
  The earlier full-sample numbers were made on the old store; the report gives both.
Count of runs after the restart: 1 (one half-0 development run; no score run).

**Half-0 base run after the restart (16:33 UTC, 2026-11-25; development, not a score).** Command: `fa dcf pit --cut 2015
--half 0 --rules acq_cv=1 --diag rev --cache <store 2d4619d>`, code c2a2ab7. 4050 company-years of 602 companies.
Cells: below P10 / inside / above P90, in percent (pass: inside 75 to 85, each tail 5 to 15).
- Revenue, "all": T+1 6.3 / 84.2 / 9.5 (pass); T+2 5.6 / 84.1 / 10.3 (pass); T+3 3.5 / 88.1 / 8.3 (FAIL: the low tail
  is too thin and the inside share is too large). T+1 and T+2 pass by less than 1 point of the inside share.
- Free cash flow, "all": T+1 15.4 / 76.1 / 8.4 (FAIL by 0.4 point on the low tail); T+2 10.8 / 80.0 / 9.1; T+3 10.6 /
  79.1 / 10.3. On half 1 (old store, the blind score of A-010) T+1 was 13.9. The difference between the two halves
  (1.5 points) is the size of the sampling noise of a tail share. RISK for the score: the free cash flow rule is fixed
  (A-010), and its T+1 low tail is at the limit; the score can fail on it with a correct revenue rule.
- The diagnosis (z = the real value on the scale of the band; a calibrated band has -1.28 and +1.28 at the 10th and
  90th percentile). All: T+1 -1.08 / +1.07; T+2 -1.03 / +1.16; T+3 -0.81 / +1.06. Real below P50: 52%, 47%, 45%; the
  median real revenue is above P50 by -0.5%, +1.1%, +3.1%.
- Reading of this head (two facts of half 0): (1) the centre of the band is too low at T+3 (the P50 falls behind by
  about 3%); (2) after the centre is corrected, the band is too wide by about 15% at T+1 and T+2 and by about 27% at
  T+3. After a year of falling revenue the low side is far too wide (T+3: -0.66 / +1.26). By width of the band, the
  bands of 15% to 25% at T+1 are the worst (-0.93 / +0.86).
- NOT decided: the rule. Two candidates need one more fact before a choice: (a) one scale on the uncertainty of the
  company mean growth (the part of the band that grows with the horizon); (b) the floor of the growth after a decline.
  The fact that decides: the share of the band variance at T+1, T+2 and T+3 that comes from the mean uncertainty, from
  the shock and from the fade (a new diagnosis on half 0). One rule only, written here before the score.
- Why no hurry: the hold-out is scored one time. A weak rule fails it, and no second partly blind sample exists before
  the next 10-K filings.
Next step: the variance diagnosis on half 0 (code and one run), then the rule in this ticket, then the gate of the
statistics (Head of Economics) on the rule text, then the one score run.
