---
id: T-20261119-python-to-rust
title: Move the remaining Python code to Rust
area: engine
status: backlog
owner: louellet-git
branch:
created: 2026-11-19
acceptance: The repo has no Python file, each moved part has Rust tests, and each command of README.md is a Rust command.
---
Owner decision, 2026-11-19: the language of the repo is Rust, for speed. One part has one implementation. Rule: `CLAUDE.md`, section "Work rules".

Done on 2026-11-19: the mapping (SEC fetch, period selection, tags, formulas, sources, checks, filing statements, comparison with the filing) is Rust only (`rust/mapcheck`, 4,375 lines, 66 tests). The Python copies are deleted. The Python importer only runs the Rust program.

Python code that stays (lines, 2026-11-19), in the recommended sequence of the move:

| Step | Part | Python lines | Notes |
|---|---|---|---|
| 1 | DCF engines, scenarios, verdict | 1,320 | **Done (2026-11-19):** the Python engine files are deleted. | First write the fixed-answer tests (ticket `engine-fixed-answer-tests`); they show that the Rust engine gives the same values. The random numbers of numpy do not exist in Rust: compare with randomness off, and compare distributions. |
| 2 | Driver models (IBM, JNJ) | 905 | Make them data files, not code. The IBM model is a reviewed record: its numbers must stay the same. |
| 3 | Backtest | 536 | **Done (2026-11-19, branch `claude/dcf-engine-rust`):** `fa backtest`; the Python command and its CLI tests are deleted. `financial_analysis/backtest.py` stays until the report moves (`scripts/visualize_dcf.py` imports it). |
| 4 | Store and schema readers | 657 | **Done (2026-11-19):** `schema.py` and `store.py` are deleted with the Python engines and tests (Rust: `rust/src/company.rs`). |
| 5 | Rates and bonds | 977 | **Done (2026-11-19, branch `claude/dcf-engine-rust`):** `fa rates`, `fa bonds` and `fa verdict` with market rates are Rust (`rates.rs`, `bonds.rs`, `verdict_rates.rs`); the Python modules and tests are deleted. FRED does not answer the cloud container (HIDDEN-ASSUMPTIONS #128). |
| 6 | Market data: prices, estimates, option chains | 824 | These use the Python library yfinance. **Trading part done (2026-11-19):** the bars (Yahoo chart endpoint, `fa trade fetch`) and the live option chain (Cboe delayed quotes, `fa trade paper`) are Rust (`rust/src/trading/fetch.rs`, `chain.rs`). Left: prices, estimates and option chains of the valuation part (`financial_analysis/`). Its option chains can use the Cboe file of `chain.rs`. |
| 7 | Report: xlsx file and HTML page | 2,254 | **Done (2026-11-19, branch `claude/dcf-engine-rust`):** `fa analyze` (xlsx) and `fa report` (HTML, `rust/src/report/`). The Rust page draws its charts with plotly.js 4.1.1 from the CDN: it needs the network (open point: an offline option). The Python script is deleted. |
| 8 | Trading backtester and bots | 3,535 | **Done (2026-11-19, branch `claude/option-backtest-rust`, with the merge of that branch):** backtester, 22 bots, bars, live chain and paper trading are Rust. The Python copy and its tests are deleted. Ticket `option-backtest-rust`. |
| 9 | SEC client (legacy) | 334 | `rust/mapcheck/src/sec.rs` replaces it. It has a contact text in a public repo. |
| | Python tests | 4,035 | Each test moves with its part. |

Open decisions of the owner:
1. **Yahoo data. Decided 2026-11-19 by the owner:** the free Yahoo source is allowed in Rust for stock prices and option chains. Result for the trading part: the Yahoo chart endpoint works for bars, but the cloud container gets HTTP 429 for about half of the requests (the program waits, tries once more, and stops). The Yahoo option endpoint refuses a program with no browser identity (the crumb request answers HTTP 429), and the program does not imitate a browser; the option chain comes from the Cboe delayed quotes (about 15 minutes late). Text of the question before the decision: the market data (prices, option chains, estimates) come from Yahoo through the Python library yfinance. A Rust program must call the Yahoo web addresses directly; these are not a published API. On 2026-11-19 a safety check of the agent environment refused a direct test of such a call. Options: permit direct calls, use a data source with a published API and a key, or keep this part in Python longer.
2. **The crate `rust/`.** (Changed 2026-11-19: the rates, bonds, DCF and verdict no longer call Python; only `fa run` does.) It was Rust code that called Python code ("Python stays the only implementation"). This does not agree with the decision. Other sessions added it, with new Python modules (rates, bonds), on 2026-11-18 and 2026-11-19. Tell the other persons and sessions about the decision.
3. **Source of estimates and earnings dates (new, 2026-11-19, ticket `report-rust`).** The Python library yfinance gave the forward consensus (revenue and EPS estimates, price targets) and the earnings dates with the estimate and the actual result. Its endpoints need a cookie and a crumb. Yahoo refuses them for a program that gives its real name, and the program does not imitate a browser. The Rust program (`fa analyze`) reads the cache files of the earlier Python code and says "no source" when there is none. Financial Modeling Prep (key `FMP_API_KEY`) gives estimates and surprises; its v3 endpoints are closed for new keys since August 2025. Options: use FMP with a key (stable endpoints), another data source with a published API, or do without the forward consensus.

## State (2026-11-19, after the report): one open step

Done: steps 1 to 8 (all valuation, report, backtest, rates, bonds, market data, option trading). No `fa` command starts Python
(`fa run`, `Fa::run_captured`, `default_python` and `FA_PYTHON` are deleted). No Python test remains (the CI job `pytest` is
removed). The packages that the remaining Python code needs: `requests` and `python-dotenv` (kept because the legacy client
was not read).

**The one open step (step 9): the legacy SEC client.** The SEC contact text is in the legacy client. The owner has not decided
its new place. These five Python files stay until then (about 390 lines):

| File | Why it stays |
|---|---|
| the legacy SEC client in `financial_analysis/` (272 lines) | The SEC contact text is in it. |
| `financial_analysis/importer.py` (50 lines, only `_run`) | It gives the contact text of the client to the Rust program when `SEC_USER_AGENT` is not set. |
| `financial_analysis/__init__.py` (3 lines) | It re-exports the client. |
| `scripts/import_edgar.py` (62 lines) | The one Python command: it downloads the filing files with the client. |
| `scripts/__init__.py` (0 lines) | Package marker of `scripts/`. |

After the decision of the owner: set the contact from `SEC_USER_AGENT` only (as `rust/mapcheck/src/sec.rs` does), delete these five
files and `requirements.txt`, and set the acceptance of this ticket ("no Python file") as met.

Open decisions of the owner, added: the offline report (inline a local copy of plotly.js), and the new place of the SEC
contact text.
