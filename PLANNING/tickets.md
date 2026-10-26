---
id: DOC-TICKETS
type: plan
item: 10.16 Development plan
status: current
date: 2026-10-25
scope: Backlog not yet ticketed, with its sources.
---
# Planning Tickets

> A ticket is done when its smoke test prints real output the next ticket consumes.

Open lines name their authoritative source. "Vision" = PLANNING/reusable-dcf-vision.md (✦n amendments, Ln layers, Pn phases). In progress: see CLAUDE.md, Open work.

## OPEN TICKETS

### Queued work

- **DUOL driver build**: DUOL is consensus-anchored only; see docs/ARCHITECTURE.md §Backtest harness and the PLANNING/token-ledger.md DUOL row; promote to a driver-model frontier run (needs either person's sign-off, vision ✦6a) or leave as screening output.
- On the board, not repeated here: T-20261025-orcl-frontier-run (basket amendment: vision L6), T-20261025-ibm-q2-grading (bet-ledger: vision L6), T-20261025-p1-validators-redteam (vision P1).

### Factory P1 (vision phasing — the active frontier)

- **Rulebook extraction**: promote the 9-item backlog into RULES.md rows with validators + specimens — PLANNING/RULES.md backlog.
- **Research Ledger formal schema** (✦5): expiry policies + schema-enforced provenance tiers (interim format in use) — analysis/IBM/research-ledger.md header; vision L1/P1.
- **RPO expiry** (overdue): consumed RPO facts expired at the Q2-2026 10-Q (late July 2026); record the Q2-2026 10-Q RPO facts in a new ledger item that supersedes analysis/IBM/research-ledger.md (expiry: its Expiry line; report watch-list item 4).

### Engine tickets (deferred with dispositions — do not re-litigate, implement)

- **T-DCF-1** (from F6): AR(1) mean-reverting growth, and single-factor correlated sampling across (g, tg, margin) for the classic engine — analysis/IBM/REVIEW.md §Deferred-fix tickets; driver engine: DRIVER-REVIEW.md.
- **T-DCF-2** (classic engine, from F12): truncated/lognormal WACC and tg draws, enforce wacc > tg + 2% jointly; remove or winsorize DCFResult mean/std — analysis/IBM/REVIEW.md §Deferred-fix tickets.
- **T-DCF-3**: derive terminal growth per path from faded segment draws — analysis/IBM/driver_model.py terminal_growth rationale; DRIVER-REVIEW.md.
- **Lognormal marginal for ma_ratio** (fat right tail) — analysis/IBM/DRIVER-REVIEW.md ma_ratio disposition.
- **Mixture marginal for g_consulting** (GenAI-cannibalization left skew; current normal is a proxy) — analysis/IBM/DRIVER-REVIEW.md g_consulting disposition.
- **Non-stationary E/V + Harris-Pringle relever** as debt grows — analysis/IBM/DRIVER-REVIEW.md equity_weight disposition.

### Importer / pipeline

- **T-IMP-1** (importer): drop phantom annual periods; align recast revenue with recast OCF across divestiture boundaries; persist prices to the JSON store; populate dividends/buybacks/interest income as first-class fields; stop the filed_date = period_end placeholder on rows without revenue (board T-20261025-filed-date-fabrication; close when T-IMP-1 lands) — analysis/IBM/REVIEW.md §Deferred-fix tickets (from F13).
- **Ticket 7 — segment revenue extraction** from filing-level XBRL instance documents — full spec below in this file.
- **requirements.txt kept current** with actual needs on every dependency change — THOUGHTS, paragraph 1 (requirements.txt).
- **News-scan idea** (unscoped): scan ticker news for knock-on effects on statement items; shock the valuation — THOUGHTS, paragraph 2.

---

# OPEN TICKET SPECS

## Ticket 7 — Segment Revenue Extraction ⬜ OPEN
- **Title:** Extract per-segment revenue from filing-level XBRL (dimensional facts)
- **Finding:** see docs/HIDDEN-ASSUMPTIONS.md #5.
- **Description:** Segment-tagged revenue (e.g. `us-gaap/RevenueFromContractWithCustomerExcludingAssessedTax`
  dimensioned by `us-gaap:StatementBusinessSegmentsAxis`) lives in the filing-level
  XBRL instance documents (R-files / `_htm.xml` under the filing accession) and in
  third-party APIs. Parse the instance document for the latest 10-K/10-Q, map
  segment member names to human-readable labels, and populate
  `PeriodFinancials.segment_revenue` (`Optional[Dict[str, float]]`, default None; schema.py:156).
- **Acceptance criteria:**
  - `segment_revenue` populated for IBM FY2023-25 (Software / Consulting /
    Infrastructure / Financing; Other if the filing reports it) and reconciles to total revenue ±1%
  - Member-label mapping documented; unmapped members carry the raw QName
  - No change to behavior when instance parsing fails: field stays `None`, warning logged
  - Tests with a synthetic instance-document fixture; no network
- **Smoke test:** `python -m scripts.import_financials IBM --refresh` prints per-segment
  revenue for FY2025 summing to ~$67.5B total revenue (FY2025 `SEGMENT_HISTORY` sum, analysis/IBM/driver_model.py)
