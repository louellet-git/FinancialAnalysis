# RULES.md — canonical rulebook (L4 of the valuation factory)

Rules won in adversarial review, each with provenance and scope (✦3: universal /
archetype-bound / ticker-bound). Every rule generates a validator (where machine-
expressible) and a red-team specimen (✦2). Deprecation requires a review fan-out of the
same weight that created the rule, recorded here.

| # | Rule | Scope | Provenance | Validator | Specimen |
|---|---|---|---|---|---|
| R1 | **Forward-looking disclosures bearing on sampled drivers must be consumed or excluded with reason.** RPO/backlog, guidance, and order books are evidence about the growth drivers; ignoring them silently is the C1 failure mode. | universal | IBM coverage flag C1 (SOURCE-AUDIT.md, 2026-07-17): $71.0B RPO (105% of revenue) unused and unmentioned in v4.0 | P1: coverage validator classifies RPO-class concepts; unclassified = fail | P1: known-material RPO omitted from a seeded model |
| R2 | **Free cash flow is operating cash flow less `capex_total`. `capex_total` is equipment (`capex`) and capitalized software. Purchased intangibles are not included.** Reason for software: a company can record the same engineering cost as an expense (operating cash flow goes down) or as an asset (it does not). With equipment only, the second company shows a higher free cash flow for the same business. Reason against intangibles: for some companies they are a regular cost (spectrum licenses) and for others they are like an acquisition (a drug right); acquisitions are treated separately (IBM review, M&A recurrence `f`). The equipment number and the intangibles number stay available as `capex` and `purchases_of_intangibles`. Do not add options to this definition; change it only with a new decision recorded here. | universal (classic engine, backtest, reports; a driver model with its own reviewed cash-flow definition keeps it) | Owner decision (Luc), 2026-11-19, after the comparison with Yahoo Finance: for 9 of 69 companies the equipment-only capex was lower than the independent source. Effect on 2026-11-19: 14 of 324 companies change, median free cash flow −5.2%, largest −16% (DASH); no ticker of the first panel changes (IBM uses a company tag for software, which the standard source does not have) | `tests/test_importer.py` (capex_total tests); `mapcheck` finding `capex_software_overlap` (a capex tag for "productive assets" can already include software: 3 of 472 companies) | Open: a seeded model that adds purchased intangibles to capex |

Backlog for P1 rulebook extraction (from analysis/IBM/REVIEW.md and DRIVER-REVIEW.md):
endpoint-not-flattering-mean; growth-credited-must-match-cost-charged (pairing); bridge
stock-or-flow-never-both; cyclical drivers get decade-CAGR dispersion; every bridge item
income-stripped-or-absent; kd must co-move with sampled rf; single-source pairing
constants; quantitative immateriality thresholds; reference-resolving exclusions.
