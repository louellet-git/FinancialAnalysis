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

## Notes of the NVDA analyst, 2026-11-21

Each line is a judgment of the NVDA model (`analysis/NVDA/`) that could be a rule of `fa dcf auto`. Ticket `T-20261121-dcf-engine-from-tickers` has the first list (items 1 to 7 below are its items). I agree or not, with the evidence. The gate for each rule is the model-change flow (a test, a record, a measured effect).

1. **Base revenue = trailing four quarters** (ticket 1). Agree. Evidence: the screen has year-1 revenue $257.8B, below the $361.8B already reported. Needs a point-in-time rule (quarters filed on or before the model date) and a fiscal-calendar rule: the Q4 of NVDA has 14 weeks, so the length of each period in days must be an input (candidate 8).
2. **Margin shrink by size and history** (ticket 2). Agree with a limit. Evidence: the screen gives 19.4% against a reported 41.1% (above 40% for three years). But MU shows a margin of 74.6% in one year after -37% three years earlier, so the weight must grow with the number of years above the cell mean, not with the level alone. A test on the point-in-time panel is needed.
3. **Base rates of scale** (ticket 3). Agree as a check, not as a cap. Evidence: the cell above $100B has 11 company-years, two thirds of them oil in 2021-22, and holds busts; the model's year 2 and year 3 growth is far above it. Use the cells above $30B pooled, and print the largest revenue and profit of the panel next to year 10 (here 1.5 times and 3.2 times). A warning flag is enough. A rule must also print the composition of the cell.
4. **Purchases of equity securities, one number r** (tickets 4 and 6). Agree, with the new frame. Evidence: $70.9B in four quarters, 19.6% of revenue. The rule is one r for the stakes in the bridge and for the charge, so that no choice is half a frame (the version 3 choice was); r = 0.5 as the default when there are no terms, with the range shown. The effect is $10.7 of P50 over r = 0 to 1. The marketable stakes stay in at full value (marks, gains); r applies to the non-marketable stakes and the purchases. Needs standard variables for purchases and proceeds of equity securities and for the balance sheet lines (a new tag set). For most companies the value is near zero, so the rule is safe.
5. **The second acquisition line** (ticket 5). Agree. Evidence: $13.0B in fiscal 2026 under a second tag is not in `mapcheck values`. A limit: the "Groq, Inc." line is in the financing section under a company label. A rule cannot find it. Say so in the output (a warning when the acquisition spend of the year is far above its mean).
6. **Cash-like securities in the bridge** (ticket 6). Agree. Evidence: `DebtSecuritiesCurrent` hides $34.1B ($1.41 a share) in the screen. Equity stakes: see line 4.
7. **Company guidance as evidence for year 1** (ticket 7). Agree, and the first step is done here, with a warning. Evidence: the 8-K press releases of 21 quarters are in the store; 19 pairs of guide and result give 18 beats and 1 miss (-17.2%, pre-announced), median +5.57%. A shorter window (the 12 pairs of the AI upswing) gave a median of +5.65% and no miss: the window changes the rule. A rule can read the sentence "Revenue is expected to be $X billion, plus or minus Y%" (a shell pipeline does it, `analysis/NVDA/COMMANDS.md`, section 3; note the old releases say "percent") and use the beat history of the company as the correction, with this form: the guide unless the median of all pairs is above zero and at least 79% (15 of 19) are beats, then half the median. The effect here is +$5.2 of P50 (103.78 less 98.60, 100,000 paths, seed 7). Limit: one company, one upswing and one bust; a preliminary release (2022-08-08) must not be used as a result. A study over all companies with an outlook sentence is needed before a rule.
8. **Period length in days.** New. Evidence: Q4 FY27 has 14 weeks ($130.0B is $120.7B in 13 weeks). The screen has no input for the length of the period.
9. **SBC ratio from the trailing four quarters when growth is high.** New. Evidence: the screen uses 4.14% (mean of three years); the trailing ratio is 2.12% and the last quarter 1.80%.
10. **A collapse regime for companies with a bust in their history, and an engine that impairs the bridge on a failed path.** New, and a study first. Evidence: MU, revenue -49.5% in FY23. The hazard and the multiple here have no source; the multiple sets the lower tail (floor $30). The engine subtracts the net debt on each path, so a collapse keeps the stakes and ignores the guarantees. The panel can measure the share of company-years with revenue -30% or worse after a year of growth of 50% or more. Without that measure the rule is not ready.
11. **The charge fades with the growth** (record A-002). Agree with the decision of the Head. Evidence: the ou fade with the half-life of the growth has a mean of 58% in years 1 to 3, close to the panel factor 0.55 (study 2.1); a ramp of 10 years is $5.5 of P50 lower. The engine cannot share one half-life between two processes: a candidate feature (a process that takes the half-life of another).
12. **Equity weight from the price.** The screen takes the equity weight from the price (0.991). This model fixes it at 1.0 so that no input comes from the price. The effect is small (under 0.1 point of cost of capital), but the rule "no input from the price" is cleaner. Disagree with the screen here; low priority.
13. **A beta that is a draw.** This model draws the beta with its standard error (0.088); the screen and IBM fix it. Evidence: a beta of 1.2 moves the P50 by $28. A draw widens the bands by little. Not a priority, but a test of the band coverage of the point-in-time test would show it.
