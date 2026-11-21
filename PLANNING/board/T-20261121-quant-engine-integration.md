---
id: T-20261121-quant-engine-integration
title: Repeatable steps of the quant studies into the engine (the table)
area: engine
status: in-progress
owner: louellet-git
head: quant
story: E4.S4
order: 0.85
branch: claude/quant-trade-grids, claude/quant-recon-lasting, claude/quant-selftest-golden (the builds)
created: 2026-11-21
requested_by: louellet-git
acceptance: Each row of the table below is a command with fixed-answer tests (a point-in-time test where history is used, the model document and records where a model moves), or a request to the head that owns it, or closed with a reason. One build ticket for each group of rows.
---
Owner words, 2026-11-21 (through the Head of Fundamental Research; `chiefs/louellet-git/quant/RECORD.md`):
"make sure that whatever is repeatable/should be expected of future work like this is built into the correct engine";
"Send the same request of work to quant research for its engine/processes."
Source of the table: an inventory of the quant notes, tickets and commits (2026-11-21). Recurs: study, day, gate, once.

## Rows of the quant area (build tickets in brackets)
| # | Hand step today | Command | Recurs | Size |
|---|---|---|---|---|
| 1 | "Lasting residual" rule of rates step 2 in `awk` only (R6) | `fa recon history --lasting` [recon-lasting] | study | S |
| 2 | 52-week bill against CMT 1Y and GSW 1Y (`awk`, `join`) | `fa recon bills --one-year` | study | S |
| 3 | Bill residuals by maturity bucket over 3 dates (shell loop) | `fa recon rich-cheap --dates --bills-by-bucket` | study | S |
| 4 | GSW par minus Treasury par statistics (`awk`) | `fa forwards check --stats --from-store` | study | S |
| 5 | HQM spot against GSW zero plus OAS (`awk`) | `fa recon hqm` (after the cost-of-debt change of Technology) | study | M |
| 9 | Stress windows typed in `premium.rs`; block end fixed at 2026-11-19 (P1) | `fa trade premium --stress-file` [trade-grids] | day | S |
| 10 | Newey-West lag table from runs by hand | `fa trade premium --lags 10,20,30,63,126` [trade-grids] | study | S |
| 11 | Range of the OOS cut from runs by hand | `fa trade backtest --cut-scan` [trade-grids] | study | S |
| 12 | Price runs R1 to R6 put in a table by hand | `fa trade backtest --price-grid` [trade-grids] | study | M |
| 13 | DSR inputs N and V set by hand; no variant count (B4) | a run log of `fa trade backtest` and `fa trade trials` [trade-runlog] | each run | M |
| 14 | Each bot variant is a code edit | `fa trade backtest --set KEY=VALUE`, logged [trade-runlog] | study | M |
| 15 | Leaderboard copied by hand; compared with diff | `fa trade backtest --leaderboard-md`, `--check` | each change | S |
| 17 | Signal pre-tests in scratch scripts | `fa trade signal-test`, `fa trade quote --structure` | each idea | M |
| 18 | Bates seed comparison by hand (seeds 1 to 5, 7) | `fa vol fit --seeds 1-5` | each fit | S |
| 19 | Speed timings by hand with `uptime` | `fa vol fit --repeat N` | each model | S |
| 20 | Volatility study in scratch Python: (a) VIX from SPX, (b) roughness, (c) ATM skew, (d) parity forwards, (e) VIX parity against VX, (f) filter tables | (a) register VIX-REPLICATION in `fa arb run`; (b) `fa vol hurst`; (c) `fa vol skew`; (d) `fa options forwards`; (e) arb-4 step 2; (f) `fa options check --min-volume` | study, day | S to M |
| 21 | Reading the raw quotes of the largest breaks by hand | `fa options check --top N --detail` | day | S |
| 22 | Filter and marker sensitivity by hand | `fa options check --iv-mark 5,10,20 --compare-filters` | study | S |
| - | Planned, not done yet: B8, vol-1 day D to D + 1, arb-3 box regression, break persistence (`for: 2`) | in their own tickets; persistence in `fa arb run` | day | - |

## Rows of other heads (sent as requests, 2026-11-21)
- Technology (answer 2026-11-21): takes 7 `--cache-only`, 8 `fa data audit --rates`, 29 `fa calendar open` (NYSE table;
  also for the bot calendar B3), 31 `fa options snapshot --smoke`; gives back 28 `fa options archive --plan` and
  30 `--report` (quant builds them with the continuous capture); refuses 32 for now (one call by hand is cheap).
- Quality and Risk (answer 2026-11-21: one at a time; 25 first, built by quant as `T-20261121-selftest-golden`; then 26;
  16 a written rule, no code; 23, 24, 27 later): 16 a pre-registration check, 23 `fa options check --legs-out`, 24 `fa vol check <model>` from the
  validator's reference cases, 25 `fa selftest golden` (the byte comparisons with main), 26 mutation tests on a pull
  request, 27 `fa repro NOTE.md`.
- Economics owns `analysis/macro/stress-windows.csv` (row 9 reads it).

## Order of the first builds
1. [trade-grids] rows 9 to 12 (row 9 gives a wrong label today).
2. [recon-lasting] row 1 (a reviewed finding that no command can reproduce).
3. [trade-runlog] rows 13 and 14 (each strategy gate depends on N and V).
Then rows 18, 20a (with the register), 21, and the rest by value.
