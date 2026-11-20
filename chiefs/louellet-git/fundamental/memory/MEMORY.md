# Dashboard: fundamental (louellet-git)
For the next session of this head: the judgments about value that are not in the code, and what the
person decided. Keep this file below 60 lines. Started 2026-11-20.

## Goal now and subsequent steps
The person wants (2026-11-20, `RECORD.md`): an engine with assumptions that are studied, decided and
written in the assumption records, and a fixed procedure that shows what the market price implies for
each stock, industry and sector. The person does not choose each assumption: this head decides.
Work started again on 2026-11-20 21:19 UTC ("Proceed with your work"). Each ticket file has its state.
1. `T-20261120-dcf-conventions-decision` (branch `claude/magical-planck-41ip3i`): built and tested; the
   full runs on the data store run now; then the gate (Head of Quality and Risk), then the pull request.
2. `T-20261120-dcf-engine-accuracy` (branch `claude/dcf-engine-accuracy`): A recovery after a decline
   year, B bands too wide for a stable margin, C the mean of the value with its standard error.
3. `T-20261120-dcf-market-implied` (branch `claude/dcf-market-implied`): parts 1 to 3 built; waits for
   the SIC codes, the findings file and the gate. Merge the conventions branch into it first.
4. `T-20261120-dcf-assumption-register` (not started): build it after the conventions merge.

## Decisions of this head (the reasons are in the records; no reason uses the price)
- 2026-11-20, long-run growth: one number `L` = lower of the 10-year zero yield and `G`. `G` = FOMC
  longer-run real growth + longer-run PCE inflation (SEP of 2026-09-16: 2.0% + 2.0% = 4.0%). Why FOMC
  and not CBO (3.8%): we can read the page, and ALFRED keeps each vintage for a point-in-time test.
- 2026-11-20, acquisitions: charge `w` x mean spend (`acq_w`), `w` = weight of the history in the growth
  mean, and the charge fades to zero along the same ramp as the credited growth above L (review cycle 1,
  F1: a constant charge made ROP -$91). Lesson: a paired rule must stay paired in EACH year, not only in
  year 1. Heavy buyers are still over-charged a little (point-in-time T+3 +2.4% of revenue).
- 2026-11-20, growth after a decline year: floor of 0% on the base-year growth in the company mean
  (A-003). A recovery term was refused: the rest of the error is one episode (base years 2019, 2020).
- 2026-11-20, margin bands: not narrowed. In sample they look too wide (95%), out of sample (`--cut
  2015`) they are right. Lesson: check an in-sample band error out of sample before a correction.
- 2026-11-20, market-implied values use the value at the means, not the P50: a risk-adjusted DCF values
  EXPECTED cash flows, so a price compares with an expected value. The P50 is lower by construction
  (IBM $159.72 at the means against P50 $132.87). So the screen ratio P50 / price is conservative by
  construction: a part of the gap to the market is the statistic, not the model.
- A market-implied value is a measurement, never an input.

## Standing words of the person
- 2026-11-20: the immediate goal is to improve the stochastic DCF engine, for stocks only ("stick to
  underlyings"); no option work in this area.
- 2026-11-20: "the agents should not be overly sophisticated just overall lean and of quality, not
  obsessive and tight fisted".
- 2026-11-19 or 2026-11-20, on the GitHub Actions use: "this is too much usage". Effect: few pull
  requests. A pull request with documents only has no check run.
- The computer of the person is a Windows laptop. Give commands for PowerShell.

## Tested rules
- The cache is the data store: `git clone --depth 1 https://github.com/louellet-git/fa-data data/raw`
  (public, read with no attach; 28 s). Do not start a bulk download (`chiefs/ORIENTATION.md`, section 2).
  Check after a clone: `fa dcf auto IBM --as-of 2026-11-19 --rules long_run=off,acq_w=0` gives P50 $132.87.
- The quant agent can build and test while data is missing; send the data runs as a second message.
- Parallel agents: one worktree each (`../wt-<name>`), the cache path with `--cache`, read only.
- cbo.gov refuses a request from the cloud machine (HTTP 403). federalreserve.gov answers.

## Pages
- No page.

## Dream pass
On the word of the person ("clean up memory" or "dream"). Last pass: none.
