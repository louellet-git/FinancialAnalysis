---
name: valuation-report-review
description: Use to run the four-step review of a valuation report that a person will use for a decision: analyst, skeptic, reviewer, auditor. It gives the entry and exit condition of each step and the order. Use it for each new report and for a changed report.
---

# Valuation report review

This skill is the flow of `CLAUDE.md` ("Review of a valuation report"). A first pass
(for example `analysis/JNJ/FIRST-PASS.md`) did not go through it and is not used alone
for a trade. The orchestrator runs the flow. Each QA step starts in a fresh context.

| Step | Agent | Enter when | Exit when |
|---|---|---|---|
| 1. Write | `equity-analyst` | Ticker approved by Luc (a new ticker); data has no FAIL status; ticket `in-progress` | Model, ledger and report draft exist; each number has a command and a seed |
| 2. Attack | `model-validator` | Step 1 done | Verdict PASS or PASS-WITH-NOTES. On REVISE the analyst fixes and writes a disposition for each finding; the skeptic runs again (one more cycle at most, then the orchestrator decides) |
| 3. Grade | `document-reviewer` | Step 2 passed; the text will not change in content | Verdict PASS or PASS-WITH-NOTES for content, format and STE |
| 4. Audit | `auditor` | Steps 2 and 3 passed | Each claim is OK, or the DIFF is fixed; no NO SOURCE on a load-bearing number |
| Close (only on request, for example a conflict of interest) | `risk-compliance-officer` | Step 4 passed | Disclosures present (first pass or reviewed; model prices; conflict of interest for ANT); no secret |

After step 4 and the close, the orchestrator asks Luc to approve a report for a new
ticker (`PLANNING/reusable-dcf-vision.md`, L5). A change after a step sends the work back
to the step that the change touches.

## What the analyst hands over
The model file, the ledger, the commands with seeds, the list of weak points that the
analyst found, and the model document and assumption records (`docs/models/`,
`docs/assumptions/`).

## What each QA agent gets
The branch and the files. Not the conclusion of the author, and not the report of an
earlier QA step (the orchestrator compares them).

## Record
Write a short review record next to the report: findings, disposition, final verdict.
Use the form of `analysis/IBM/REVIEW.md` (do not change that file). A published report is
frozen (`CLAUDE.md`, rule 7): a new version is a new file.

## Check
- The steps ran in order. No step reviewed the work of its own context.
- Each CRIT and MAJ has a disposition.
- The orchestrator wrote the final decision.
