# Financial Statement Automation Design

**Date:** 2026-06-05
**Status:** Approved for implementation

---

## Goal

Replace ad-hoc exploratory scripts with a single reusable command that fetches, caches, and exports full financial statements for any SEC us-gaap filer to a structured xlsx file. Eliminate repeated API calls, concept discovery scripts, and throwaway formatting code. Output moves to disk (not terminal) to minimize token usage in future sessions.

---

## Architecture

```
scripts/analyze.py NVDA [--refresh]
        |
        ├── cache.load_company_facts('NVDA', cache_dir=data/raw, refresh=False)
        │       ├── cache hit  → read data/raw/NVDA/company_facts.json
        │       └── cache miss → fetch SEC API → write to disk → return
        │
        ├── detect IFRS / non-USD → raise UnsupportedFilingError (abort early)
        │
        ├── import_company('NVDA')          ← Ticket 4, extended
        │       ├── xbrl_tags.py            ← concept registry with sign_flip + amended forms
        │       ├── schema.py               ← populates annual + quarterly periods
        │       ├── reconciliation.py       ← validates before save
        │       └── store.save()            → data/processed/NVDA.json
        │
        └── store.load('NVDA')
                └── flatten all PeriodFinancials → long-format rows
                        └── write data/processed/NVDA_financials.xlsx (4 tabs)
```

Single fetch per session. Output goes to disk. Terminal prints one confirmation line.

---

## Components

### 1. `financial_analysis/cache.py` — new

Single public function:

```python
def load_company_facts(
    ticker: str,
    cache_dir: Path,
    refresh: bool = False,
) -> dict:
```

Behaviour:
- Cache path: `cache_dir / ticker.upper() / "company_facts.json"`
- If `refresh=False` and file exists: attempt to read and parse. On `json.JSONDecodeError` (corrupt file): log warning, fall through to live fetch.
- If `refresh=True` or file missing or corrupt: call `get_cik_for_ticker()` + `fetch_company_facts()`, write result to disk with `encoding="utf-8"`, return dict.
- Creates `cache_dir / ticker.upper() /` automatically if absent.
- `fetch_company_facts()` in `xbrl_api.py` is **not modified** — caching is a caller-level concern.

---

### 2. `financial_analysis/xbrl_tags.py` — two additions to existing design

**a) `sign_flip` flag per concept**

Each entry in `INCOME_STATEMENT_TAGS`, `BALANCE_SHEET_TAGS`, `CASH_FLOW_TAGS` gains an optional `sign_flip: bool` (default `False`). When `True`, the importer multiplies the raw XBRL value by `-1` after fetching. Applied once at import time, never again downstream.

CapEx concepts default to `sign_flip=True` — companies file outflows as positive but we store them as negative to match sign convention (cash out = negative). This eliminates the `abs()` workaround discovered during the NVDA walkthrough.

`TagSpec` is a simple dataclass defined at the top of `xbrl_tags.py`:

```python
@dataclass
class TagSpec:
    concepts: list[str]       # tried in order; first with data wins
    sign_flip: bool = False   # multiply raw value by -1 after fetch

CASH_FLOW_TAGS: dict[str, TagSpec] = {
    "capex": TagSpec(
        concepts=[
            "us-gaap/PaymentsToAcquireProductiveAssets",
            "us-gaap/PaymentsToAcquirePropertyPlantAndEquipment",
        ],
        sign_flip=True,  # filed as positive outflow; stored as negative
    ),
    ...
}
```

**b) Amended form filters**

All form type filters expand to include amendments:
- Annual: `["10-K", "10-K/A"]`
- Quarterly: `["10-Q", "10-Q/A"]`

The existing `_deduplicate` logic (keep latest `filed` date per `end` date) naturally prefers the amendment over the original filing when both exist.

---

### 3. `financial_analysis/importer.py` — extensions to Ticket 4 scope

**Quarterly support**

`import_company()` fetches both annual and quarterly series for every concept in the tag registry. `CompanyFinancials.periods` contains a mixed list of `PeriodFinancials`, each tagged with `form` (`"10-K"` or `"10-Q"`). Existing callers that only use annual periods are unaffected — they filter by `period.form == "10-K"`.

**IFRS / non-USD detection**

Immediately after fetching facts, before any concept extraction: check that `"us-gaap"` is present in the facts namespaces. If not, raise `UnsupportedFilingError` with a message listing the namespaces found. `UnsupportedFilingError` is defined in `financial_analysis/importer.py` alongside the existing `EdgarClientError` pattern from `edgar.py`.

If all monetary concepts return empty (non-USD reporting currency), the reconciliation pass will raise `ReconciliationError` before saving, so no explicit check is needed for that case.

**Any-ticker support**

No hardcoded ticker list. `get_cik_for_ticker()` resolves any SEC-registered ticker. Company-specific extension tags (e.g. `panw/ProductRevenue`) are defined in `xbrl_tags.py` alongside the us-gaap fallbacks and are tried in order. For tickers with no company-specific tags, the us-gaap baseline populates all canonical fields. Unknown concepts land in `raw_extras`.

---

### 4. `scripts/analyze.py` — new CLI

```
python -m scripts.analyze NVDA
python -m scripts.analyze NVDA --refresh
python -m scripts.analyze NVDA --out path/to/custom.xlsx
```

Steps:
1. Call `cache.load_company_facts(ticker, cache_dir=Path("data/raw"), refresh=args.refresh)`
2. Detect IFRS → abort with clear message
3. Call `import_company(ticker)` → populates and saves `data/processed/NVDA.json`
4. Call `store.load(ticker)` → typed `CompanyFinancials`
5. Flatten all `PeriodFinancials` to long-format rows (see CSV Format below)
6. Write `data/processed/NVDA_financials.xlsx` via `openpyxl` with four tabs
7. Print: `Written: data/processed/NVDA_financials.xlsx (N rows across 4 tabs)`

---

## xlsx Format

**Output file:** `data/processed/{TICKER}_financials.xlsx`

**New dependency:** `openpyxl` (no other dependencies added)

### Tabs

| Tab | Content |
|---|---|
| `Income Statement` | Canonical IS metrics — revenue waterfall, margins, EPS, SBC, EBITDA family |
| `Balance Sheet` | Canonical BS metrics — assets, liabilities, equity, derived ratios |
| `Cash Flow` | Canonical CF metrics — OCF, CapEx, FCF, buybacks, financing |
| `Raw` | All `raw_extras` — every XBRL concept not mapped to a canonical field |

### Columns (all tabs)

| Column | Type | Example |
|---|---|---|
| `ticker` | string | `NVDA` |
| `cik` | string | `1045810` |
| `company_name` | string | `NVIDIA CORP` |
| `period_end` | ISO date | `2024-10-27` |
| `form_type` | string | `10-Q` |
| `filed_date` | ISO date | `2024-11-20` |
| `statement` | string | `income_statement` |
| `metric` | string | `revenue_total` |
| `value` | float or empty | `35082000000` |
| `unit` | string | `USD` |

### Sample rows

```
ticker, cik,     company_name, period_end,  form_type, filed_date,  statement,        metric,          value,         unit
NVDA,   1045810, NVIDIA CORP,  2024-10-27,  10-Q,      2024-11-20,  income_statement, revenue_total,   35082000000,   USD
NVDA,   1045810, NVIDIA CORP,  2024-10-27,  10-Q,      2024-11-20,  income_statement, gross_margin,    0.7449,        ratio
NVDA,   1045810, NVIDIA CORP,  2024-10-27,  10-Q,      2024-11-20,  cash_flow,        free_cash_flow,  16816000000,   USD
NVDA,   1045810, NVIDIA CORP,  2025-01-26,  10-K,      2025-02-26,  balance_sheet,    total_assets,    111600000000,  USD
NVDA,   1045810, NVIDIA CORP,  2024-10-27,  10-Q,      2024-11-20,  income_statement, eps_diluted,     0.81,          USD/share
NVDA,   1045810, NVIDIA CORP,  2025-01-26,  10-K,      2025-02-26,  raw,              us-gaap/PaymentsToAcquireAvailableForSaleSecuritiesDebt, 40620000000, USD
NVDA,   1045810, NVIDIA CORP,  2025-01-26,  10-K,      2025-02-26,  raw,              us-gaap/PaymentsToAcquireEquitySecuritiesFvNi, 17500000000, USD
```

### Design decisions

- **Empty cell for missing values** — not `"NA"` or `0`; pandas reads as `NaN` automatically
- **`form_type` column** — distinguishes annual from quarterly; filter `form_type == "10-K"` for annual-only view
- **`filed_date` column** — critical for point-in-time analysis and backtesting
- **Raw tab metric names** — full XBRL concept string (e.g. `us-gaap/PaymentsToAcquireEquitySecuritiesFvNi`); unambiguous, traceable to SEC source
- **Encoding** — all file writes use explicit `encoding="utf-8"`; no cp1252 fallback

---

## Edge Cases

| Case | Handling |
|---|---|
| Ticker not in SEC | `EdgarClientError` from `get_cik_for_ticker()` — already implemented |
| IFRS filer (no us-gaap namespace) | `UnsupportedFilingError` raised before any extraction |
| Non-USD monetary reporting | Reconciliation raises `ReconciliationError`; data not saved |
| Corrupt cache file | `json.JSONDecodeError` caught; falls back to live fetch, overwrites bad file |
| Amended filings (10-K/A, 10-Q/A) | Included in form filters; deduplication keeps latest-filed per period end |
| CapEx sign conventions | `sign_flip=True` in tag registry normalizes at import; no `abs()` in analysis code |
| Stock splits | Raw XBRL values as-filed; no restatement. `filed_date` column lets users identify split dates |
| Short filing history (recent IPOs) | Fewer rows in xlsx; DCF guard enforces minimum 4 annual periods with positive FCF |
| Missing concepts | Empty `value` cell; no row omitted |
| Windows UTF-8 | All file writes use explicit `encoding="utf-8"` |

---

## Ticket Structure

Updated sequence: **4 → 4.5 → 5 → 6**

### Ticket 4 (extended) — Importer Interface

Original scope plus:
- `financial_analysis/cache.py` — disk-backed fetch, corruption fallback, `--refresh` flag
- `xbrl_tags.py` — `sign_flip` flag, amended form filters
- `import_company()` — annual + quarterly periods, IFRS detection, any-ticker support
- `openpyxl` added to `requirements.txt` (file created if absent — project currently has no requirements file; stdlib + numpy are the only existing dependencies)

Smoke test: `python -m scripts.import_financials NVDA` — prints annual period count, quarterly period count, confirms `data/processed/NVDA.json` written, confirms cache at `data/raw/NVDA/company_facts.json`

---

### Ticket 4.5 (new) — Financial Statement xlsx Export

- `scripts/analyze.py` — full CLI with `--refresh` and `--out` flags
- Four-tab xlsx output: Income Statement, Balance Sheet, Cash Flow, Raw
- Long-format rows, all periods (annual + quarterly), UTF-8, empty nulls

Smoke test: `python -m scripts.analyze NVDA` — opens `NVDA_financials.xlsx`, confirms four tabs present, Income Statement tab has canonical metrics with non-empty values for recent periods, Raw tab has 400+ rows for a company like NVDA with 626 us-gaap concepts

---

### Ticket 5 — Stochastic DCF Scaffold (unchanged)

Loads from `store.load()`. Reads annual periods only via `form == "10-K"` filter. No changes needed — quarterly data in store is additive and does not affect DCF inputs.

---

### Ticket 6 — README Update (unchanged)

Add `analyze` command to usage examples alongside `import_financials`.

---

## Backwards Compatibility

- `store.load()`, `get_series()`, `compare()` — unchanged
- `import_company()` return type — unchanged (`CompanyFinancials`); quarterly periods are additive
- `fetch_company_facts()` — unchanged; cache is caller-level
- DCF model — reads annual-only from store; quarterly data invisible to it
- `data/processed/PANW.json` — existing saved files remain valid; re-import adds quarterly periods on next run
