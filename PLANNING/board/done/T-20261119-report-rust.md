---
id: T-20261119-report-rust
title: Move the report, analyze, the market data, the store and the importer to Rust
area: engine
status: done
owner: louellet-git
branch: claude/report-rust
created: 2026-11-19
acceptance: The xlsx file and the HTML report come from Rust commands with the same values as the Python commands, and the Python files of these parts are deleted.
---
Owner request, 2026-11-19: refactor the code base to Rust. This ticket is steps 4, 6,
7 and 9 of `T-20261119-python-to-rust`. The branch starts from
`claude/dcf-engine-rust` (it has the Rust engines).

Parts:

1. `scripts/analyze.py` (xlsx file) and `scripts/visualize_dcf.py` (HTML report).
2. Market data: `market_data.py`, `estimates.py`, `options.py`, `fmp.py`.
3. Store, schema, importer, import scripts, `xbrl_tags.py`.
4. The legacy SEC client. After its deletion the SEC contact comes from the
   environment variable `SEC_USER_AGENT` only.

## State (2026-11-19)

- **Part 1 (xlsx and HTML report): done.** The xlsx file is `fa analyze`. The HTML report is `fa report` (`rust/src/report/`,
  branch `claude/dcf-engine-rust`; parity with the Python page, see the ticket `dcf-engine-rust`). `scripts/analyze.py` and
  `scripts/visualize_dcf.py` are deleted. The report loads plotly.js 4.1.1 from the CDN: it needs the network (open point:
  an offline option).
- **Part 2 (market data): done.** Prices, option chains and FMP data are Rust (`rust/src/market/`). The Python files
  `market_data.py`, `options.py`, `estimates.py` and `fmp.py` are deleted. The forward consensus and the earnings dates
  have no source in Rust (decision of the owner open, `python-to-rust`, decision 3).
- **Part 3 (store, schema, importer): done.** `schema.py` and `store.py` are deleted (Rust: `rust/src/company.rs`,
  `rust/mapcheck/src/store.rs`). `financial_analysis/importer.py` has only `_run`.
- **Part 4 (the legacy SEC client): open.** The SEC contact text is in the legacy client. The owner has not decided its new
  place. The client, `importer.py` (`_run`), `financial_analysis/__init__.py`, `scripts/import_edgar.py` and
  `scripts/__init__.py` stay. After the decision: the contact comes from `SEC_USER_AGENT` only, and these five files go.
