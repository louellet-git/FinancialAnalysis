# Planning Tickets

> Each ticket must end with a mandatory smoke test before it is closed.
> A ticket is not done until real output is printed and verified consumable by the next ticket.

---

# CURRENT OPEN TICKETS (consolidated 2026-07-17)

One list of everything open, with the source that created each item. Detailed specs live
at the cited sources; Ticket 7 has its full spec at the bottom of this file. The
completed pipeline build-out (Tickets 1–6 + 1.5/3.5/4.5/4.6) follows below as
**historical record — closed, preserved verbatim**.

## Engine (driver_dcf.py)

- **T-DCF-1 — AR(1)/mean-reverting growth** *(source: REVIEW.md F6 ticket; DRIVER-REVIEW
  deferred list)*. One draw held constant per path is the single largest known
  distortion (both the infrastructure and correlation reviewers converged on it;
  reviewer experiment: φ=0.6 reverting to 2.5% cut base P50 ~10%, P90 ~16%). Includes
  the single-factor correlation structure across (g, tg, margin) from the original F6
  spec. Until it lands, P10/P90 are decorative — no margin-of-safety claims.
- **T-DCF-3 — terminal growth derived per path** from faded, mix-weighted segment draws
  *(source: DRIVER-REVIEW terminal_growth disposition + TV consolidation: the current
  tg anchoring is correlational, not structural)*.
- **Lognormal / mixture marginals** — g_consulting left-skew mixture (GenAI
  cannibalization), ma_ratio fat right tail *(source: DRIVER-REVIEW g_consulting and
  ma_ratio dispositions; deferred list)*.
- **Harris-Pringle re-levered β + target capital-structure weights**; non-stationary
  E/V as debt grows *(source: DRIVER-REVIEW equity_weight / beta dispositions)*.
- **Correlation upgrades** — replace pairwise judgment entries with a single-factor
  model where defensible; estimate the rf·erp entry from market history instead of
  judgment *(source: REVIEW.md T-DCF-1 spec; DRIVERS.md matrix note — every entry is
  currently a reviewed judgment, none estimated)*.

## Importer / data (T-IMP-1 family)

- **T-IMP-1 — importer fidelity** *(source: REVIEW.md F13 ticket; now evidence-backed by
  the backtest build)*:
  - **filed_date fidelity** — `filed_date` is currently the filing date of the
    latest-filed (often comparative-column) source, lagging real filings by up to ~2
    years, and falls back to a period_end placeholder where no revenue fact matched
    (verified in IBM.json; see docs/HIDDEN-ASSUMPTIONS.md #3–#4). The backtest treats
    this as conservative-by-construction, but true first-filed dates would sharpen
    vintages.
  - **Persist prices into the JSON store** (currently cache/xlsx only).
  - Drop phantom annual rows (2012-06-30, 2018-01-01, 2019-01-01 pattern); align recast
    revenue with recast OCF across divestiture boundaries.
  - ~~dividends_paid as first-class field~~ ✅ DONE (commit `e2c711a`, with
    interest_income and RPO; SOURCE-AUDIT flag C5 closed). Buybacks post-2019: verified
    genuinely absent, nothing to import.
- **Ticket 7 — segment revenue from filing-level XBRL** *(spec below; opened 2026-07-17;
  companyfacts has no dimensional facts — verified)*.

## Factory P1 remainder (source: reusable-dcf-vision.md phasing; RULES.md backlog)

- **validators.py** — machine-checkable subset of RULES.md, run at model build + CI
  (R1's RPO-class coverage check is the first).
- **Red-team corpus + injection harness (✦2)** — known-bad models seeded with past
  accepted findings (F2 debt double-count, F16 cost-without-growth, stale parameters,
  negative credit spreads); every accepted finding generates a validator AND a specimen.
- **Statement-coverage validator (✦7)** — every statement item classified used/excluded
  with checkable reason codes; run retroactively on the IBM register (SOURCE-AUDIT
  flags C1–C5 are the seed backlog).
- **Charter-review wiring** — reviewer must ACCEPT the rules+assumptions package on
  every run before publication.
- **Rulebook extraction** — promote the DRIVER-REVIEW/REVIEW lessons listed in RULES.md
  backlog into numbered rules with validators.

## Report pipeline

- **Deterministic report-generator templating** *(source: token-ledger.md — the analyst
  share (~0.77M tokens) of the report pipeline must be cut ~80% by templating the
  deterministic parts; vision L7)*.

---

# HISTORICAL RECORD — Tickets 1–6 (all CLOSED; preserved verbatim)

## Ticket 1 — EDGAR Import ✅ CLOSED
- **Title:** Verify SEC EDGAR filing retrieval end-to-end
- **Status:** Closed 2026-06-03. Smoke test passed.
- **Results:**
  - 217 files downloaded for PANW under `data/raw/PANW/`
  - CIK confirmed as `1327567`
  - `0001327567-23-000024.txt` opens with `<SEC-DOCUMENT>` and is a real `10-K` ✓
- **Finding:** The bulk of downloaded files are form type `144` (insider stock sale notices), not financial filings. The importer does not filter by form type — it downloads everything matching the CIK in the date window. This is expected behaviour; form-type cataloguing is Ticket 1.5's responsibility.

---

## Ticket 1.5 — Data Inventory ✅ CLOSED
- **Title:** Map downloaded files and confirm XBRL API path
- **Status:** Closed 2026-06-03. All acceptance criteria met.
- **Findings:**
  - 217 files downloaded; only 2 × 10-K and 3 × 10-Q are financial filings. The remaining 212 are form 4 (insider ownership), 144 (proposed insider sales), 8-K, and misc. Raw files are not a viable parse path for financial analysis.
  - XBRL API confirmed live: `data.sec.gov/api/xbrl/companyfacts/CIK0001327567.json` returns 507 `us-gaap` concepts for PANW.
  - Revenue concept: `us-gaap/RevenueFromContractWithCustomerExcludingAssessedTax` (not the generic `Revenues`).
  - FY2023: $6.89B · FY2024: $8.03B · FY2025: $9.22B ✓
  - **Watch out:** The API returns duplicate entries for the same period end date (observed for 2024-07-31). `extract_annual_series` in `xbrl_api.py` must deduplicate by `end` date, keeping the entry with the latest `filed` date.
  - Decision confirmed in `context.json`: XBRL API = primary path, raw `.txt` files = archival fallback.

---

## Ticket 2 — XBRL API Client ✅ CLOSED
- **Title:** Build SEC XBRL JSON API client
- **Description:** Implement `financial_analysis/xbrl_api.py` that fetches company facts from `data.sec.gov/api/xbrl/companyfacts/` and extracts time-series financial line items by concept name and form type.
- **Acceptance criteria:**
  - `fetch_company_facts(cik: str) -> dict` — fetches raw company facts JSON, reuses rate limiting and User-Agent from `edgar.py`
  - `extract_annual_series(facts: dict, concept: str) -> list[dict]` — returns 10-K entries for a concept, sorted by `end` date ascending
  - `extract_quarterly_series(facts: dict, concept: str) -> list[dict]` — returns 10-Q entries
  - Missing concept returns `[]`, no exception raised
  - Deduplicates entries with the same `end` date, keeping the one with the latest `filed` date (observed in live PANW data)
  - Unit tests mock HTTP; no live calls in test suite
- **Smoke test:** Fetch PANW facts live, extract `us-gaap/RevenueFromContractWithCustomerExcludingAssessedTax` for last 5 annual periods, confirm no duplicate `end` dates, print `end` date + value

---

## Ticket 3 — Financial Schema ✅ CLOSED
- **Title:** Define time-series financial schema
- **Description:** Create `financial_analysis/schema.py` with dataclasses for multi-period financial data. Schema must be time-series-first (a list of periods per company) because Monte Carlo simulation requires 5–10 years of history to fit input distributions.
- **Acceptance criteria:**
  - `PeriodFinancials` dataclass: one reporting period. All financial fields `Optional[float]`.
  - `CompanyFinancials` dataclass: holds `List[PeriodFinancials]` sorted by `period_end` ascending.
  - Core financial slots: `revenue`, `gross_profit`, `operating_income`, `net_income`, `rd_expense`, `sga_expense`, `operating_cash_flow`, `capex`, `free_cash_flow`
  - Derived field: `free_cash_flow` auto-computed as `operating_cash_flow - capex` when both present, else `None`
  - Metadata slots: `ticker`, `cik`, `company_name`, `period_end`, `form`, `filed_date`, `currency`
  - Balance sheet slots: `total_assets`, `total_liabilities`, `equity`, `shares_outstanding`
- **Smoke test:** Instantiate `CompanyFinancials` with 5 mock `PeriodFinancials` entries; assert FCF derivation is correct; print a period summary table

---

## Ticket 3.5 — Data Store ✅ CLOSED
- **Title:** Persist and retrieve CompanyFinancials as JSON with query helpers
- **Description:** Implement `financial_analysis/store.py` that saves `CompanyFinancials` to `data/processed/{TICKER}.json` (human-readable), loads it back as a typed Python object with an in-memory session cache, and exposes query helpers for time-series and cross-ticker analysis.
- **Acceptance criteria:**
  - `save(company: CompanyFinancials, data_dir: Path = Path("data/processed"))` → writes `data/processed/PANW.json`, pretty-printed, dates as ISO strings
  - `load(ticker: str, data_dir: Path = Path("data/processed")) -> CompanyFinancials` → deserializes; returns cached instance if already loaded this session (no re-parse)
  - `get_series(ticker: str, metric: str, data_dir: Path) -> list[tuple[date, float | None]]` → `(period_end, value)` pairs sorted ascending; `metric` is any `PeriodFinancials` field name (e.g. `"revenue"`, `"free_cash_flow"`)
  - `compare(tickers: list[str], metric: str, data_dir: Path) -> dict[str, list[tuple[date, float | None]]]` → per-ticker series; easily wrapped with `pd.DataFrame(compare(...))` for visualization
  - `data/processed/` directory created automatically on first save
  - Invalid metric name raises `ValueError` with a list of valid field names
  - Unit tests use a `tmp_path` fixture — no writes to real `data/` in tests
- **Smoke test:**
  - Import PANW via importer, save to disk, clear the in-memory cache, load back
  - Print `get_series('PANW', 'revenue')` — 5+ rows
  - Print `compare(['PANW'], 'fcf_margin')` — confirm values are in 0–1 range
  - Confirm `data/processed/PANW.json` is human-readable when opened in a text editor

---

## Ticket 4 — Importer Interface
- **Title:** Build the financial statement importer
- **Description:** Implement `financial_analysis/importer.py` that orchestrates CIK lookup (`edgar.py`), XBRL API fetch (`xbrl_api.py`), and schema population (`schema.py`) into a single reusable call. Add a CLI script entrypoint.
- **Acceptance criteria:**
  - `import_company(ticker: str, form: str = "10-K", n_periods: int = 8, save: bool = True) -> CompanyFinancials` is the primary API; when `save=True` auto-saves to `data/processed/{TICKER}.json` via `store.save()`
  - Maps these XBRL concepts to schema slots (with fallback concept names where PANW uses non-standard tags):
    - Revenue: `us-gaap/RevenueFromContractWithCustomerExcludingAssessedTax` → fallback `us-gaap/Revenues`
    - Operating CF: `us-gaap/NetCashProvidedByUsedInOperatingActivities`
    - CapEx: `us-gaap/PaymentsToAcquirePropertyPlantAndEquipment` → fallback `us-gaap/PaymentsToAcquireProductiveAssets` (PANW uses the latter)
    - Net income: `us-gaap/NetIncomeLoss`
  - `scripts/import_financials.py` CLI: `python -m scripts.import_financials PANW` prints a table of periods and confirms save path
  - Gracefully handles tickers with no XBRL facts (raises `EdgarClientError` with clear message)
- **Smoke test:** `python -m scripts.import_financials PANW` prints 5+ annual periods showing revenue, operating income, and FCF — all non-zero; confirms `data/processed/PANW.json` written
- **Status:** ✅ CLOSED 2026-06-05. Smoke test passed.
- **Results:**
  - Annual periods (10-K): 24, Quarterly periods (10-Q): 45
  - Revenue (FY2026): 215,938,000,000 ✓ (merged across RevenueFromContract... FY2018-FY2022 + Revenues FY2023+)
  - OCF (FY2026): 102,718,000,000 ✓
  - CapEx (FY2026): -6,042,000,000 ✓ (sign_flip applied, stored as negative)
  - raw_extras count: 146 us-gaap concepts ✓
  - Cache hit on second run ✓ (~0.5s vs ~3s for live fetch)
  - Fix: `_resolve_field` updated to merge all concept fallbacks per period_end (NVDA uses two revenue concepts across fiscal years)

---

## Ticket 4.5 — Financial Statement xlsx Export ✅ CLOSED
- **Title:** Export financial statements for any ticker to a four-tab xlsx file
- **Status:** ✅ CLOSED 2026-06-05. Smoke test passed.
- **Results:**
  - NVDA: 4,886 rows across 4 tabs (IS: 356, BS: 391, CF: 213, Raw: 3,926)
  - FY2026 revenue: 215,938,000,000 ✓; capex: -6,042,000,000 ✓ (negative); FCF: 108,760,000,000 ✓
  - `filed_date` column populated (e.g. 2026-02-25 for FY2026 10-K) ✓
  - Raw tab has 3,926 unmapped XBRL concept rows ✓
  - PANW second ticker: 4,073 rows, FY2025 revenue: 9,221,500,000 ✓
  - Empty cells for None values (not "NA" or 0) ✓

---

## Ticket 4.6 — Market Data Enrichment ✅ CLOSED
- **Title:** Add historical stock prices and analyst earnings surprises to the pipeline
- **Status:** Closed 2026-06-05. Smoke test passed.
- **Results:**
  - New modules: `financial_analysis/market_data.py` (yfinance prices + earnings), `financial_analysis/fmp.py` (FMP client + IFRS stub)
  - New schema dataclasses: `PricePoint`, `EarningsEstimate`, `EarningsSurprise`
  - `CompanyFinancials` extended with `prices`, `estimates`, `surprises` lists
  - `scripts/analyze.py` now produces 6-tab xlsx (+ Prices, + Estimates); `--no-market` for 4-tab fallback
  - NVDA smoke test: 6885 price points (yfinance, ~27 years), 25 earnings surprise records (yfinance)
  - 11871 rows across 6 tabs; 4886 rows across 4 tabs with `--no-market` ✓
  - Cache hit on second run (instant, no API calls) ✓
  - FMP analyst-estimates (v3 legacy) unavailable on free plan post-Aug 2025 — pipeline degrades gracefully to yfinance; FMP stub preserved for future upgrade

---

## Ticket 5 — Stochastic DCF Scaffold ✅ CLOSED
- **Title:** Implement Monte Carlo stochastic DCF model
- **Status:** Closed 2026-06-12. Smoke test passed.
- **Results:**
  - `financial_analysis/dcf.py`: `DCFAssumptions`, `DCFResult`, `StochasticDCF`; 13 unit tests green (`tests/test_dcf.py`); full suite 113 passed.
  - Vectorized numpy MC (no per-sim Python loop); `seed` field added for reproducible runs.
  - PANW smoke test (loaded from `data/processed/PANW.json`, no live call): P10 $69.9B · P50 $118.0B · P90 $204.2B · P50/share $176.70.
  - Min-history guard requires ≥4 annual periods with positive FCF; ignores 10-Q rows.
  - CLI added: `scripts/run_dcf.py` (overridable simulations/years/growth/wacc/terminal-growth/seed).
  - **Bug fixed en route:** `schema.free_cash_flow` was `OCF - capex` but capex is stored sign-flipped (negative), overstating FCF by 2×capex. Now `OCF - abs(capex)`; regression test added.
  - **SpaceX (SPCX) sample requested:** ticker resolves to SPACE EXPLORATION TECHNOLOGIES CORP (CIK 0001181412) but only a Form D (`ffd` namespace) is on file — no `us-gaap` financials, no 10-K (IPO'd 2026-06-12). Importer raises `UnsupportedFilingError`; DCF guard independently rejects post-IPO single-period histories. Both layers verified.

### Original spec
- **Title:** Implement Monte Carlo stochastic DCF model
- **Description:** Create `financial_analysis/dcf.py` with a `StochasticDCF` class that fits distributions to historical FCF data from `CompanyFinancials`, runs a vectorized Monte Carlo simulation, and outputs P10/P50/P90 present value estimates. Architecture must support future extension to options valuation (Black-Scholes, real options on growth).
- **Acceptance criteria:**
  - `DCFAssumptions` dataclass: `growth_rate_mean`, `growth_rate_std`, `terminal_growth_mean`, `terminal_growth_std`, `wacc_mean`, `wacc_std`, `n_simulations` (default 10 000), `projection_years` (default 10), optional `fcf_margin_mean`/`fcf_margin_std`
  - `DCFResult` dataclass: `median_value` (P50), `p10_value`, `p90_value`, `mean_value`, `std_value`, `simulations: np.ndarray`
  - `StochasticDCF(company: CompanyFinancials, assumptions: DCFAssumptions).run() -> DCFResult`
  - When `fcf_margin_mean/std` are `None`, derive them from the last 5 available FCF/revenue ratios in `company.periods`
  - MC loop is vectorized with numpy (no Python-level for-loop over simulations)
  - Per-share value computed when `shares_outstanding` is available
  - Typical usage loads from store — no re-fetch: `StochasticDCF(store.load('PANW'), assumptions).run()`
  - **Minimum history guard:** raise `ValueError` with a clear message if fewer than 4 annual periods with positive FCF are available — prevents meaningless MC output for companies with short histories or early negative FCF (e.g. post-IPO companies like DUOL)
- **Smoke test:** Load PANW from `data/processed/PANW.json` via `store.load()`; run MC; print P10/P50/P90 total equity value and per-share value — no live API call should occur

---

## Ticket 6 — README Update ✅ CLOSED
- **Status:** Closed 2026-06-12. README documents the full EDGAR → XBRL → schema → Monte Carlo DCF pipeline, adds a Stochastic DCF section with usage + `run_dcf.py` CLI, and marks Tickets 5–6 complete in the roadmap. `context.json` updated.
- **Title:** Document the full stochastic pipeline
- **Description:** Update `README.md` to reflect the XBRL API path, time-series schema, and Monte Carlo DCF direction.
- **Acceptance criteria:**
  - Describes the pipeline: EDGAR CIK lookup → XBRL API → time-series schema → Monte Carlo DCF
  - Notes future direction: options valuation, additional cybersecurity tickers
  - Includes usage examples for `import_edgar` and `import_financials` scripts
  - Lists smoke test command for each ticket stage
  - Mentions `context.json` and `PLANNING/tickets.md` as agent orientation files

---

# OPEN TICKET SPECS (not part of the historical record)

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
