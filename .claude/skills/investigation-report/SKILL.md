---
name: investigation-report
description: Use to write the findings file of an investigation (a question about a model, a data set or a strategy, with evidence): question, method, evidence table, defects with tests, conventions against defects, what stays open. The form follows analysis/priors/dcf_investigation_2026-11-20.md.
---

# Investigation report

Read `analysis/priors/dcf_investigation_2026-11-20.md` once. It is the model of the
form. Put the file in `analysis/priors/<name>_<date>.md`. Keep it short.

## Sections
1. **Header.** The ticket, one paragraph on what the file gives, and the rule of the
   ticket (for a model: the price measures and ranks; no input is set from the price).
2. **Question.** One question that a person can answer with yes, no, or a number.
3. **Data and commands.** The cache date, the as-of date, the seed, the sample size, the
   exact commands. A reader can run them again.
4. **Evidence table.** One row for each measure: what was changed, the alternative, the
   effect (with the base), and a judgment: "defect", "convention", or "open".
5. **Defects.** Each defect has: the claim, the evidence, the correction, and a test
   that fails before the correction and passes after. A correction has a reason that
   does not use the price.
6. **Conventions.** Choices that are not defects. Each one points to its assumption
   record (`docs/assumptions/`) or says that the owner decides.
7. **What stays open.** Each open point has a name and who decides.
8. **Limits.** Survivorship, sample, data age, what the test cannot show.

## Rules
- Measure one thing at a time. The effects of several changes do not add: say so.
- Do not select a rule on the set that scores it.
- Write what you did not do.
- STE (skill `ste-writing`). Numbers have units and dates (skill `number-audit`).

## Check
- A reader can run each command.
- Each defect has a test.
- Open points are listed with an owner.
- The auditor recalculated the numbers before the file is used.
