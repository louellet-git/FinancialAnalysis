# Backlog

This file contains work that does not have a board ticket.
Work that a person does now, or does next, is on the board: `PLANNING/board/`.
When you start an item from this file, make a board ticket and delete the item here.

Rule: a ticket is complete only after a smoke test. Run the real command, print the
real output, and make sure that the next step can use the output.

Closed tickets 1 to 6: `PLANNING/tickets-closed.md`.

## Factory P1 (source: `PLANNING/reusable-dcf-vision.md`, `PLANNING/RULES.md`)

- **Rulebook extraction.** Move the 9 backlog items in `PLANNING/RULES.md` into rules.
  Each rule gets a validator and a specimen.
- **Research ledger schema.** Give the ledger a fixed schema with expiry dates and
  source tiers. The interim format is in `analysis/IBM/research-ledger.md`.
- **RPO expiry.** The RPO facts in `analysis/IBM/research-ledger.md` expired at the
  Q2-2026 10-Q. Replace them when IBM is graded (board ticket `ibm-q2-grading`).

The validators, the red-team corpus and the coverage validator are on the board
(`p1-validators-redteam`).

## Engine (decisions are recorded; do not open them again, implement them)

- **T-DCF-1.** Mean-reverting growth, or correlated year-to-year samples. Until this is
  done, P10 and P90 are too narrow. Source: `analysis/IBM/REVIEW.md` (F6).
- **T-DCF-3.** Calculate terminal growth for each path from the faded segment draws.
  Source: `analysis/IBM/driver_model.py`, `analysis/IBM/DRIVER-REVIEW.md`.
- **Lognormal `ma_ratio`.** Source: `analysis/IBM/DRIVER-REVIEW.md`.
- **Mixture distribution for `g_consulting`.** Source: `analysis/IBM/DRIVER-REVIEW.md`.
- **Non-stationary E/V** with a Harris-Pringle relever as debt increases.
  Source: `analysis/IBM/DRIVER-REVIEW.md`.

## Importer

- **T-IMP-1.** Align recast revenue with recast operating cash flow across divestitures
  (IBM FY2019-20, JNJ FY2022-23). Save prices in the JSON store. (The phantom annual
  periods are removed, 2026-11-19.) Source: `analysis/IBM/REVIEW.md` (F13), `analysis/JNJ/FIRST-PASS.md` (defect 4).
- **Ticket 7, segment revenue.** The full specification is at the end of this file.
  Extend it to all dimensional facts and company-specific tags: this is the next step
  to get the most granular inputs (the companyfacts source has line items only).
- **Successor registrants.** A ticker that moved to a new registrant (XOM) has no
  history. Join the facts of the old and the new CIK.
- **Fourth-quarter rows.** The quarterly rows are Q1 to Q3. Calculate Q4 as the fiscal
  year less the 9-month value.
- **Statement templates for banks, insurers and REITs.** The audit only marks them.
- **Foreign filers.** A filer with annual reports on form 20-F or 40-F has no annual
  rows (51 of the 500 largest registrants). Accept these forms as annual forms.
- **Contact in the SEC User-Agent. Done 2026-11-20.** The contact is the project address in `sec_contact.toml`; the
  environment variable `SEC_USER_AGENT` replaces it (`docs/HIDDEN-ASSUMPTIONS.md` #285).
- **Sign of expense tags.** Some filers report an expense as a negative number (DIS
  interest expense). Add a registry option that stores the absolute value.

## Templates (source: `analysis/JNJ/FIRST-PASS.md`)

- **Pharma M&A rule.** Acquired pipeline revenue becomes "organic" after one year. The
  IBM pairing possibly undercounts what M&A buys. A sector rule is necessary.
- **Beta method rule.** Regression betas for JNJ are 0.22 and approximately 0; the model
  uses 0.65 by judgment. A method rule is necessary.

## Ideas without a scope (source: `THOUGHTS`)

- **News scan.** Read the news for a ticker, find the effect on statement items, and
  shock the valuation and the position.

## Data sources (small corrections)

- **Alpaca and Tiingo after the first real tests (2026-11-20).** Index options (SPX, VIX) with a level and an implied volatility, weekly roots, a practical default selection of `fa options history`, Alpaca last in the order of the daily price sources, and the `divCash` fault of the Tiingo check (#280 to #284 of `docs/HIDDEN-ASSUMPTIONS.md`). Open: the FRED API has not run; NDX and RUT have no level proxy.

## Standard duties for each run

- Add a row to `PLANNING/token-ledger.md`.
- A charter review must accept a report before publication. Luc approves frontier runs
  (`PLANNING/reusable-dcf-vision.md`, L5).

---

## Ticket 7 — Segment Revenue Extraction ⬜ OPEN
- **Title:** Extract per-segment revenue from filing-level XBRL (dimensional facts)
- **Opened:** 2026-07-17 (P1 importer-completeness)
- **Finding that motivates it:** The SEC `companyfacts` JSON API strips dimensional
  facts — every entry carries exactly `{accn, end, filed, form, fp, frame, fy, start,
  val}`, no segment axis/member metadata (verified across all us-gaap concepts in
  IBM's cached payload). `PeriodFinancials.segment_revenue` therefore stays `None`
  from this source; the field is reserved and documented in `importer.py`.
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
  revenue for FY2025 summing to ~$65B total revenue
