---
id: T-20261120-dcf-engine-accuracy
title: Measure the weak points of the stochastic DCF engine and correct them one at a time
area: engine
status: review
owner: louellet-git
head: fundamental
order: 1.3
branch: claude/dcf-engine-accuracy
created: 2026-11-20
acceptance: Each part has a measurement before the change and after it (the point-in-time test, or the standard error of the P50), a test with a fixed answer, and a record of the choice; a part that gives no measured gain is not merged.
---
Owner word, 2026-11-20: "Immediate goal is basically optimizing / improving the DCF engine. It's a
stochastic engine for calculating various simulations of what a stock price or asset price should be.
We're going to stick to underlyings and not expand into options and other stuff for now."

Scope: the automatic path model and the path engine, for stocks. No option work.

Candidate parts, from the evidence that the repo has (`docs/ARCHITECTURE.md`, section "The automatic
model"; `docs/HIDDEN-ASSUMPTIONS.md`, #194 and #199; the two studies of 2026-11-20). Measure each one first.
Do them one at a time, the largest measured error first:
1. Precision of the simulation: the standard error of the P50 is 1.8% of the P50 with 4,000 paths, and the
   engine makes no antithetic numbers. Aim: the same precision with fewer paths, or a better precision.
2. Base revenue: the model starts from the latest fiscal year, which can be 11 months old. Aim: start from
   the trailing four quarters when they exist, with the point-in-time rule.
3. Bands: the bands are too wide for a company with a stable margin (coverage 95% against a target of 80%),
   and the revenue P50 is too low after a year of decline (+11% at T+3) and at 10 years (+12% to +24%).
4. Parameter cells by sector: the growth and margin cells have a size and no sector. Needs the SIC code
   (`T-20261120-sic-code`).
5. Terminal value: the margin of year 10 pays for the growth of the history, not for the terminal growth.

Measured errors of the point-in-time test (`analysis/priors/auto_pit_2026-11-20.txt`, 6,030 company-years;
target: 10% below P10, 80% inside, 10% above P90). These two are the first parts, before the list above:
- A. After a year of decline (1,345 company-years, 22%): the real revenue is above P90 in 15% to 19% of the
  cases, and the P50 is 7.6% too low at T+2 and 11.3% too low at T+3. The model does not see the recovery.
- B. Stable margin (range below 5 points, 1,540 company-years, 26%): 95% of the real free cash flows are
  inside the band. The band is too wide. For a range above 10 points the band is right (78% to 80%).

Method: develop a rule on the companies A to CME (the 215 tickers of the partial cache). Score it on the
other companies when the data store has them. A test set does not select the rule that it scores.

Rule: no part is chosen or tuned with the market price (`docs/QUALITY-STANDARDS.md`, section 6).

**State and decisions, 2026-11-20 (Head of Fundamental Research).** Data of 2026-11-19 (data store), seed 7.
Development half and scoring half: parity of the FNV-1a hash of the ticker (`fa dcf pit --half 0|1`).
- **A, accepted (default on):** `last_floor = 0`: a base-year organic growth below 0% enters the company mean and
  the first step as 0% (record A-003). Decline class, scoring half, revenue P50 error T+1 / T+2 / T+3: +2.0 / +7.8 /
  +11.8% to +0.0 / +5.1 / +7.4%; `--cut 2015`: +3.1 / +12.7 / +17.9% to +1.2 / +8.5 / +13.6%. All company-years (full panel):
  -0.4 / +0.7 / +1.6% to -0.7 / +0.1 / +1.0%. Files: `analysis/priors/auto_pit_floor_{before,after}_2026-11-20.txt`.
  A recovery term is refused: the rest of the error is in a few base years with a common shock (2018 to 2021; A-003, #286).
  The numbers above were on the defaults before pull request 77 (`long_run` off, `acq_w` off). On the merged defaults
  (`long_run = 0.040`, `acq_w`, `acq_fade`; seed 7, 2,000 paths, full files `analysis/priors/auto_pit_floor_{before,after}_2026-11-20.txt`,
  before = `--rules last_floor=-1`): decline class scoring half +2.0 / +7.8 / +11.8% to +0.0 / +5.1 / +7.4%; `--cut 2015`
  +3.1 / +12.7 / +17.9% to +1.2 / +8.6 / +13.7%; all company-years, full panel -0.4 / +0.7 / +1.7% to -0.7 / +0.1 / +1.0%.
  Screen of `sp500.txt` (`--as-of 2026-11-19`), median P50 / price: 0.4745 floor off (as main), 0.4770 floor on.
  On the merged defaults the free cash flow bands of a stable margin are at 83.0 / 83.4 / 83.6% (scoring half): B has no case now.
- **B, not accepted (option only, `margin_sigma_nu`):** the band of a stable margin is too wide in sample (95%), but
  out of sample (`--cut 2015`) the bands are already at or below the target (82.6 / 80.8 / 79.1%), and a narrower
  band makes the size rows 1-10B and 30-100B worse by more than 2 points. Examine again with a table made from a
  longer panel.
- **C, accepted:** the output gives the mean of the value and its standard error; the screen CSV has a `mean`
  column. The headline stays the P50: the mean is unstable for some tickers (median mean / P50 1.34 over 393
  tickers, P99 8.97; the top 1% of paths hold 58% of the mean of EXE). No default changed.
Open: the gate (model-validator, code-reviewer); merge after `T-20261120-dcf-conventions-decision`.
Candidate parts 1, 2, 4 and 5 of the list above are not started.

**Gate, 2026-11-20.** Head of Quality and Risk, review cycle 2 on commit c799006: PASS-WITH-NOTES
(`model-validator` PASS-WITH-NOTES, `code-reviewer` PASS-WITH-NOTES). The seven notes are done in the merge
commit with `main` (the conventions change). Pull request open.
