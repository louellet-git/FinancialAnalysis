# Week study: state on 2026-11-21 19:35 UTC

Tickets: `T-20261121-sp100-week-attribution`, `T-20261121-premium-company-part`, `T-20261121-dcf-engine-integration`
(in progress); `T-20261121-rates-balance-sheet`, `T-20261121-evaluator-captive-finance` (backlog).

## Branches
- `claude/premium-series` (worktree `../wt-premium`): attribute (par curve, full bridge, stale price, columns beta1,
  beta2, dur_premium1, earnings_8k, the common and company split, the economics view), `fa dcf premium-series`, the
  documents A-027 to A-033. At the gate of Quality since a993e68 (code-reviewer, model-validator, document-reviewer):
  NO verdict yet. Local commits after a993e68 are NOT pushed (record, ticket, the pit test results, the reality evidence).
- `claude/attribution-reality` (worktree `../wt-reality`, from the local head of premium-series): `fa dcf baserates`,
  `attribute --top N --reality --prior-window`, `--curve auto` (the default), `premium-series --per-ticker` and
  `--windows-out`, the reader of the `fa macro view` file, documents A-034 to A-037. Pushed (38ab703 and later).
  NOT at a gate yet: send the request after the verdict on the first branch. 3924a05 does not compile (fixed by 38ab703).
- Plan: one pull request for all of it after both gates. Then the register rows of `assumptions.rs`.
- Paused: `claude/t1-revenue-band`, `claude/trade-level-backtest` (the state is on each ticket).

## Commands of the results (store 0340bc1 or later; binary of `../wt-reality`)
- `fa dcf attribute --tickers-file rust/mapcheck/universe/sp100.txt --from 2026-11-13 --to 2026-11-20 --cache <store>
  --prior-window` (S&P 100); add `--top 10 --reality --baserate-list rust/mapcheck/universe/sp500.txt
  --premium-history <series csv>` for the S&P 10.
- `fa dcf premium-series --tickers-file <list> --start 2019-01-04 --end 2026-11-20 --cache <store> --out F`.

## Numbers (preliminary, not reviewed): `analysis/priors/sp100_week_prelim_results_2026-11-21.md`, sections 1 to 8
- S&P 100 -9.22%, S&P 10 -9.96% (BRK-B in, WMT out). Premium first, S&P 100: rates +5.08, premium common -12.64,
  premium company +1.79, growth common -1.22, growth company -0.39, no model -1.94; common change +0.76 points.
- Two weeks (11-06 to 11-20): -7.01%, common change +0.13 points; the yields rose the week before (real 10-year +28 bp).
- List premium, S&P 100: 1.20% (11-13, the lowest of 2019-2026), 1.67% (11-20, 2nd percentile); without the
  technology sector 2.07% and 2.58%, the same path. Index dividend model: 1.64%, 24% of the months since 1871 are lower.
- Base rate (revenue 50B or more, 10 years): median 3.9%, 90th 10.0%, maximum 21.0%; implied: NVDA 29.3%, AVGO 28.2%,
  LLY 27.1%, META 22.8% (above every case), GOOG 17.5%, MSFT 16.1%, AAPL 15.0%, BRK-B 8.6%.
- Economics (a248803): a rate rise of about two quarter-point steps in a year is priced; "late reaction" is one fair
  reading, not proven. Rates on the balance sheet: small for the sums, material for 29 leveraged companies.

## Open
- Luc's go-ahead for the report; then validator, document reviewer, auditor; then the Head of Staff; then Luc.
- The premium-series run of record (S&P 500 from 2019-01-04) after the gate; send the commit and the switch week
  (2026-11-20) to Economics for its frozen test (`fa macro premium-test`); the pit tests on 3 dates passed.
- Technology: `claude/revenue-old-tags` (rerun the base rates after its merge); the daily collection decision.
- Board: add the `story` field to my tickets; rewrite two old tickets that name Python; read `EPICS.md`.
