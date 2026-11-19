---
id: T-20261119-report-rust
title: Move the report, analyze, the market data, the store and the importer to Rust
area: engine
status: review
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
