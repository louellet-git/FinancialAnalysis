# Week study: state on 2026-11-21 21:10 UTC

Tickets: `T-20261121-sp100-week-attribution`, `T-20261121-premium-company-part`, `T-20261121-dcf-engine-integration`
(in progress); `T-20261121-rates-balance-sheet`, `T-20261121-evaluator-captive-finance` (backlog).

## Branches
- `claude/premium-series`: MERGED (pull request 190, main 7e88cfe). It has attribute (par curve, full bridge, stale
  price, the common and company split, the economics view) and `fa dcf premium-series`, documents A-027 to A-033.
- `claude/attribution-reality` (worktree `../wt-reality`): `fa dcf baserates`, `attribute --top N --reality
  --prior-window`, `--curve auto` (the default), `premium-series --per-ticker` and `--windows-out`, the reader of the
  `fa macro view` file, documents A-034 to A-037, the code notes of gate cycle 1. At the gate of Quality since a0e2480:
  NO verdict yet. After the verdict: merge main into it, the release checks, one pull request.
- Paused: `claude/t1-revenue-band`, `claude/trade-level-backtest` (the state is on each ticket).

## The three pieces for the person (on `claude/attribution-reality`, f201a55; PDFs in `analysis/priors/pdf/`)
1. `sp10_week_page_v2_2026-11-21`: the days, the stocks, the press reason and the test for each day (Economics).
2. `sp10_week_valuation_page_2026-11-21`: the valuation blocks, actual against implied; a row for each stock.
3. `sp500_chips_week_page_2026-11-21`: the 16 chip companies of the S&P 500 against their expected fall.
Each is one page. Quality passed all three. They are with the Head of Staff, who gives them to the person.
Style: `chiefs/louellet-git/staff/memory/owner-report-style.md`. PDF method: pandoc to HTML with `pdf/style.css`, then
a headless Chromium print (Letter); check the page count after each change.

## Numbers (store 0340bc1)
- Run of record on main 7e88cfe: `analysis/priors/data/week_2026-11-20_record_main-7e88cfe/` (README has the command).
  S&P 100 -9.22%, S&P 10 -9.96%. S&P 10 blocks, premium first: facts 0.00, time +0.08, rates +5.81, premium common
  -15.89, premium company +3.09, growth common -1.01, growth company -0.65, no model -1.39.
- Not of record: the base rates (`data/baserates_sp500_2026-11-20/`: 46 companies of 50B or more, median 3.9%, maximum
  21.0%), the list premium series (1.20% to 1.67%), the deviations and the expected fall by beta (one-time scripts in
  `data/week_2026-11-20_chips/`; design R6 puts them in the engine).
- Deviations against the equal-weight market: NVDA -2.7, AAPL +1.0, META -1.0; NVDA against the other chips 1.1.
- Chips of the S&P 500: -16.4% on average against -9.4% (beta) and -4.3% (value); r -0.63 with the needed growth.
- Economics (note `analysis/macro/2026-11-21-chip-fall-economics-view.md`, branch econ/rate-rise-question): one reason
  through the week, a doubt about the AI build-out and its debt; a rate rise is not supported.

## Open
- The review of the person on the three pieces; then the S&P 100 part (the same tests on 100 companies).
- The gate verdict on `claude/attribution-reality`; then the pull request and the base rates of record.
- The series run of record: a review of the pit results of 61fe8de first, and a cut parameter table.
- Technology: SWKS has no close of 2026-11-20 at the source (next try 2026-11-26); `claude/revenue-old-tags`.
- Lesson of the audit: a shell sum goes into a file only after a second calculation (three sums were wrong).
