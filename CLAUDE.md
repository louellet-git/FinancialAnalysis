# CLAUDE.md — orientation for agents

If your session title starts with `FA-L` or `FA-M`, you are a head of an area. Read
`chiefs/ORIENTATION.md` after this file. It says who you are and how the heads send messages
to each other.

Read this file first. Then read `README.md` (commands), `docs/ARCHITECTURE.md` (module
map), `docs/HIDDEN-ASSUMPTIONS.md` (known traps), `docs/TEAMS.md` (the agent teams) and
`docs/QUALITY-STANDARDS.md` (the standards and the gates).

## Purpose

Two persons use this repo (GitHub logins `louellet-git` and `merlan1996`) to make
investment decisions and to trade options. The repo has two parts:

1. **Valuation** (`rust/mapcheck/`, `rust/src/dcf/`, `analysis/<TICKER>/`). The
   Rust program `mapcheck` gets SEC EDGAR data and makes the standard input variables.
   The Rust program `fa` runs two Monte Carlo DCF engines and compares the value
   range with the market price.
2. **Options trading research** (`trading/`). A backtester and a paper trader for
   SPY/QQQ option strategies. All backtest option prices are model prices.

The repo gives analysis. It does not send orders to a broker.

## Where the status is

- Work now and next: the board, `PLANNING/board/` (rules in its `README.md`).
- Work without a ticket: `PLANNING/tickets.md`.
- No other file contains status. Do not add status text to this file or to `README.md`.

## Which source is correct when two sources disagree

1. **Engine code is correct.** The Rust engines `rust/src/dcf/driver.rs` and
   `rust/src/dcf/classic.rs` define the calculation. A number in a document that
   disagrees with a new engine run is old.
2. **`analysis/<TICKER>/driver_model.toml` is correct** before `DRIVERS.md` text.
3. **Recorded decisions stay.** `analysis/IBM/REVIEW.md` (F1 to F19) and
   `analysis/IBM/DRIVER-REVIEW.md` are closed decisions. To open one again, do a review
   of the same weight.
4. **`PLANNING/RULES.md` rules are mandatory** in their scope. To remove a rule, do a
   review of the same weight as the review that made the rule.
5. **`financial_analysis/standard_variables.toml` defines the input variables.** Models
   read the standard variable names only, not XBRL tag names and not `raw_extras`. To
   add a tag, add it to this file, with the same scope as its variable. The Rust program
   `rust/mapcheck` is the only code that makes the variables. After a change: build
   `rust/mapcheck`, then run `cargo test` for `rust/mapcheck` and for `rust/`.
6. **The IBM entries in `rust/src/dcf/scenarios.rs` are version 3 on purpose**
   (`docs/HIDDEN-ASSUMPTIONS.md` #13). The version 4 headline numbers come only from
   `fa dcf drivers IBM` (seed 7: $83.84 / $151.84 / $258.03).
7. **Published reports are frozen.** Do not change `analysis/IBM/report/*`. Write a new
   version.
8. `README.md` and `docs/` are for orientation only. `THOUGHTS` is a list of raw ideas.

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
- **The main language is Rust** (owner decision, 2026-11-19; scope set by the owner, 2026-11-20).
  Write in Rust the programs that calculate, trade or get data: the engines, the trading code, the
  data loaders and their tests. The repo has no Python code, and no part has a second copy in a
  different language. A different language is permitted where it fits the work better than Rust:
  SQL for a database, a shell command, the CI workflow file, the HTML of a report page, a small
  one-time script. If you are not sure, use Rust.
- **SEC contact.** The SEC contact is in `sec_contact.toml` (a project address, public on purpose).
  The environment variable `SEC_USER_AGENT` replaces it. Do not put a private contact in the repo.
- **Do not commit** `data/`, `__pycache__/`, `.env`, `vendor/`, or the files of a private
  review framework of a third party. This repo is public. The own agent teams and skills of
  this repo (`.claude/agents/`, `.claude/skills/`) are committed.
  Install a private framework in the user folder (`~/.claude/agents/`), not in the repo.
- **Model documents.** Each model has a model document (`docs/models/`) and each
  assumption has a record (`docs/assumptions/`). A change of an assumption value changes
  its record in the same pull request.
- **Token ledger.** Add a row to `PLANNING/token-ledger.md` for each run. A head adds the
  row to the `ledger.md` file of its own folder (`chiefs/ORIENTATION.md`, section 8).

## Review of a valuation report

A report that a person uses for a decision goes through four steps:

1. An analyst writes the report.
2. A skeptic attacks the substance and reproduces each number with the engine. Repeat
   until the skeptic accepts.
3. A reviewer grades the content and the format. Repeat until it passes.
4. An auditor traces each claim to a source and calculates each number again.

The four roles are agents of this repo: `equity-analyst`, `model-validator`,
`document-reviewer` and `auditor` (`.claude/agents/`). The flow is the skill
`valuation-report-review`. Luc approves each report for a new ticker (`PLANNING/reusable-dcf-vision.md`,
L5).

A first pass (for example `analysis/JNJ/FIRST-PASS.md`) did not go through these steps.
Do not use a first pass alone for a trade.

## Commands

```bash
cargo test --manifest-path rust/mapcheck/Cargo.toml   # 97 Rust tests, no network
cargo test --manifest-path rust/Cargo.toml --locked   # 572 Rust tests (554 + 17 + 1), no network
cargo build --release --manifest-path rust/mapcheck/Cargo.toml   # the Rust program (necessary)
rust/mapcheck/target/release/mapcheck fetch --submissions IBM   # SEC 8-K item 2.02 list into data/raw/IBM/sec_8k_earnings.json
rust/target/release/fa estimates get IBM; rust/target/release/fa estimates show IBM   # consensus, targets, surprises (Nasdaq site, no key)
rust/target/release/fa earnings calendar IBM; rust/target/release/fa earnings dates IBM   # next report date; past dates against the SEC list
rust/mapcheck/target/release/mapcheck values IBM          # each input value with its source
rust/mapcheck/target/release/mapcheck IBM --detail        # mapping checks
rust/mapcheck/target/release/mapcheck filing IBM --focus  # capex, depreciation and debt lines of the 10-K
cargo build --release --manifest-path rust/Cargo.toml             # the fa command
rust/target/release/fa import IBM             # SEC data -> data/processed/IBM.json
rust/target/release/fa analyze IBM            # import + market data -> xlsx (nine tabs)
rust/target/release/fa verdict IBM --price 228.53   # value range against a price (add --rf-date today --rating A-)
rust/target/release/fa dcf drivers IBM        # version 4 headline numbers; options for each analysis (fa dcf --help)
rust/target/release/fa dcf auto IBM --cache data/raw --explain   # automatic path model of a ticker (a screen, not a reviewed valuation)
rust/target/release/fa dcf screen --tickers-file rust/mapcheck/universe/sp500.txt --cache data/raw   # one row for each ticker
rust/target/release/fa dcf pit --cache data/raw   # point-in-time test of the automatic model
rust/target/release/fa dcf returns --cache data/raw   # the automatic model value against the later stock returns (a measurement)
rust/target/release/fa dcf scenarios IBM      # classic engine, bear/base/bull (version 3 inputs)
rust/target/release/fa backtest IBM           # point-in-time vintage backtest of revenue and FCF
rust/target/release/fa report IBM             # one-page HTML report: data/processed/IBM_dcf.html (plotly.js from the CDN)
rust/target/release/fa rates                  # Treasury curves, SOFR, EFFR, inflation (also: fa bonds)
rust/target/release/fa sync --status IBM      # what is old, for each data set (no request); also: fa sync --dry-run
rust/target/release/fa sync IBM MSFT          # bring the old data sets up to date (one queue for each host; --only, --skip, --force, --max-minutes)
rust/target/release/fa trade fetch            # option bots: download the bars (Yahoo) into the cache
rust/target/release/fa trade backtest         # option bots, leaderboard
rust/target/release/fa trade paper            # option bots: one paper-trade step (live Cboe chain)
```

All commands: `README.md`.
