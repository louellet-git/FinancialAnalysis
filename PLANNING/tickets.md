# Planning Tickets

> Each ticket must end with a mandatory smoke test before it is closed.
> A ticket is not done until real output is printed and verified consumable by the next ticket.

---

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

## Ticket 5 — Stochastic DCF Scaffold
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

## Ticket 6 — README Update
- **Title:** Document the full stochastic pipeline
- **Description:** Update `README.md` to reflect the XBRL API path, time-series schema, and Monte Carlo DCF direction.
- **Acceptance criteria:**
  - Describes the pipeline: EDGAR CIK lookup → XBRL API → time-series schema → Monte Carlo DCF
  - Notes future direction: options valuation, additional cybersecurity tickers
  - Includes usage examples for `import_edgar` and `import_financials` scripts
  - Lists smoke test command for each ticket stage
  - Mentions `context.json` and `PLANNING/tickets.md` as agent orientation files
