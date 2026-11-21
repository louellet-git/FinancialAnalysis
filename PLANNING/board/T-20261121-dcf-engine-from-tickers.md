---
id: T-20261121-dcf-engine-from-tickers
title: Make the judgments of the ticker models into rules of the automatic model, one at a time
area: engine
status: backlog
owner: louellet-git
requested_by: louellet-git
head: fundamental
order: 0.3
branch:
created: 2026-11-21
acceptance: Each candidate below is a rule of `fa dcf auto` (or is refused with a reason), with a fixed-answer test, a record in `docs/assumptions/`, a row in `fa dcf assumptions`, and the measured effect of `fa dcf assumptions --effect` and `fa dcf pit --half` / `--cut`.
---
Owner word, 2026-11-21: "Ensure that work we do is repeatable and contributes to the dcf engine." A ticker model
(NVDA first) is the test bench: each judgment that the analyst or the skeptic finds there is checked as a general
rule of the automatic model, so that the next ticker gets it with no hand work.

Candidates from the NVDA work (`analysis/NVDA/`, review step 2 of 2026-11-21):
1. **Base revenue = the trailing four quarters** when the 10-Qs exist, not the last fiscal year (NVDA: $215.9B
   against $361.8B). Point-in-time rule: the four quarters filed on or before the model date. (Also candidate 2 of the
   done ticket `T-20261120-dcf-engine-accuracy`.)
2. **Shrink of the margin by size and history**: the screen shrinks NVDA's free cash flow margin (above 40% for three
   years) to 19.4%. Test a weight that grows with the number of years above the cell mean.
3. **Base rates of scale**: the fade of a company above $100B of revenue (the `[[fade]]` cell above $100B) as a guard of
   the growth path; the largest revenue and profit in the panel as a check of year 10.
4. **Purchases of equity securities as a cost** (a new standard variable from the cash flow statement, Head of
   Technology), paired like the acquisition charge (A-002): a purchase that finances a buyer buys credited growth.
5. **The second acquisition line** (`acquisitions_net_of_cash_second`, after the mapping merge; only when its value
   differs from the first line).
6. **Cash-like securities in the bridge** (`DebtSecuritiesCurrent`, after the mapping merge) and equity stakes as a
   decision with a reason.
7. **Company guidance** as evidence for the growth of year 1 (rule R1). Needs the guidance as data (8-K text): a
   request to the Head of Technology; the first step is a study of how often the guide was beaten.

Rule: one candidate at a time; each has a reason that does not use the price; the gate is the model-change flow.
