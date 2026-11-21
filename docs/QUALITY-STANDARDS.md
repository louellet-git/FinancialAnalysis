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
| CRIT | A wrong number or rule in a result that a person can use; a secret in a file; code that sends an order; an input set from the price; a fabricated source | Merge and publication |
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
| Model change (DCF, rates, volatility) | the quant of the area | `model-validator`, `code-reviewer` | Numbers reproduced; no fit to the price; records current |
| Strategy or backtest | `strategy-researcher` | `model-validator` | Costs, out of sample, model prices, no orders |
| Investigation | the researcher who owns it | `auditor` | Numbers calculated again |
| Valuation report | `equity-analyst` | `model-validator`, `document-reviewer`, `auditor` | The four steps, in order |
| Document or model record | the author, `model-documenter` | `document-reviewer` | Purpose, evidence, STE |

A small change (a typing error, a count, a ticket, a ledger row) has no gate: the
orchestrator examines it. `test-engineer` and `risk-compliance-officer` are not in the
gates. The orchestrator adds one of them when the work needs it (`docs/TEAMS.md`, "Main
flows").

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

No order code. No secret in a file, a commit or a chat (skill `secrets-and-privacy`). No
change to `analysis/IBM/report/*`, `REVIEW.md` or `DRIVER-REVIEW.md`. The engines, the trading code and the data loaders are in Rust (`CLAUDE.md`, "Work rules").
A subagent does not commit. A client name is honest; no program imitates a browser.
