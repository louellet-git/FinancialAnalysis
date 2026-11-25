# Week study: state on 2026-11-25 (evening UTC)

Tickets: `T-20261121-sp100-week-attribution`, `T-20261121-premium-company-part`, `T-20261121-dcf-engine-integration`
(in progress); `T-20261121-rates-balance-sheet`, `T-20261121-evaluator-captive-finance` (backlog).

## Branches
- MERGED to main: `claude/premium-series` (pull request 190, 7e88cfe) and `claude/attribution-reality` (pull request
  199, 3fc1acc). Main has `fa dcf attribute` (par curve, the default curve auto, full bridge, stale price, the common
  and company split, the economics view, `--top`, `--reality`, `--prior-window`), `fa dcf baserates`,
  `fa dcf premium-series`, documents A-027 to A-037. Both gates of Quality passed (PASS-WITH-NOTES).
- Release check from now on: `fa selftest golden` (the tool of Quant Research; data store of the case as data/raw,
  FA_REPO = the repo root). Pull request 190 made its case `dcf_attribute_panel32` fail on main (corrected: 195).
- In progress: `claude/t1-revenue-band` (the rule `band_q` is chosen; it waits for the statistics gate, then one score run).
  Paused: `claude/trade-level-backtest`. The state is on each ticket.

## The three pieces for the person (on main; PDFs in `analysis/priors/pdf/`)
1. `sp10_week_page_v2_2026-11-21`: the days, the stocks, the press reason and the test for each day (Economics).
2. `sp10_week_valuation_page_2026-11-21`: the valuation blocks, actual against implied; a row for each stock.
3. `sp500_chips_week_page_2026-11-21`: the 16 chip companies of the S&P 500 against their expected fall.
Each is one page. Quality passed all three. They are with the Head of Staff, who gives them to the person.
Style: `chiefs/louellet-git/staff/memory/owner-report-style.md`. PDF method: pandoc to HTML with `pdf/style.css`, then
a headless Chromium print (Letter); check the page count after each change.

## Numbers (store 0340bc1)
- Runs of record: `analysis/priors/data/week_2026-11-20_record_main-7e88cfe/` (the bridge) and `..._main-3fc1acc/` (base rates, reality).
  S&P 100 -9.22%, S&P 10 -9.96%. S&P 10 blocks, premium first: facts 0.00, time +0.08, rates +5.81, premium common
  -15.89, premium company +3.09, growth common -1.01, growth company -0.65, no model -1.39.
- Base rates of record on main e2557b6, after the revenue fix (50 companies of 50B or more, median 3.7%, maximum
  21.0%). The list premium series is a run of record with the label "parameters in-sample" (1.20% and 1.67%; folder
  `premium_series_sp100_2019_record_main-e2557b6`). NOT of record: the deviations and the fall by beta (one-time scripts in
  `data/week_2026-11-20_chips/`; design R6 puts them in the engine).
- Deviations against the equal-weight market: NVDA -2.7, AAPL +1.0, META -1.0; NVDA against the other chips 1.1.
- Chips of the S&P 500: -16.4% on average against -9.4% (beta) and -4.3% (value); r -0.63 with the needed growth.
- Economics (note `analysis/macro/2026-11-21-chip-fall-economics-view.md`, branch econ/rate-rise-question): one reason
  through the week, a doubt about the AI build-out and its debt; a rate rise is not supported.

## Open
- The review of the person on the three pieces; then the S&P 100 part (the same tests on 100 companies). Not before.
- Done after the pieces (all on main): the point-in-time tests of the series (`data/premium_series_pit_main/`), the
  series runs of record of the S&P 100 and S&P 500 lists with the label "parameters in-sample", the fix of the stale
  rule in the series bridge (pull request 213), and the S&P 500 series with the bridge for the Head of Economics
  (`data/premium_series_sp500_2019_bridge_record_main-6c59388/`).
- Still open on `T-20261121-premium-company-part`: a parameter table cut before 2019; a truncation test of the base
  rates in the suite; the long sentences of premium-series.md; the register rows of the records A-027 to A-037.
- For two pages of the Head of Economics (2026-11-23): `analysis/priors/ant_filing_numbers_2026-11-23.md` (numbers of
  the Anthropic prospectus, with the disclosure; in its report) and the engine result for URA (only LEU has implied
  numbers; store a798b9b). Story E3.S5 on the board: the miner design ticket, the fund ticket, the R6 note.
- Technology: SWKS has no close of 2026-11-20 at the source; the BWXT mapping fails; UUUU, SMR, DNN, OKLO have no prices.
