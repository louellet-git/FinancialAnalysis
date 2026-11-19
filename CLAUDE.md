# CLAUDE.md — orientation for agents

Read this file first. Then read `README.md` (commands), `docs/ARCHITECTURE.md` (module
map) and `docs/HIDDEN-ASSUMPTIONS.md` (known traps).

## Purpose

Two persons use this repo (GitHub logins `louellet-git` and `merlan1996`) to make
investment decisions and to trade options. The repo has two parts:

1. **Valuation** (`financial_analysis/`, `analysis/<TICKER>/`). The pipeline imports
   SEC EDGAR data, runs two Monte Carlo DCF engines, and compares the value range with
   the market price.
2. **Options trading research** (`trading/`). A backtester and a paper trader for
   SPY/QQQ option strategies. All backtest option prices are model prices.

The repo gives analysis. It does not send orders to a broker.

## Where the status is

- Work now and next: the board, `PLANNING/board/` (rules in its `README.md`).
- Work without a ticket: `PLANNING/tickets.md`.
- No other file contains status. Do not add status text to this file or to `README.md`.

## Which source is correct when two sources disagree

1. **Engine code is correct.** `financial_analysis/driver_dcf.py` and
   `financial_analysis/dcf.py` define the calculation. A number in a document that
   disagrees with a new engine run is old.
2. **`analysis/<TICKER>/driver_model.py` is correct** before `DRIVERS.md` text.
3. **Recorded decisions stay.** `analysis/IBM/REVIEW.md` (F1 to F19) and
   `analysis/IBM/DRIVER-REVIEW.md` are closed decisions. To open one again, do a review
   of the same weight.
4. **`PLANNING/RULES.md` rules are mandatory** in their scope. To remove a rule, do a
   review of the same weight as the review that made the rule.
5. **The IBM entries in `financial_analysis/scenarios.py` are version 3 on purpose**
   (`docs/HIDDEN-ASSUMPTIONS.md` #13). The version 4 headline numbers come only from
   `python -m analysis.IBM.dcf_drivers_run`.
6. **Published reports are frozen.** Do not change `analysis/IBM/report/*`. Write a new
   version.
7. `README.md` and `docs/` are for orientation only. `THOUGHTS` is a list of raw ideas.

## Work rules

- **Language.** Write each new document, and each document that you change, in
  ASD-STE100 Simplified Technical English. This is the only document standard. Frozen
  documents and review records keep their original text.
- **Ticket first.** Before you start, set a board ticket to `in-progress` with your
  login and branch, and push that change.
- **One branch for one ticket.** Start it from `origin/main`. Merge through a pull
  request only.
- **Parallel agents use git worktrees** (`git worktree add ../wt-<name> -b <branch>
  origin/main`). Do not change the branch of a checkout that other agents use.
- **Subagents do not commit.** The orchestrator examines the work and makes the commit.
- **Smoke test.** A ticket is complete only after the real command runs and the next
  step can use the output.
- **No bloat.** Add one small part, show that it gives value, then add the next part.
- **Do not commit** `data/`, `__pycache__/`, `.env`, or private review-framework files
  (`.claude/agents/`, `vendor/`). This repo is public.
- **Token ledger.** Add a row to `PLANNING/token-ledger.md` for each run.

## Review of a valuation report

A report that a person uses for a decision goes through four steps:

1. An analyst writes the report.
2. A skeptic attacks the substance and reproduces each number with the engine. Repeat
   until the skeptic accepts.
3. A reviewer grades the content and the format. Repeat until it passes.
4. An auditor traces each claim to a source and calculates each number again.

The agent definitions for these roles are in a private framework. Each person installs
them locally; they are not in this repo. If they are not installed, do the four steps
manually. Luc approves each report for a new ticker (`PLANNING/reusable-dcf-vision.md`,
L5).

A first pass (for example `analysis/JNJ/FIRST-PASS.md`) did not go through these steps.
Do not use a first pass alone for a trade.

## Commands

```bash
pip install -r requirements.txt -r requirements-dev.txt
python -m pytest                              # 390 tests, no network
python -m scripts.analyze IBM                 # import + market data -> xlsx
python -m scripts.verdict IBM --price 228.53  # value range against a price
python -m scripts.trade_backtest              # option bots, leaderboard
```

All commands: `README.md`.
