---
id: T-20261121-split-basis-fix
title: Put each past price on the share basis of its SEC vintage (split look-ahead), and add a live split guard
area: engine
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: fundamental
order: 0.01
branch: claude/dcf-engine-backtest
created: 2026-11-21
acceptance: A backtest or a study that compares a past model value per share with a past price uses a price on the share basis of the vintage of that date; a test multiplies all prices before a later date by k and shows that no earlier decision changes; a live split guard stops a pick whose share count of the last 10-K and the current share basis differ by a split; the runs that the defect touched run again once, as new logged variants, after a test file written before them.
---
Found by the strategy gate (Head of Quality and Risk, 2026-11-21, CRIT F1). The price files of the store are
split-adjusted to today (`prices.meta.json`: "adjusted": "splits"). The model value per share uses the share
count of each SEC vintage. So a company that split after the date gets a value per share that is too high by the
split factor. Example: BKNG closes at 72.94 on 2018-01-05 in the store (it traded at about 1,800); 25:1 split
filed 2026-04-28. The later splitters are the winners of 2018 to 2026, so the defect made the value rank look good.

Touched (invalid until the reruns): the variant log runs 1 to 14 and R1, R2
(`analysis/priors/dcf_engine_backtest_2026-11-21.md`); the blind projection T2 to T4
(`analysis/priors/blind_projection_test_2026-11-21.md`); `fa dcf returns` and its recorded files
(`analysis/priors/auto_returns_*`); the value-rank backtest of pull request 104. Not touched: `fa dcf pit` (no
price), momentum (price only), the market-level test (an index), the NVDA driver model (its own share count; the
10:1 split of 2024 is before its last 10-K).

Plan:
1. Data: a split table (ticker, ex-date, ratio) or unadjusted closes, from the Head of Technology (request
   2026-11-21). No guess from the share-count steps for a result of record.
2. Code: one function that gives the factor from a date to today; the strategy and `fa dcf returns` divide the
   price by it (or multiply the shares). The k-test. A live guard: the current share basis against the last 10-K.
3. A new test file (before the reruns), with the other gate findings: a no-rebalance mode (F2), the cut at each
   rebalance Friday (F3), a deflated t and a confirmation window that the design did not use (F4).

## Rerun test (written 2026-11-21 before any rerun on corrected prices)
Data store `f6e66fc` (the prices are the same as `1804f48`, plus `_corporate/splits.csv`). Table: cut at 2016
(SHA-256 without the `command =` line: 41443b66...). List: the 431 scored tickers, less the 12 `mismatch` tickers
and AA. Seed 7, 300 paths, 10 bp. Each run once, as new variants 15 and later of the variant log.
1. Checks first: the k-test passes (k = 2, 25, 0.1); a truncation test at 20 rebalance Fridays (every 23rd from
   2018-01-05), with the prices and the curve cut after that Friday and `--end` one trading day later, gives
   the same weights at that Friday (F3).
2. Runs: B (top fifth, quarterly), D (sector-neutral, quarterly), R1 (A-009, weekly) in two modes, R1-hold (no
   trade back to 1/39; A-009 as written; the PRIMARY run) and R1-reb (the old mode), R2 (bottom fifth), and
   T2 to T4 (`fa portfolio calibrate`, the same command as before).
3. Reading (fixed now): the design sample 2018-2026 has been used, so these runs DESCRIBE; they do not confirm
   (F4). Each states its t and a t deflated for the count of value-signal runs (Bonferroni over all variants,
   15 and more). The weekly trades start on paper only if T1 passes (see the blind projection ticket) AND T2 to T4
   pass on corrected prices. The paper weeks are the first window that the design did not use: they confirm or
   not. Survivorship cannot be bounded with the store (F7).
4. Additions (2026-11-21, before any rerun; comments of the Head of Quality and Risk):
   - **Decision rule for A-009.** A-009 is the rule of the weekly proposals only if R1-hold gives: a mean yearly
     excess over the equal weight above 0, a raw t of 2.0 or more, the excess above 0 in at least 7 of 9 years,
     and T2 to T4 pass on corrected prices. The deflated t is reported. If R1-hold fails, no weekly proposal uses
     A-009, and the head tells Luc.
   - **The paper weeks test the process, not the edge.** 4 weeks with a 13-week hold close no position. The first
     real confirmation is a later fresh window of at least one year, or older prices if the store gets them.
   - **The 13 excluded tickers** (12 `mismatch` and AA): only DD is in the scored list, and the old R1 held none of
     them (0 of 17,511 holding rows; `awk -F, '$2==T' R1_h.csv` for each ticker). The exclusion cannot drive the
     result. If the gate of the split table changes a row, the reruns wait for that change.
5. The split table of record is store `c422939` (after its gate review), not `f6e66fc` (2026-11-21, before any rerun).
   The exclude list is replaced: the tickers with a `mismatch` row in `c422939` (GSAT) and the tickers with an
   estimated ex-date (`ex_date_estimated` = true) on or after 2016-01-01, because that date can be up to about 250
   days late, near the filings that the backtest uses: of the scored list, BALL, EW, EXE, GPN and HWM. A row with
   `status` = pending (SNDK) is after its last price date and is not counted (the `to` rule).
