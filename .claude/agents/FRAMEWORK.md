# Vendored doc-review framework

Vendored from **github.com/louellet-git/doc-review-framework @ `d9d9075`** (the canonical
source, itself extracted from CLIENT-WORK @ `7376e5c`). Re-vendor from that repo to update.

| Here | Role | Origin |
|---|---|---|
| `memo-reviewer.md`, `memo-review-rubric.md` | .docx craft review loop (scores, never edits) | verbatim |
| `valuation-analyst.md` | the analysis agent — drafts the report | renamed/specified for this repo |
| `valuation-skeptic.md` | the reviewer agent — adversarial substance review | renamed/specified for this repo |
| `report-auditor.md` | FINAL audit gate — claim→source traceability + soften | adapted from `deck-auditor.md` |
| `../skills/officecli/SKILL.md`, `../../scripts/*.sh` | docx tooling + review-gate hooks | verbatim |

## Pipeline (every client-grade valuation report)

`valuation-analyst` drafts → back-and-forth with `valuation-skeptic` until sign-off →
report exists → **cleanup loop**: `memo-reviewer` grades content + format against the
rubric, analyst fixes, until PASS (target=good minimum) → **final audit**:
`report-auditor` traces every claim, re-derives every number, softens over-claims →
PASS required to ship.
