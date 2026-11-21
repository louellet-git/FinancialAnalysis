# Epics and stories

This page gives the board a shape: **epic → story → ticket**. The Head of Staff keeps it (owner word,
2026-11-21). The tickets stay one file each; this page only puts them in order. Rules: `README.md`.

- An **epic** serves one decision of the person. It has a condition that ends it.
- A **story** is one result that the person can use or check. It has one head that leads it.
- A **ticket** is one branch and one pull request (`README.md`).
- Status marks: **now** (in progress or in review), **next** (backlog, the order below), **hold** (waits for
  a result or a word of the person), **old** (the text does not agree with the repo now; rewrite or close).
- The order of the tickets in a story is the order of the work. The `order` field of a ticket is its place
  in the queue of its head.

## E1. Trade stocks from value

Decision: which stock trades the person places each week. Ends when: three reviewed proposals are ready each
Friday, from a rule that passed its test. State: **hold**. No rule has passed (rule A-009 is invalid), and
the person said "No trades until T1 passes".

| Story | Lead | Tickets, in order | Mark |
|---|---|---|---|
| E1.S1 The engine is proven on past data | fundamental | `T-20261121-blind-projection-test` (T1 fails on revenue) → `T-20261121-trade-level-backtest` | hold (the person paused it for E2.S1) |
| E1.S2 The backtest is fair | technology | `T-20261121-backtest-environment-2`, `T-20261121-backtest-10ka-vintage` | next |
| E1.S3 A rule for the side and the size | fundamental | `T-20261121-trade-side-rule` | hold (needs E1.S1) |
| E1.S4 Weekly proposals | fundamental | `T-20261121-weekly-trade-proposals` | hold (needs E1.S1 and E1.S3) |

Gap: no ticket says what the next candidate rule is after A-009. It comes from E1.S1.

## E2. Explain a market move by valuation

Decision: how the person reads a fall or a rise of the market. Ends when: one command gives the parts of a
move (new facts, time, rates, premium, growth, no model) for a list of stocks, and one report used it.

| Story | Lead | Tickets, in order | Mark |
|---|---|---|---|
| E2.S1 The week to 2026-11-20 (report of 3 pages) | fundamental | `T-20261121-sp100-week-attribution` | now (at the gate; waits for the go-ahead of the person) |
| E2.S2 The parts that the report leaves open | fundamental | `T-20261121-premium-company-part` → `T-20261121-rates-balance-sheet` | now, next |
| E2.S3 The common part comes from Economics, not from the prices | economics | `T-20261121-economics-view-engine` (needs the SPF loader, pull request 160) → `T-20261121-economics-premium-macro` → `T-20261121-economics-jst-design` | now |
| E2.S4 The study is engine code, not a script | fundamental | `T-20261121-dcf-engine-integration` | now |

## E3. Value each type of company

Decision: which companies the person can trust the model for. Ends when: each company class has a model or
a recorded reason for no model, and each report that the person uses passed the four-step review.

| Story | Lead | Tickets, in order | Mark |
|---|---|---|---|
| E3.S1 Models for the classes that the cash-flow model does not fit | fundamental | `T-20261119-evaluator-bank` → `T-20261120-evaluator-utility` → `T-20261119-evaluator-insurer` → `T-20261119-evaluator-reit` → `T-20261121-evaluator-captive-finance` → `T-20261119-evaluator-etf-fund` → `T-20261119-evaluator-other-classes` | next |
| E3.S2 The judgments of the ticker models become rules | fundamental | `T-20261121-dcf-engine-from-tickers` | next |
| E3.S3 Ticker reports | fundamental | `T-20261025-ibm-q2-grading`, `T-20261119-duol-depth` (a decision of the person), `T-20261025-orcl-frontier-run` | next |
| E3.S4 Old tickets | fundamental | `T-20261119-engine-fixed-answer-tests` (names Pytest; the repo has no Python), `T-20261025-v3-reproducibility-drift` (names a Python module) | old |

## E4. Trade options and arbitrage from the quant models

Decision: which quant strategy gets paper orders, then real money. Ends when: one strategy passed its gates,
ran on paper under the controls of `docs/QUALITY-STANDARDS.md` section 8, and the person decided on it.
State: alpha-1 stopped at gate 1; no strategy goes to paper now.

| Story | Lead | Tickets, in order | Mark |
|---|---|---|---|
| E4.S1 The algo engine runs each day | quant | pull request 131 (`fa algo`; waits for the live run of the person, 2026-11-23) → `T-20261121-algo-host-gcp` (technology) → `T-20261121-algo-paper-orders` | hold |
| E4.S2 Checks of prices that must agree | quant | `T-20261120-arb-4-futures-basis` → `T-20261120-arb-3-index-parity-box` → the arbitrage register (`T-20261121-arb-invariants`, on its branch) | now |
| E4.S3 Volatility models | quant | `T-20261120-vol-2-sabr-localvol` → `vol-3-bergomi-2f` → `vol-4-rough-bergomi` → `vol-5-quintic-pdv` → `vol-6-joint-spx-vix` | now |
| E4.S4 The studies are engine code, and their defects are closed | quant | `T-20261121-quant-engine-integration`, `T-20261121-quant-doc-findings` (B1 to B4 before a new bot result) | now, next |
| E4.S5 Paper desk | owner | `T-20261120-paper-desk` (decision 1 is old: the paper account is permitted now), `T-20261119-options-paper-test` | hold (no strategy passed) |

Question for the frame: E4.S3 has five model tickets and no strategy that uses them yet. Each must name the
check or the strategy that needs it before it starts.

## E5. Data that is correct and available

Decision: none direct; each other epic reads this data. Ends when: the audit has no open defect on the
S&P 100, and each head runs its checks from the store with no network.

| Story | Lead | Tickets, in order | Mark |
|---|---|---|---|
| E5.S1 The store has no known defect | technology | `T-20261121-data-audit` → `T-20261121-revenue-old-tags`, `T-20261121-share-count-scale-guard`, `T-20261121-missing-price-files`, `T-20261119-filing-values-in-variables` | now |
| E5.S2 Each source is permitted | technology | `T-20261121-cboe-off`, `T-20261121-fred-local` → `T-20261121-economics-data` → `T-20261121-daily-history` (a decision of the person) | now |
| E5.S3 Work with no network | technology | `T-20261121-cache-only-switch`, `T-20261120-store-arb-data-rest`, `T-20261120-sync-chains` | next |
| E5.S4 Live view | technology | `T-20261119-live-data-stream` | next (last) |

## E6. Controls

Decision: the person can trust a number and the repo is safe to show. Ends when: each model has its
documents, the agent teams ran on real work, and the public repo has no private file in its history.

| Story | Lead | Tickets, in order | Mark |
|---|---|---|---|
| E6.S1 Each model has its documents | quality | `T-20261121-nvda-assumption-records` → `T-20261120-model-docs-backlog` | now |
| E6.S2 The review teams work | quality | `T-20261120-agent-teams-first-run`, `T-20261025-bridge-per-ticker-rule`, `T-20261025-p1-validators-redteam` (names Pytest) | next, old |
| E6.S3 Work of the person | owner | `T-20261120-owner-first-run`, `T-20261120-delete-merged-branches` → `T-20261025-keep-private-frameworks-out-of-the-public-repo` | waits for the person |

## Order across the epics (proposal of the Head of Staff; the person decides)

1. E2.S1, because the person asked for it first and paused other work for it.
2. E5.S1, because each result of E1 and E2 reads this data.
3. E1.S1, because it opens E1.S3 and E1.S4.
4. E4.S1, on the date of the live run.
5. The other stories, in the order of their heads.
