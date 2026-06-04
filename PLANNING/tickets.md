# Planning Tickets

## 1. EDGAR Import
- Title: Implement SEC EDGAR filing retrieval
- Description: Build a module that downloads filings for a given public company ticker and reporting period using SEC EDGAR.
- Acceptance criteria:
  - Accept a company ticker (e.g. PANW) and a year/quarter or filing type.
  - Download the corresponding filing index and retrieve the XBRL or HTML filing package.
  - Return local paths to the downloaded filing assets.

## 2. XBRL Parser
- Title: Parse EDGAR financial filings into structured variables
- Description: Create an XBRL parser that extracts financial statement line items from downloaded filings, including income statement, balance sheet, and cash flow statement.
- Acceptance criteria:
  - Accept a downloaded filing path and extract standardized financial variables.
  - Populate metadata fields: company ticker, report period, filing date, currency, statement type.
  - Return a structured dictionary or dataclass of financial slots.

## 3. Financial Schema
- Title: Define the core financial variable schema for cybersecurity companies
- Description: Create a schema of financial slots that includes core financial metrics for a cybersecurity company and remains extensible for other sectors.
- Acceptance criteria:
  - Include core slots such as revenue, gross profit, operating income, net income, R&D, SG&A, operating cash flow, capital expenditures, total assets, total liabilities, and equity.
  - Include metadata slots for company name, ticker, filing date, report period, currency, and shares outstanding.
  - Document how to extend the schema with additional XBRL variables.

## 4. Importer Interface
- Title: Build the financial statement importer interface
- Description: Implement `financial_analysis/importer.py` and a script entrypoint to load filings, parse them, and expose structured input slots for modeling.
- Acceptance criteria:
  - `financial_analysis/importer.py` provides a reusable importer API.
  - `scripts/import_financials.py` can be run to import a target company filing and output parsed financial slots.
  - The importer supports EDGAR retrieval and XBRL parsing from the first two tickets.

## 5. DCF Scaffold
- Title: Implement a basic DCF model consumer
- Description: Create a minimal valuation engine that consumes imported financial slots and computes a present value estimate from free cash flow.
- Acceptance criteria:
  - Implement `financial_analysis/dcf.py` with a simple `DCFModel` class.
  - Consume historic financial inputs and user-provided assumptions.
  - Output an estimated present value of cash flows.

## 6. README Update
- Title: Document the financial importer workflow
- Description: Update `README.md` with the project vision, EDGAR data source, target cybersecurity company, and usage instructions.
- Acceptance criteria:
  - Clearly describe the pipeline from EDGAR to parsed financial slots to DCF modeling.
  - Note the initial target company: Palo Alto Networks.
  - Include steps for running the importer script and extending the schema.
