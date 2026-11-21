---
name: model-validation
description: Use to attack a model before a person uses it: reproduce its numbers, challenge each driver, compare with base rates, run the sensitivity, ask what the price implies, and give a verdict. This is the method of the model validator (the skeptic).
---

# Model validation

The method that found the errors in `analysis/IBM/REVIEW.md` (F1 to F19). Read that file
once to see the form of a finding.

## Steps
1. **Reproduce.** Run the author's command with the same seed, model file and cache. Write
   each number. A number that you cannot reproduce is a finding. Check that the IBM
   headline numbers still hold: `fa dcf drivers IBM` (seed 7) gives $83.84 / $151.84 /
   $258.03.
2. **Attack each driver.** For each driver ask: what is the source of the mean and the
   spread; does the distribution fit the history (a skew, a limit, a sawtooth); is the
   choice stochastic or fixed, and why; is the sign of each correlation right.
3. **Look for double counts and mismatches.** Debt charged twice (F1); a stock with its
   flow (F15); growth credited without the cost that bought it (F16); SBC added back and
   not charged (F5); a terminal value share that nobody reports (F11); a rule that is right for
   the median and wrong for the tail (count the values at or below 0 before and after).
4. **Base rates.** Compare each driver with the history of the company, the panel and the
   tables in `analysis/priors/`. A growth path that no company has done is a finding.
5. **Sensitivity.** Move each large driver (`--sensitivity NAME=V1,V2`, `--set`, `--pset`).
   Report the three drivers that move the P50 most.
6. **What the price implies.** Use `--implied-price P --implied-param NAME`. Read the
   result as a measurement of the gap. Never use it to change an input.
7. **Search for the omitted.** Forward disclosures (RPO, guidance), stale ledger facts,
   hand-entered numbers; for a backtest, a poison test of each data source it reads; for a
   fit, the convergence status and the parameters on a bound; for a rejected alternative in a
   record, a run of it.
8. **Verdict.** Give PASS, PASS-WITH-NOTES, REVISE or REJECT with each finding, its level
   and the fix that you ask for (`docs/QUALITY-STANDARDS.md`, section 3).

## The rule against fitting to the price
An input has a reason that does not use the price. If the only reason for a value is that
it moves the P50 towards the price, the finding is a CRIT.

## Check
- You reproduced each number yourself.
- Each finding has evidence (command or file and line).
- The sensitivity table and the base-rate comparison are in the report.
