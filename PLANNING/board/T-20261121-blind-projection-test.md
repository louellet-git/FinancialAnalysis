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
Count of runs after the restart: see the last count line of this ticket.

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

**Five more half-0 development runs (16:37 UTC, 2026-11-25): one rule switch each; a rule that passes is REJECTED.**
Command of each: the base command with one more rule (`growth_w=1`, `growth_w=0`, `own_sigma_nu=0`,
`own_sigma_nu=100`, `last_floor=-1`). Rows "all", below P10 / inside / above P90 at T+1, T+2, T+3:
| Rule | Revenue | Free cash flow |
|---|---|---|
| base (`acq_cv=1`) | 6.3/84.2/9.5; 5.6/84.1/10.3; 3.5/88.1/8.3 | 15.4/76.1/8.4; 10.8/80.0/9.1; 10.6/79.1/10.3 |
| `growth_w=1` | 8.7/81.1/10.3; 9.0/80.0/11.0; 9.2/80.1/10.7 | 14.2/76.4/9.4; 11.8/77.8/10.4; 11.9/77.4/10.8 |
| `growth_w=0` | 5.9/85.2/8.9; 5.0/85.3/9.7; 2.8/89.6/7.6 | 20.9/71.9/7.2; 18.7/73.1/8.2; 17.6/73.9/8.5 |
| `own_sigma_nu=0` | 5.7/85.9/8.4; 5.2/85.4/9.4; 3.6/88.6/7.9 | 15.3/75.5/9.3; 10.9/79.6/9.5; 10.0/79.8/10.2 |
| `own_sigma_nu=100` | 5.8/85.8/8.4; 5.2/85.3/9.5; 3.4/88.7/7.9 | 15.2/75.8/9.0; 11.0/79.7/9.3; 10.1/79.9/10.0 |
| `last_floor=-1` | 6.0/83.9/10.1; 5.3/83.5/11.2; 3.2/87.4/9.4 | 15.4/76.1/8.5; 11.1/79.5/9.5; 10.8/78.5/10.7 |
- `growth_w=1` (full weight on the own growth history, no uncertainty of the company mean) passes the "all" rows of
  revenue and of free cash flow at each horizon. **This head REJECTS it as the rule.** Its class rows show that the
  pass is two opposite errors that cancel: companies with a growth of 20% to 50% in the base year have a real revenue
  17% below the P50 at T+3 (with the default weight: 4% below), those above 50% are 38% below (default: 5% below), and
  those with falling revenue are 13% above (default: 11% above). The shrink of the company mean to the mean of its
  size class is necessary for the centre. A rule that passes the "all" row and makes the centre of the fast growers
  worse is a pass of the letter of T1, not of its purpose.
- What the runs show: the centre needs the shrink (the default weight); the band is too wide because of the term for
  the uncertainty of the company mean, (1 - w) tau^2. In the middle classes (growth 0% to 20%) the band with no such
  term is calibrated (`growth_w=1`: 8.1/83.4/8.5 and 7.0/84.7/8.2 at T+1).
- Checked in the code: the table value tau already has the correction for the noise of a short history
  (`calibrate.rs`, the variance of the company means less the mean noise). So tau is not too large for that reason.
- Next candidate, NOT run: tau is a variance of company means, and a few extreme companies can make it large for all
  companies of the class. A spread from the quartiles of the company means (a robust spread) can be the one rule, with
  the half-0 fact as its reason. It needs an option in the table estimate, one half-0 run, and the class rows.
- The rule is still not chosen. The hold-out is not scored.

**Candidate `tau_robust`, written before its half-0 run (16:59 UTC, 2026-11-25; development).** Definition: for the growth
only, the spread tau of the company means of a size class is calculated from the quartiles. tau^2 = (the 75th
percentile of the company means less the 25th percentile, divided by 1.349)^2, less the MEDIAN of the noise of the
company means (today: the variance of the company means, less the MEAN of the noise). Floor 0. The weight w and the
term (1 - w) tau^2 use this tau. No other change; no free number (1.349 is the distance between the quartiles of a
normal distribution in standard deviations). Switch: `--rules tau_robust=1`, off by default.
- Reason from half 0 (the runs above): the band is too wide through the term (1 - w) tau^2, and the centre of the
  growth classes is better with less weight on the own history (weight 0: falling revenue +6.2% at T+3 against +10.9%;
  growth 20% to 50%: +1.1% against -4.0%). A smaller tau moves the two in the correct direction. The band is a P10 to
  P90 band: the spread of the middle of the companies is the correct measure for it, and a variance is sensitive to a
  few extreme company means.
- What this head reads after the run: the "all" rows, the rows by growth class and by size (centre and tails), and the
  diagnosis by band width. A pass of the "all" rows with a worse centre in a class is not a pass (as for `growth_w=1`).
- Risk seen before the run: in a cell where the noise is larger than the robust spread, tau is 0, the weight is 0 and
  the band has no term for the company mean. The 30-100B class has tau 0 today (its rows are equal with weight 0),
  and its centre at T+3 is the worst (+7.4%).

**Result of `tau_robust` (17:03 UTC, 2026-11-25; half-0 development run 7): REJECTED.** Command: the base command with
`tau_robust=1`, code d6fc94b plus the switch (not committed yet). Rows "all", below P10 / inside / above P90:
- Revenue: 6.6/83.6/9.8; 5.8/82.5/11.7; 4.4/85.1/10.6. T+3 fails (low tail 4.4, inside 85.1).
- Free cash flow: 19.8/72.7/7.5; 16.4/75.0/8.6; 16.3/74.5/9.2. All three horizons fail (base: 15.4, 10.8, 10.6 below P10).
- Why: the robust tau is near 0 in most classes, so the weight of the own history is near 0 (the rows are near those of
  `growth_w=0`). The acquisition charge of A-002 is the weight times the expected spend: with a weight near 0 the
  charge is near 0, the P50 of the free cash flow goes up (real below P50: 56% at T+1 against 49%), and the low tail
  fails. Fact for each later candidate: a rule that changes the weight w changes the free cash flow through A-002.
- Fact from the diagnosis rows of all runs: the excess width is in the companies with a growing base year (base run,
  10th and 90th percentile of z: -1.05/+0.91 at T+1 and -0.85/+0.96 at T+3; a calibrated band has -1.28/+1.28). The
  band of the companies with a falling base year has about the correct width (-1.16/+1.34 at T+1), and its centre is
  too low at T+2 and T+3.

**Candidate `level_last`, written before its half-0 run (17:03 UTC).** Found in the code (`auto.rs`, the shock size):
the shock of the growth is at least kappa times a growth level, and the level is the LARGEST absolute growth of the
history window (the last 4 growth values, with the base year). kappa is measured from the fade table as the spread of
the growth one year after a start year, over the growth of the START year (`level_spread_ratio`). So the measurement
is for the growth of the last year, and the model uses it on the largest growth of the window: one year of high growth
in the past keeps the band wide for each later base year. Definition of the candidate: the level is the absolute
organic growth of the base year; if the base year has no organic growth (a jump year), the last value of the history.
No other change, no free number. Switch: `--rules level_last=1`, off by default.
- Reason: it makes the use of kappa agree with its measurement. It does not come from a fit of the band.
- Expected, before the run: narrower bands for companies with a growing base year and a past year of higher growth;
  no change for a company whose base year has its largest growth. A smaller shock gives a larger weight w for these
  companies (less noise in the history mean): read the centre of the growth classes and the free cash flow rows.
- Second run for a bound, not a candidate: `level_kappa=0` (no level term). It shows the largest possible effect.
- The same read as before: a pass of the "all" rows with a worse centre in a class is not a pass.

**Results of `level_last` and of the bound `level_kappa=0` (17:09 UTC, 2026-11-25; half-0 development runs 8 and 9):
`level_last` is NOT the rule.** Rows "all", below P10 / inside / above P90 at T+1, T+2, T+3:
| Rule | Revenue | Free cash flow |
|---|---|---|
| `level_last=1` | 7.0/83.0/10.0; 6.0/83.3/10.7; 3.9/87.6/8.6 | 15.4/76.0/8.6; 10.9/79.8/9.2; 10.7/79.0/10.3 |
| `level_kappa=0` (bound) | 7.4/82.2/10.4; 6.3/82.9/10.7; 4.2/87.3/8.5 | 15.4/75.9/8.7; 11.0/79.7/9.3; 10.7/79.0/10.3 |
- The level term is a small part of the width. With no level term at all, T+3 of the revenue still fails (4.2 below
  P10, 87.3 inside). The mismatch between the measurement of kappa and its use is real, but it does not decide T1; it
  stays as a note for the model document.
- Fact from the diagnosis of all nine runs: the real errors have fat tails on the scale of the band. Base run, T+1:
  the 10th and 25th percentile of z are -1.08 and -0.46 (ratio 2.3), the 75th and 90th are +0.43 and +1.07 (ratio
  2.5). A normal distribution has the ratio 1.9; a Student distribution with 2 to 3 degrees of freedom has 2.1 to 2.3.
  A band from a normal draw with the correct variance is too wide at P10 and P90 for such data. The robust spread of
  the company means near 0 (run 7) says the same about the company means: most companies are near the class mean, and
  a few are far.
- Found in the path engine, a note and not a candidate: the draw of the company mean (the target) keeps its full
  deviation for all 10 years, while the mean itself fades to the long-run level. The ramp is about the 10 years of
  the projection, so the effect at T+3 is small (about 10% of the mean term); it matters for the value band. It goes to
  the model-change gate as a separate question.

**Candidate `band_q`, written before its half-0 run (17:09 UTC).** Principle: the outputs of the model are P10 and P90,
and the growth data have fat tails. So the width of each normal draw of the growth comes from the 10th and 90th
percentile of the calibration data of the size class, not from their standard deviation. The variances stay where a
variance is the correct input: the weight w, the company mean and the acquisition charge of A-002 do not change.
- Shock: the draw uses sigma times r_s. r_s = (P90 less P10 of the within residuals of the growth model of the class)
  / (2.563 times the standard deviation of the same residuals); companies with one observation are not in it.
- Company mean: the draw of the target uses the root of (1 - w) tau^2, times r_t. r_t = (P90 less P10 of the company
  means of the class) / (2.563 times the standard deviation of the same company means).
- Year 1: the start draw uses the two scaled terms. The level term (kappa times the level) does not change: kappa is
  measured from P10 and P90 already.
- A ratio is 1 for a normal sample and below 1 for a sample with fat tails. No free number: 2.563 is the distance
  between the 10th and the 90th percentile of a normal distribution in standard deviations. The ratios come from the
  table of the cut (the years before the base years), not from the band test.
- Switch: `--rules band_q=1`, off by default.
- Known weak point, before the run: the residuals of all companies of a class are one sample. Companies with different
  shock sizes make this sample fat-tailed even when each company is normal, so r_s can be too small and the band too
  narrow. The run shows it: a band that is too narrow fails on the two tails.
- Known risk, before the run: the free cash flow band uses the revenue paths. A narrower revenue band with the same
  centre moves the low tail of the free cash flow up, and its T+1 value is 15.4 on half 0 (limit 15).
- The same read as before: the "all" rows, the class rows (centre and tails) and the diagnosis.

**Result of `band_q` (17:17 UTC, 2026-11-25; half-0 development run 10).** Command: `fa dcf pit --cut 2015 --half 0 --rules
acq_cv=1,band_q=1 --diag rev --cache <store 2d4619d>`. 4050 company-years of 602 companies. The unit tests of the new
code pass (1063 + 17 + 1, 0 failed).
| Rows "all" | T+1 | T+2 | T+3 |
|---|---|---|---|
| Revenue, base | 6.3/84.2/9.5 | 5.6/84.1/10.3 | 3.5/88.1/8.3 (fail) |
| Revenue, `band_q` | 8.2/80.2/11.6 (pass) | 8.2/78.8/13.0 (pass) | 6.0/82.7/11.3 (pass) |
| Free cash flow, base | 15.4/76.1/8.4 (fail by 0.4) | 10.8/80.0/9.1 | 10.6/79.1/10.3 |
| Free cash flow, `band_q` | 15.5/76.0/8.5 (fail by 0.5) | 10.9/79.5/9.5 | 10.7/78.3/11.0 |
- The shape ratios of the table of the cut (shock / company means), standard template: <1B 0.61/0.80; 1-10B 0.83/0.85;
  10-30B 0.87/0.66; 30-100B 0.76/0.67; all sizes 0.68/0.64.
- The diagnosis (10th and 90th percentile of z; calibrated: -1.28/+1.28). All: T+1 -1.28/+1.28; T+2 -1.23/+1.41; T+3
  -1.03/+1.28. By band width at T+1: -1.37/+1.28; -1.28/+1.30; -1.25/+1.25; -1.12/+1.20. The width is correct in each
  class of band width; the base run had -0.93/+0.86 in its worst class.
- The centre does not change (by design): the median error of the "all" row is -0.5%, +0.7%, +2.0%.
- Class rows, the read of this head. Better: growth 0% to 10% (8.4/81.9/9.7 at T+1; base 6.6/85.6/7.8), 10% to 20%
  (6.4/84.1/9.5; base 4.4/87.9/7.7). Worse: a falling base year (9.6/72.8/17.6 at T+1 and 5.7/72.5/21.8 at T+2; base
  7.3/78.4/14.3 and 4.2/78.2/17.6). For these companies the band was about correct in width and the centre is too low
  at T+2 and T+3 (+6.3%, +9.5%); the wide band hid a part of the centre error. This is a known error of the centre
  (A-003), not of the width, and this rule does not correct it. Growth above 50% (234 company-years): the median error
  at T+3 goes from -4.8% to -9.9% (the P50 moves with the width of the draws); the tails stay in the limits.
- The free cash flow does not change with the rule. Its T+1 low tail is at the limit on half 0 with and without the
  rule (15.4 and 15.5; half 1 on the old store: 13.9). It is a matter of A-010 (the size of one year of acquisitions),
  not of the revenue band.

**THE RULE for the revenue band (chosen 17:17 UTC, 2026-11-25, before any score run): `band_q=1`.** Text of the rule:
the width of each normal draw of the revenue growth (the shock, the company mean, year 1) is its standard deviation
times the shape ratio of the size class, where the shape ratio is (P90 less P10) / (2.563 times the standard
deviation) of the same calibration sample (the within residuals for the shock; the company means for the company
mean). The table of the cut gives the ratios. The weight of the history, the company mean, the level term and the
acquisition charge do not change. A cell with fewer than 20 values has the ratio 1.
- Reason: the outputs are P10 and P90, the growth data have fat tails (z ratios of 2.3 to 2.5 against 1.9 for a
  normal distribution), and a normal draw with the correct variance is then too wide at P10 and P90. The ratio is
  measured on the years before the base years. It is not a fit to the band test.
- Why not the other nine: see the runs above (two errors that cancel; a change of the weight breaks the free cash
  flow through A-002; the level term is too small to matter).
- Limits that this head states before the score: (1) ten development runs on half 0 came before this choice, so half 0
  is not a test of it; (2) the residuals of one class are one sample, and a mix of companies with different shock
  sizes also gives a ratio below 1; (3) the band of the companies with a falling base year becomes too narrow for its
  centre error; (4) the free cash flow T+1 low tail can fail the score with a correct revenue rule.
- The score (one time, both halves): `fa dcf pit --cut 2011 --base-to 2014 --rules acq_cv=1,band_q=1`, store 2d4619d,
  the code of this commit. Pass: the rule of T1 as written at the top of this ticket (rows "all", revenue and free
  cash flow, T+1 to T+3: inside 75% to 85%, each tail 5% to 15%). No pass rule changes. The report gives the revenue
  rows and the free cash flow rows as two separate facts, with the class rows.
- Before the score: the statistics gate (Head of Economics) reads this rule text. No score run before its answer.
Count of runs after the restart: 10 (ten half-0 development runs; no score run).

**Statistics gate on the rule text (17:21 UTC, 2026-11-25): PASS-WITH-NOTES (Head of Economics). The conditions, written
before the score run.**
- (i) The ratios of the score come from the table of the cut 2011, which the code calculates again: `pit_setup` builds
  the panel, cuts it at the year of `--cut` (`up_to_year`), and `estimate_table` calls `fit_ar1`, which calculates
  the two shape ratios from that panel. Checked in the code by this head. The output of the run prints the ratios of
  each cell. The values of the cut 2015 are not used.
- (ii) Ten development runs came before the choice of the rule, all on half 0 with the cut 2015. The companies of half
  0 are in the score again, with their base years 2011 to 2014; the real values of T+1 to T+3 of those base years are
  the years 2012 to 2017, and a part of them was history in the development runs. Half 1 is the cleaner part. Label:
  "partly blind".
- (iii) One score. The rule does not change after it. A fail is reported as a fail.
- The score is three commands in one go, with the same store, code, seed and rule: all companies (this run decides),
  `--half 0` and `--half 1` (the same companies in two parts, reported next to it). The table of each command comes
  from all companies up to 2011.
- Notes of the gate for the report: the pooled band of a class is correct on average and wrong for one company when
  the companies have different shock sizes (too narrow for a company with large shocks, too wide for one with small
  shocks); the sum of three scaled normal parts is not exactly the scaled sum (an approximation); give the number of
  values of each cell. The class row of the falling base year is a limit to state (centre error, A-003), not a new
  pass rule.
- The result of T1 keeps its written pass rule. If the free cash flow fails, T1 is "fail (free cash flow ...)", and
  the revenue result is a separate fact below it, not a T1 pass.
