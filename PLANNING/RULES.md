---
id: DOC-RULES
type: policy
item: 10.45 Quality management policy
status: current
date: 2026-10-25
scope: Canonical rulebook (L4) of the valuation factory; rules bind at their declared scope.
---
# Rulebook (L4)

L4, ✦2, ✦3 and the fan-out: `PLANNING/reusable-dcf-vision.md` §L4, §L5. P1 (validators, red-team
corpus): `PLANNING/board/T-20261025-p1-validators-redteam.md`.

Objective: rules won in adversarial review, each with provenance and scope (✦3:
universal / archetype-bound / ticker-bound), so no run re-pays them. Every rule
generates a validator (where machine-expressible) and a red-team specimen (✦2); these
and the per-run charter review enforce it. Deprecation requires a review fan-out of the
same weight that created the rule, recorded here.

| # | Rule | Scope | Provenance | Validator | Specimen |
|---|---|---|---|---|---|
| R1 | **Forward-looking disclosures bearing on sampled drivers must be consumed or excluded with reason.** RPO/backlog, guidance, and order books are evidence about the growth drivers; ignoring them silently is the C1 failure mode. | universal | IBM coverage flag C1 (analysis/IBM/report/SOURCE-AUDIT.md, 2026-07-17): $71.0B RPO (105% of revenue) unused and unmentioned in v4.0 | P1: coverage validator classifies RPO-class concepts; unclassified = fail | P1: known-material RPO omitted from a seeded model |

Backlog for P1 rulebook extraction (from analysis/IBM/REVIEW.md and analysis/IBM/DRIVER-REVIEW.md):
endpoint-not-flattering-mean; growth-credited-must-match-cost-charged (pairing); bridge
stock-or-flow-never-both; cyclical drivers get decade-CAGR dispersion; every bridge item
income-stripped-or-absent; kd must co-move with sampled rf; single-source pairing
constants; quantitative immateriality thresholds; reference-resolving exclusions.
