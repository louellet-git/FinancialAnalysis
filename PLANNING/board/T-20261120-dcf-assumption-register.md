---
id: T-20261120-dcf-assumption-register
title: Tie each assumption of the automatic model to a record and a repeatable study
area: engine
status: review
owner: louellet-git
head: fundamental
order: 1.1
branch: claude/dcf-assumption-register
created: 2026-11-20
acceptance: `fa dcf assumptions` lists each assumption of the automatic model with its value, its record and its decision state, a test fails when a default of the code and its record disagree, and each assumption has a record.
---
Owner request, 2026-11-20: the choice of an assumption is studied, understood and written in the
assumption records, "in a way that is easy, repeatable, understandable". The owner does not answer each
question. This ticket comes before order 2 (owner request of the same date).

Parts, one at a time:
1. A table in the code (`rust/src/dcf/`) with one row for each assumption of the automatic model: record
   number, name, the `--rules` name, the default, the decision state (`decided` with a date, or `convention,
   not studied`).
2. `fa dcf assumptions`: prints the table. `fa dcf auto T --explain` names the record of each number.
3. A test: each row has a record file in `docs/assumptions/`, and the record has the default value.
4. The records that are not written (`docs/models/auto-path-model.md`, rows "not written"). The equity
   risk premium of 4.41% has no source that we can find (`analysis/priors/dcf_conventions_study_2026-11-20.md`,
   section 1.3): give it a source and a date, or change it in a study.
5. The procedure of an assumption study as a skill (question, alternatives as `--rules`, evidence that does
   not use the price, measured effect, decision, record), and one command that makes the effect table of
   the alternatives again (the tables of the two studies came from scratch programs that are not in the repo).

Open conventions that the register must list for a study: flows in the middle of the year (`mid_year`),
the equity risk premium, the 10-year horizon.

Part 4 touches the ticket `T-20261120-model-docs-backlog` (head: quality), item 1. Tell that head.

**Gate, 2026-11-20.** Head of Quality and Risk, review cycle 1 on commit d55daf2: PASS-WITH-NOTES
(`code-reviewer` and `document-reviewer` PASS-WITH-NOTES). The seven notes are done in the merge commit with
`main`. Pull request open. The five records (premium, rf, horizon, beta, SBC) stay with the Head of Quality and
Risk (`T-20261120-model-docs-backlog`).
