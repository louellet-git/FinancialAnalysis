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
| E2.S5 The AI build-out: its financing and its limits (a report of 1 to 2 pages for the person) | economics (fundamental gives the filing numbers) | the ticket of the Head of Economics (2026-11-23) | now |
| E2.S6 Why a fund or a sector is at its level: one-page answers for the person (first: URA, 2026-11-23) | economics (fundamental gives the implied numbers) | the ticket of the Head of Economics | now |

## E3. Value each type of company

Decision: which companies the person can trust the model for. Ends when: each company class has a model or
a recorded reason for no model, and each report that the person uses passed the four-step review.

| Story | Lead | Tickets, in order | Mark |
|---|---|---|---|
| E3.S1 Models for the classes that the cash-flow model does not fit | fundamental | `T-20261119-evaluator-bank` → `T-20261120-evaluator-utility` → `T-20261119-evaluator-insurer` → `T-20261119-evaluator-reit` → `T-20261121-evaluator-captive-finance` → `T-20261119-evaluator-other-classes` (the fund look-through is in E3.S5) | next |
| E3.S2 The judgments of the ticker models become rules | fundamental | `T-20261121-dcf-engine-from-tickers` | next |
| E3.S3 Ticker reports | fundamental | `T-20261025-ibm-q2-grading`, `T-20261119-duol-depth` (a decision of the person), `T-20261025-orcl-frontier-run` | next |
| E3.S4 Old tickets | fundamental | `T-20261119-engine-fixed-answer-tests` (names Pytest; the repo has no Python), `T-20261025-v3-reproducibility-drift` (names a Python module) | old |
| E3.S5 Value what a fund holds: remove the limits of the URA page (owner word 2026-11-23) | technology (data), fundamental (models) | dated fund holdings from the SEC form N-PORT → a scout of which holdings our SEC data can read (foreign filers) → the uranium prices and the physical trust in the store → a design of a value model for a miner → `T-20261119-evaluator-etf-fund` (the look-through) | now (the small parts), next (after the token reset) |

## E4. Trade options and arbitrage from the quant models

Decision: which quant strategy gets paper orders, then real money. Ends when: one strategy passed its gates,
ran on paper under the controls of `docs/QUALITY-STANDARDS.md` section 8, and the person decided on it.
State: alpha-1 stopped at gate 1; no strategy goes to paper now.

| Story | Lead | Tickets, in order | Mark |
|---|---|---|---|
| E4.S1 The algo engine runs each day | quant | pull request 131 (`fa algo`; waits for the live run of the person, 2026-11-23) → the continuous option capture (a part of `T-20261121-arb-invariants`) → `T-20261121-algo-host-gcp` (technology) → `T-20261121-algo-paper-orders` | hold |
| E4.S2 Checks of prices that must agree | quant | the arbitrage register (`T-20261121-arb-invariants`, in review on its branch) → `T-20261120-arb-3-index-parity-box` → `T-20261120-arb-4-futures-basis` (steps 2 and 3); the two checks become relations of the register | now |
| E4.S3 Volatility models | quant | `T-20261120-vol-2-sabr-localvol` (the SVI part; the bot leg prices need it) | next |
| E4.S4 The studies are engine code, and their defects are closed | quant | `T-20261121-quant-engine-integration` with its builds (`T-20261121-trade-grids`, `T-20261121-recon-lasting`, `T-20261121-selftest-golden`, on their branches), `T-20261121-quant-doc-findings` (B1 to B4 before a new bot result) | now, next |
| E4.S5 Paper desk | owner | `T-20261120-paper-desk` (decision 1 is decided: the Alpaca paper account), `T-20261119-options-paper-test`, `T-20261121-paper-alpaca-chain` (the chain source after the Cboe stop) | hold (no strategy passed) |

Wait list (no check and no strategy needs them now; each ticket says so): `vol-3-bergomi-2f`,
`vol-4-rough-bergomi`, `vol-5-quintic-pdv`, `vol-6-joint-spx-vix`. They start when a strategy or a check
names them.

## E5. Data that is correct and available

Decision: none direct; each other epic reads this data. Ends when: the audit has no open defect on the
S&P 100, and each head runs its checks from the store with no network.

| Story | Lead | Tickets, in order | Mark |
|---|---|---|---|
| E5.S1 The store has no known defect | technology | `T-20261121-data-audit` (done) → `T-20261121-revenue-old-tags` (on its branch), `T-20261121-share-count-scale-guard`, `T-20261121-missing-price-files`, `T-20261125-bwxt-lease-scale`, `T-20261119-filing-values-in-variables` | now |
| E5.S2 Each source is permitted | technology | `T-20261121-cboe-off` (done), `T-20261121-fred-local` → `T-20261121-economics-data` → `T-20261121-daily-history` (a decision of the person) | now |
| E5.S3 Work with no network | technology | `T-20261121-cache-only-switch`, `T-20261120-store-arb-data-rest`, `T-20261120-sync-chains` | next |
| E5.S4 Live view | technology | `T-20261119-live-data-stream` | next (last) |

## E6. Controls

Decision: the person can trust a number and the repo is safe to show. Ends when: each model has its
documents, the order code passed its gate, and the public repo has no private file in its history.

| Story | Lead | Tickets, in order | Mark |
|---|---|---|---|
| E6.S1 Each model has its documents | each head for its models; quality is the gate | `T-20261120-model-docs-backlog` (the list of what is missing; each head marks its rows) | now |
| E6.S2 The gates are repeatable | quality | `fa selftest golden` (row 25 of `T-20261121-quant-engine-integration`; Quant builds) → the mutation-test script (row 26), `T-20261025-bridge-per-ticker-rule` | next |
| E6.S3 Work of the person | owner | `T-20261120-owner-first-run`, `T-20261120-delete-merged-branches` → `T-20261025-keep-private-frameworks-out-of-the-public-repo` | waits for the person |
| E6.S4 Controls of orders and of data terms | quality (technology for the terms) | the gate "Order code (paper only)" before the first order (controls 0 to 8), a terms line for each source in `docs/DATA-SOURCES.md` | now (no ticket yet) |
| E6.S5 The rules are checked by a script, not by memory | technology (the Head of Staff reads the output) | `T-20261121-board-check` (done; run `fa board check`) → `T-20261121-usage-log` → `T-20261121-token-calculator` (on hold: needs the data); also this page: `T-20261121-board-epics` (staff) | next |

To close (the Head of Quality and Risk reported them done or replaced on 2026-11-21; the files are still
open on `main`): `T-20261121-nvda-assumption-records`, `T-20261120-agent-teams-first-run`,
`T-20261025-p1-validators-redteam`.

## Order across the epics

**Week of 2026-11-25 to 2026-12-02 (the token limit resets at 16:00 UTC on each date). Agreed with the
person on 2026-11-25 ("Agreed to the week"); it is an order and a pace, not a stop.**

Facts of the week before: the limit reached 99% on Monday 2026-11-23, and the heads stopped for two days.
All projects of the person share the limit. The shares below are estimates from the cost of that week.

| Order | Story | Result of the week | Head | Share of the week (estimate) |
|---|---|---|---|---|
| 1 | E4.S1 | Owner word 2026-11-25: the quant team begins algo trading on the Alpaca paper account (live data; the broker makes the fills, the spreads and the fees). First a one-page plan, then the gate "Order code (paper only)", then the first paper orders; the always-on host | quant, technology | 8% |
| 2 | E3.S5 | The IFRS mapping for Cameco and the size of the rest; the scout of the sources outside the SEC; the design of a miner model (one page, for the person) | technology, fundamental | 10% |
| 3 | E1.S1 | The T1 correction of the projection test, with its gate | fundamental | 6% |
| 4 | E2.S1 | The S&P 100 part, after the review of the three S&P pieces by the person | fundamental | 5% |
| 5 | E5.S1, E6.S5 | The data audit closed; the usage table of each head at the close of each ticket | technology, each head | 3% |
| - | Gates | The reviews of the work above | quality | 6% |
| - | Staff | The board map, the reads before the person | staff | 2% |
| - | Reserve | The questions of the person and new requests | - | 5% |
| | | **Financial Analysis in all (owner word 2026-11-25: 45%; Scan Support 15%, ShredTree 30%)** | | **45%** |

Below the line if the 45% is used: the uranium and gold series, the last step of the FRED replacement,
the first real run of the economics view. They move to the next week; no result is lost.

Owner word 2026-11-25 (E2.S6, E3.S5): URA and GLD are decomposed in five layers, so that a fund is not a
black box: 1 identity (the price against what the fund holds), 2 attribution (which holdings made the move),
3 drivers, 4 value, 5 flows. Layers 1 and 2 are first, for both funds, over 3 years; then layer 3. The gold
and uranium series are above the line again for this. The sizes come to the Head of Staff before a build.

Not in this week: the models for banks, utilities, insurers and REITs; the volatility models; the token
calculator; the live data view; the trade-level backtest.

Pace: Financial Analysis uses about 6% to 7% of the week for each day. The Head of Staff measures it as the
cost of the six sessions since the reset (`get_session`; about 850 USD at list price is 45%, an estimate
from the week before) and tells the heads once if it is above 40% of that on Friday, 65% on Sunday or 90%
on Tuesday (each at 16:00 UTC).

**Table: the share of each head for the week to 2026-12-02** (session cost in USD at list price; 1% of the
weekly limit is about 19 USD, an estimate). See: a head reads its own use with one call and one subtraction.

| Head | Share of the limit | Share in USD | Cost of the session at the reset (USD) | Used on 2026-11-25, 21:50 UTC |
|---|---|---|---|---|
| Technology | 12% | about 227 | 674.19 | 30 (13% of its share) |
| Fundamental Research | 13% | about 246 | 407.65 | 59 (24%) |
| Quality and Risk | 6% | about 113 | 284.85 | 20 (18%) |
| Quant Research | 5% | about 95 | 169.53 | 26 (27%) |
| Economics | 2% | about 38 | 153.84 | 16 (42%) |
| Staff | 2% | about 38 | 108.23 | about 33 (87%) |
| Reserve | 5% | about 95 | - | - |

How a head reads its own use: call `get_session` with no session id and read `usage.cost_usd`; the use of
this week is that number less the cost at the reset in the table. The reset is each Wednesday at 16:00 UTC.
The Head of Staff writes the new reset costs in this table after each reset. A share is a guide for the
order and the pace of the work. It is not a stop.
