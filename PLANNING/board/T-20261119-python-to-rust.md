---
id: T-20261119-python-to-rust
title: Move the remaining Python code to Rust
area: engine
status: review
owner: louellet-git
branch: claude/sec-contact-file
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
| 9 | SEC client (legacy) | 334 | **Done (2026-11-20):** `rust/mapcheck/src/sec.rs` replaces it; the contact is in `sec_contact.toml`. |
| | Python tests | 4,035 | Each test moves with its part. |

Open decisions of the owner:
1. **Yahoo data. Decided 2026-11-19 by the owner:** the free Yahoo source is allowed in Rust for stock prices and option chains. Result for the trading part: the Yahoo chart endpoint works for bars, but the cloud container gets HTTP 429 for about half of the requests (the program waits, tries once more, and stops). The Yahoo option endpoint refuses a program with no browser identity (the crumb request answers HTTP 429), and the program does not imitate a browser; the option chain comes from the Cboe delayed quotes (about 15 minutes late). Text of the question before the decision: the market data (prices, option chains, estimates) come from Yahoo through the Python library yfinance. A Rust program must call the Yahoo web addresses directly; these are not a published API. On 2026-11-19 a safety check of the agent environment refused a direct test of such a call. Options: permit direct calls, use a data source with a published API and a key, or keep this part in Python longer.
2. **The crate `rust/`.** (Changed 2026-11-19: the rates, bonds, DCF and verdict no longer call Python; only `fa run` does.) It was Rust code that called Python code ("Python stays the only implementation"). This does not agree with the decision. Other sessions added it, with new Python modules (rates, bonds), on 2026-11-18 and 2026-11-19. Tell the other persons and sessions about the decision.
3. **Source of estimates and earnings dates (new, 2026-11-19, ticket `report-rust`).** The Python library yfinance gave the forward consensus (revenue and EPS estimates, price targets) and the earnings dates with the estimate and the actual result. Its endpoints need a cookie and a crumb. Yahoo refuses them for a program that gives its real name, and the program does not imitate a browser. The Rust program (`fa analyze`) reads the cache files of the earlier Python code and says "no source" when there is none. Financial Modeling Prep (key `FMP_API_KEY`) gives estimates and surprises; its v3 endpoints are closed for new keys since August 2025. Options: use FMP with a key (stable endpoints), another data source with a published API, or do without the forward consensus.
   **Answer, 2026-11-19, ticket `estimates-earnings`:** the Nasdaq site API (vendor Zacks, no key, not a published API) gives the consensus EPS, the price targets, the ratings, the surprises with the report dates and an earnings calendar (`fa estimates`, `fa earnings`). The SEC filing list (form 8-K, item 2.02) checks the past dates. There are no revenue estimates in it; FMP with a key stays the source of revenue estimates. Alpha Vantage and Finnhub were read: the free plan is not clearly documented to give revenue estimates. The owner may still choose a paid source. The Python cache files stay readable.

## State (2026-11-20): the last step is done

Done: steps 1 to 9. Owner decision, 2026-11-20: the SEC contact is a project address in the public file `sec_contact.toml`
(key `user_agent`). The environment variable `SEC_USER_AGENT` replaces it. The function `mapcheck::http::sec_user_agent`
gives the contact: variable first, then the file (folder of `FA_REPO`, the current folder and its parents, the repo root known
at build time). `Client::sec`, `fa sync` (skip rule and status line), `fa import`, `fa analyze` and `fa serve` use it. A real
request to the SEC without the variable worked (`mapcheck fetch --refresh IBM`, then `mapcheck values IBM`). See
`docs/HIDDEN-ASSUMPTIONS.md` #285. The old private contact text of the legacy client stays in the git history.

The orchestrator deletes the five Python files (the legacy SEC client in `financial_analysis/`, `importer.py`,
`financial_analysis/__init__.py`, `scripts/import_edgar.py`, `scripts/__init__.py`) and `requirements.txt`. After that the
acceptance ("The repo has no Python file ...") is met. The status is `review` until the owner or the reviewer accepts it.

Open decision of the owner, still open: the offline report (inline a local copy of plotly.js).
