# FinancialAnalysis
A small Python project for importing public-company financial statements, extracting structured valuation inputs, and building a basic DCF workflow.

Initial focus:
- Data source: SEC EDGAR filings and XBRL financial statements
- First target: Palo Alto Networks (cybersecurity)
- Goal: support a generic importer for income statement, balance sheet, and cash flow variables

Current status:
- Planning tickets created in `PLANNING/tickets.md`
- Ticket 1 implemented: SEC full-index retrieval using `master.idx` and direct filing downloads
- Next work: XBRL parsing, financial schema design, importer interface, and DCF scaffold

Usage:
- Run `python -m scripts.import_edgar PANW 2025-06-30 --output-dir data/raw`
- This downloads index files and matching filings for the target ticker into `data/raw/PANW`

Agent note:
- Agents should be concise when updating `README.md` and `context.json`.
