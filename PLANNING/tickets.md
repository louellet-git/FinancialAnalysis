---
id: DOC-TICKETS
type: plan
item: 10.16 Development plan
status: current
date: 2026-10-25
scope: Backlog not yet on the board, with its sources.
---
# Planning Tickets

> A ticket is done when its smoke test prints real output the next ticket consumes.

Open lines name their authoritative source. "Vision" = PLANNING/reusable-dcf-vision.md (✦n amendments, Ln layers, Pn phases). Work in progress: CLAUDE.md, Open work.

## OPEN TICKETS

### Queued work

- **DUOL driver build**: DUOL is consensus-anchored only, bands decorative (mechanical backtest 0/6: docs/ARCHITECTURE.md, Backtest harness; run record: `PLANNING/token-ledger.md` DUOL row); promote to a driver-model frontier run (needs either person's sign-off, vision ✦6a) or leave as screening output.
- **ORCL frontier run** (ticker #2), then basket and cross-sectional gap decomposition — board T-20261025-orcl-frontier-run; basket amendment (vision L6).
- **IBM Q2 grading** — board T-20261025-ibm-q2-grading (scope: 5-item watch list, first bet-ledger entries, 2026 mechanical vintage).

### Factory P1 (vision phasing — the active frontier)

- **Rulebook extraction**: promote the 9-item backlog into RULES.md rows with validators + specimens — PLANNING/RULES.md backlog.
- **`validators.py`**: machine-checkable rule subset, run at model build + CI; R1 coverage validator — PLANNING/RULES.md R1; vision L4.
- **Red-team corpus + injection harness** (✦2) — vision L4/P1.
- **Statement-coverage validator** (✦7), run retroactively on the IBM register — vision L4/P1.
- **Research Ledger formal schema** (✦5): expiry policies + schema-enforced provenance tiers (interim format in use) — analysis/IBM/research-ledger.md header; vision L1/P1.
- **RPO expiry** (overdue): consumed RPO facts expired at the Q2-2026 10-Q (late July 2026); append the Q2-2026 10-Q RPO row to analysis/IBM/research-ledger.md; report watch-list item 4.

### Engine tickets (deferred with dispositions — do not re-litigate, implement)

- **T-DCF-1**: AR(1) mean-reverting growth / correlated year-to-year sampling — until then P10/P90 are decorative — analysis/IBM/REVIEW.md §Deferred-fix tickets (from F6).
- **T-DCF-3**: derive terminal growth per path from faded segment draws — analysis/IBM/driver_model.py terminal_growth rationale; DRIVER-REVIEW.md.
- **Lognormal marginal for ma_ratio** (fat right tail) — analysis/IBM/DRIVER-REVIEW.md ma_ratio disposition.
- **Mixture marginal for g_consulting** (GenAI-cannibalization left skew; current normal is a proxy) — analysis/IBM/DRIVER-REVIEW.md g_consulting disposition.
- **Non-stationary E/V + Harris-Pringle relever** as debt grows — analysis/IBM/DRIVER-REVIEW.md equity_weight disposition.
- **T-IMP-1** (importer): drop phantom annual periods; align recast revenue with recast OCF across divestiture boundaries; persist prices to the JSON store — analysis/IBM/REVIEW.md §Deferred-fix tickets (from F13).

### Importer / pipeline

- **Ticket 7 — segment revenue extraction** from filing-level XBRL instance documents — full spec below in this file.
- **requirements.txt kept current** with actual needs on every dependency change — THOUGHTS, paragraph 1 (requirements.txt).
- **News-scan idea** (unscoped): scan ticker news for knock-on effects on statement items; shock the valuation — THOUGHTS, paragraph 2.

---

# OPEN TICKET SPECS

## Ticket 7 — Segment Revenue Extraction ⬜ OPEN
- **Title:** Extract per-segment revenue from filing-level XBRL (dimensional facts)
- **Finding:** SEC `companyfacts` strips dimensional facts (docs/HIDDEN-ASSUMPTIONS.md #5). `PeriodFinancials.segment_revenue` stays `None` from this source; reserved in `importer.py`.
- **Description:** Segment-tagged revenue (e.g. `us-gaap/RevenueFromContractWithCustomerExcludingAssessedTax`
  dimensioned by `us-gaap:StatementBusinessSegmentsAxis`) lives in the filing-level
  XBRL instance documents (R-files / `_htm.xml` under the filing accession) and in
  third-party APIs. Parse the instance document for the latest 10-K/10-Q, map
  segment member names to human-readable labels, and populate
  `PeriodFinancials.segment_revenue: Dict[str, float]`.
- **Acceptance criteria:**
  - `segment_revenue` populated for IBM FY2023-25 (Software / Consulting /
    Infrastructure / Financing / Other) and reconciles to total revenue ±1%
  - Member-label mapping documented; unmapped members carry the raw QName
  - No change to behavior when instance parsing fails: field stays `None`, warning logged
  - Tests with a synthetic instance-document fixture; no network
- **Smoke test:** `python -m scripts.import_financials IBM --refresh` prints per-segment
  revenue for FY2025 summing to ~$67.5B total revenue (FY2025 `SEGMENT_HISTORY` sum, analysis/IBM/driver_model.py)
