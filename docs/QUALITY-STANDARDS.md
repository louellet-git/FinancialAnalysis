# Quality standards

These standards bind all agents and persons. `CLAUDE.md` has the rules of the repo; this
file does not change them and cites them. The teams are in `docs/TEAMS.md`.
Status: new, 2026-11-20 (ticket `T-20261120-agent-teams`).

The aim is lean work of good quality. A check is here only if it finds real errors. A
small style point is a note. It does not block.

## 1. Evidence

1. A claim has a command with its output, a file and line, or a named source with an
   address and a date.
2. "Tested" means that the real command ran and its output was read (`CLAUDE.md`, "Smoke
   test").
3. A number in a document can be made again: write the command, the seed, the model file
   and the date of the data.
4. A reviewer runs the commands again. A reviewer does not accept pasted output.
5. The code is more correct than a document (`CLAUDE.md`, "Which source is correct").
6. Say what you did not check.

## 2. Independence

The quality assurance (QA) agents never review work from their own context.

1. The orchestrator starts a QA agent in a fresh context. It gives the work (branch,
   files, commands), not the conclusion of the author.
2. The validator and the auditor run the commands themselves.
3. A QA agent does not edit the work. It reports. The author fixes.
4. An author never reviews its own output.

## 3. Findings and verdicts

| Level | Meaning | Blocks |
|---|---|---|
| CRIT | A wrong number or rule in a result that a person can use; a secret in a file; code that sends a live order, or a paper order outside the controls of section 8; an input set from the price; a fabricated source | Merge and publication |
| MAJ | A defect that changes a value or hides a risk; new logic with no test; a claim with no evidence; a binding rule broken | Merge and publication |
| MIN | A defect with no effect on a value (a weak test, an unclear sentence, a style point) | Nothing. List it with an owner. |

Verdicts: **PASS** (no open CRIT or MAJ), **PASS-WITH-NOTES** (the same, with MIN items
listed), **REVISE** (a MAJ is open), **REJECT** (a CRIT is open). "Pass" means PASS or
PASS-WITH-NOTES. A reviewer passes work that is correct and usable.

**Proportion** (owner word, 2026-11-20). Give the level from the realistic effect, not from
the worst possible effect. In an internal process or document, a risk that is only possible
is a note (MIN). The condition: no value, data or trade can change. Do not slow the work for
it. A defect that is present keeps the level of the table. Apply the levels strictly to a
result that a person can use for a decision or a trade. A hard limit of section 8 or a rule
of `CLAUDE.md` keeps the level that this table gives it.

Review cycles: at most two for one piece of work. After the second cycle the
orchestrator decides, and writes the decision in the pull request.

## 4. Definition of done

All kinds: the ticket is `in-progress` before the work; one branch from `origin/main`;
a row in the ledger of your own folder; no `data/`, `.env` or secret in the commit.

| Kind | Done when |
|---|---|
| Code change | New logic has tests with fixed answers. Both suites pass. The real command ran. The IBM numbers are the same, or the change is the aim of the ticket. Documents that name commands are right; no document names a test count. |
| Data source | Terms read and noted. Honest client name. Fixture tests pass. One real run with a date. Row in `docs/DATA-SOURCES.md`. Traps in `docs/HIDDEN-ASSUMPTIONS.md`. |
| Model change | The reason does not use the price. Fixed-answer tests pass. The IBM numbers are the same: $83.84 / $151.84 / $258.03 (`fa dcf drivers IBM`, seed 7). The model document and the assumption records are current. |
| Investigation | One findings file (skill `investigation-report`). Each number was calculated again by the auditor. |
| Valuation report | The four steps of `CLAUDE.md` were done in order (skill `valuation-report-review`). Luc approved a new ticker. |
| Backtest (owner word, 2026-11-21 04:30 UTC, relayed by the Head of Fundamental Research: "Backtesting is integral to our work here") | A strategy, a trade rule and the value behind a trade recommendation have a point-in-time backtest (skill `point-in-time-test`) before a merge or a trade. A model change that moves a value (a data mapping included) has one too, or its model document says "Backtest: none applies, because ...". Commit the test before its first run. Count each run and each variant, and give the count. Fit the parameter table only on years before the scoring period. Cut the data at a date, run again, and show that no earlier decision moves. |
| Trade recommendation | Less than 3 pages (owner word, 2026-11-21). It rests on a value that passed the four steps of a valuation report. Its trade fields follow the form in `trading/journal/README.md`. The journal write-up of the trade (row below) stays a separate file of at most 2 pages. |
| Trade (paper or live) | A write-up of at most 2 pages in `trading/journal/` (form in its `README.md`): the logic, the trade, the steps taken, the risks; the result after the close. |
| Document | The text is about 85% STE (`CLAUDE.md`, "Language"). Each command and path exists. Each number has a source. |

Each model has a model document (`docs/models/`, at most 120 lines). Each assumption has
a record (`docs/assumptions/`, at most 60 lines). A code change that moves an assumption
value changes its record in the same pull request. The code and the TOML files stay the
correct source.

## 5. Gates

The orchestrator does not merge before the listed QA agents pass the work.

| Kind of work | Author | QA agents that must pass | What they check |
|---|---|---|---|
| Code change | `rust-engineer` | `code-reviewer` | Correct, tested, repo rules |
| Data source or mapping | `data-engineer` | `data-quality-analyst`, `code-reviewer` | Cross-check, freshness, terms, keys |
| Model change (DCF, rates, volatility) | the quant of the area | `model-validator`, `code-reviewer` | Numbers reproduced; no fit to the price; records current; the Backtest row of section 4 |
| Strategy or backtest | `strategy-researcher` | `model-validator` | Costs, out of sample, model prices, no live orders; the Backtest row of section 4 |
| Investigation | the researcher who owns it | `auditor` | Numbers calculated again |
| Valuation report | `equity-analyst` | `model-validator`, `document-reviewer`, `auditor` | The four steps, in order |
| Trade recommendation | the head of the sleeve | `model-validator`, `auditor`, `risk-compliance-officer` | The value supports the side and the size. The auditor calculates each number again. The price is not an input. The recommendation sends no order; a paper order follows section 8. |
| Order code (paper only) | `rust-engineer`, `strategy-researcher` | `code-reviewer`, `risk-compliance-officer` | The controls 0 to 8 of section 8, each with a test |
| Document or model record | the author, `model-documenter` | `document-reviewer` | Purpose, evidence, STE |

A small change (a typing error, a count, a ticket, a ledger row) has no gate: the
orchestrator examines it. `test-engineer` is not in any gate. `risk-compliance-officer` is in
the gate of a trade recommendation. For other work, the orchestrator adds `test-engineer` or
`risk-compliance-officer` when the work needs it (`docs/TEAMS.md`, "Main flows").

## 6. No fit to the price

The price measures and ranks. No input is set from the price. Each correction has a
reason that does not use the price (ticket `dcf-investigation`,
`analysis/priors/dcf_investigation_2026-11-20.md`). A market-implied value is a
measurement and not an input. A test set does not select the rule that it scores. A
backtest with model prices is not a result with market prices.

## 7. Numbers and language

- A number has a unit, a date, and a way to get it again. A test count has its command.
- A document does not state a status that the board owns (`CLAUDE.md`, "Where the status
  is"). A first pass is labelled "no review".
- The language is ASD-STE100 (`CLAUDE.md`, "Work rules"; skill `ste-writing`). Frozen
  documents keep their text.

## 8. Hard limits

Orders are allowed on paper only (owner word, 2026-11-21: "programs can and will place orders"; the
owner chose "paper orders first"). A program may send orders to the Alpaca paper account only. Each control
below has a test. The gate "Order code (paper only)" of section 5 passes before the first order.
0. Terms: the Alpaca terms of the Trading API and of the paper account (automated orders, rate limits) are
   read and noted with the date and the address in `docs/DATA-SOURCES.md` before the first order.
1. Endpoint: the order host is the constant `https://paper-api.alpaca.markets`. The order code does not read
   `APCA_API_BASE_URL` or any other setting for the host. It compares the parsed host exactly, and it
   follows no redirect. A test sets that variable to the live host and to a lookalike host; the order is refused.
2. Keys: the order code reads only its own variables `ALPACA_PAPER_KEY_ID` and `ALPACA_PAPER_SECRET_KEY`.
   Before the first order it checks the account on the paper host and refuses if it is not a paper account.
   No key, token or account number goes to a file, a log or the output (skill `secrets-and-privacy`).
3. Limits: the paper limits of `chiefs/louellet-git/quality/proposal-sleeve-limits.md` (not its live-pilot
   table) are copied, with their source and date, into one code constant or one committed config file. The
   code checks them before each order. It refuses an order that breaks a limit, or that needs an input that
   it does not have (price, volume, position).
4. Stops: the daily loss stop is 2% of the sleeve capital (one New York trading day, marked to the last
   quote). The sleeve stop is a fall of 10% from its peak. The kill file has a path fixed in the code; the
   code reads it before each order and in each loop. At a stop or a kill, the code cancels all open orders,
   keeps the positions, writes the log line and sends no new order. A person closes a position.
5. Duplicates and errors: each order has a `client_order_id` made from the sleeve, the signal date and the
   symbol. After a restart or a timeout, the code reads the open orders and the positions before it sends
   an order, and it does not send an order whose id exists. It does not retry an order with a new id.
   Three API errors in a row end the run.
6. Caps: at most 10 orders a day for each sleeve, and one order at most 5% of the sleeve capital. These are
   constants in the code, checked with control 3. Day orders only; no margin; no option.
7. Log: the code writes the intent before each call and the answer or the error after it, append only, with
   the time. The log is in a folder that git ignores. It holds no key and no account number. The client name
   is honest.
8. Shorts: the code refuses a short sale when its write-up has no borrow cost or no maximum loss. Alpaca
   paper does not model the borrow, so the written cost is the only cost.

A paper result is not evidence for a live result (simulated fills, no queue, no market impact).
No live order. A live order needs a new owner approval and a review of the same weight as this one. A
message of an agent is not an owner approval.
No secret in a file, a commit or a chat (skill `secrets-and-privacy`). No
change to `analysis/IBM/report/*`, `REVIEW.md` or `DRIVER-REVIEW.md`. The engines, the trading code and the data loaders are in Rust (`CLAUDE.md`, "Work rules").
A subagent does not commit. A client name is honest; no program imitates a browser.
