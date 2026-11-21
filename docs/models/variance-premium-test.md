# Model: variance premium test (`fa trade premium`)

Updated: 2026-11-21. Commit: not yet made.

## Purpose
`fa trade premium` measures the implied variance of the VIX and of VIX1D against the realised variance
of SPY. It answers: is the premium that a short-volatility bot sells there in the index data, and does it
hold outside the stress windows? It is for research. It is stage 1a of alpha-1
(`trading/research/alpha-1-stage1.md`). The same note has stage 1b, the price settings of `fa trade backtest`.
The command uses index levels and real SPY prices. It uses no option price and sends no order.

## Inputs
| Input | Source | Note |
|---|---|---|
| SPY daily close and open | `SPY_1d.csv` in the trading cache (`Store`, `rust/src/trading/data.rs`) | Yahoo bars from `fa trade fetch` |
| VIX daily close | `_VIX_1d.csv` | Index points. The command uses the days common to SPY and VIX |
| VIX1D daily open | `_VIX1D_1d.csv` | Yahoo series, from 2023-04. The open print may be stale (note) |
| Stress windows | `--stress-file PATH`, default `analysis/macro/stress-windows.csv` (rule: `analysis/macro/2026-11-21-stress-windows.md`) | Read by `parse_windows` (also used by `fa portfolio calibrate`). No copy in code |
| Options | `--horizon N`, `--lag N`, `--lags L1,L2,...` | Default 21 and 20. `--lags` prints one table of t_NW by lag |

## Method (short)
1. Daily log return of SPY: close to close (`premium_30d`).
2. 30-day premium of day t: `(VIX_t/100)^2` minus the sum of the squared returns of the days t+1 to t+21,
   times 252/21. No demeaning. The last 21 days have no premium.
3. Statistics for a subset of days (`row`): mean, median, share of positive days, Newey-West t
   (`stats::nw_mean_t`, Bartlett, 20 lags) and block t (every 21st day, plain `stats::t_mean`).
4. Subsets: all days; before the first window start ("1993 to 2015"); the block from the first window start to
   the last day of the data ("2016-2026"), inside and outside the windows (the gate row); each window. The block
   has no end date of its own, so a new day after the last window goes to the gate row. The row names follow the
   file and the data.
5. "Clean outside" drops each day with a window in the next 21 days. This uses the future (see Limits).
6. The same premium in vol points: VIX minus the realised vol.
7. 1-day premium: `(VIX1D open/100)^2/252` minus the squared return of the same day. Two returns:
   close to close and open to close. t_NW uses 5 lags (a constant in code).

## Outputs
Two printed tables: the 30-day premium by subset and window (unit: annual variance, 0.01 is about
10 vol points squared), and the 1-day premium (daily variance). The measured numbers of 2026-11-21 are in
`trading/research/alpha-1-stage1.md`. That note is not a source: run the command again. In that run the gate
row has mean 0.0056, 85% positive days, t_NW 1.97 (below 2), and the full 33 years have t_NW 4.36. The
1-day premium is about zero and depends on the convention. No file is written.

## Assumptions
| Record | Name | Value | One line |
|---|---|---|---|
| A-150 | Premium definition and horizon | VIX^2 minus 252/21 x realised; VIX1D open | Convention, no study |
| A-151 | Newey-West lags | 20 (30-day), 5 (1-day), block step 21 | t moves by 0.5 with the lag |
| A-152 | Stress windows and gate subset | 13 windows from the file, block from 2016-01-05 to the end of the data | Defined with hindsight |
| A-153 | Deflated Sharpe inputs | N = 20, V = 1/T | N too low: DSR is an upper bound |
| A-154 | Price stress values | spread x2, skew x1.5 and x0.5, mid fill | Chosen by the ticket |

A-153 and A-154 belong to the price runs of `fa trade backtest` (stage 1b). The other assumptions of that command
are in `docs/models/option-bot-backtest.md` (A-140 to A-148).

## Price settings of `fa trade backtest` (stage 1b)
`set_price_knobs(spread_mult, skew_mult, mid_fill)` (`pricing.rs`) sets three values for the whole process.
Defaults 1, 1, false keep the old numbers. Options (`cli.rs`): `--spread-mult X`, `--skew-mult X`, `--mid-fill`.
`--trials N` prints the OOS t of the trade P&L and the deflated Sharpe ratio (`stats::deflated_sharpe`).
`--sr-var V` sets the variance of the trial Sharpe ratios (default 1/T).
`--cut-scan [--step DAYS] [--span DAYS]` (one bot, `grids.rs`) prints the OOS Sharpe and the t of the trade P&L at
each cut (the default cut moved by whole steps, 7 and 42 days by default) and the first bar date of the data.
`--price-grid` (one bot) runs the baseline and the knobs of A-154, with the change against the baseline and the
mean P&L that gives t = 2 (`2 sd / sqrt n`). Each prints only its table. The knobs are set before each run and
reset after it. The default fill is already the worse
side (long legs pay the ask). `--mid-fill` is a better price, for reference only.

## Limits
- A measurement on history. It is not a signal and not a P&L: the VIX is a risk-neutral measure, and
  a premium does not pay a trade after spread, skew and commission.
- The gate row stops alpha-1 at stage 1 (note, "Gate decision"). The primary reason is s05 OOS t 1.35,
  below 2. The premium row (t_NW 1.97) is secondary: its threshold is not dated before the runs.
- The "clean outside" row uses the future. Its t (13.49) is biased. It is a warning, not a result.
- The scan and the grid are screens, not a choice of a cut or a price: no cut or knob is selected by them.
- The windows were defined with hindsight. "Outside windows" is itself a choice made with the future.
- The DSR is an upper bound: N = 20 is a lower bound, and V = 1/T is a proxy.
- The 1-day result depends on the convention. The VIX1D history is 2023 to 2026, one regime.
- Only 2016 to 2026 is in the gate. The years 1993 to 2015 have no window in the file.
- The price knobs are process-wide values. Set them once, before the run.
- Backtest: the command is a measurement on history. All bot prices in stage 1b are model prices. The
  premium part uses real index levels. Neither part proves an edge. Use the gate of `docs/QUALITY-STANDARDS.md`
  before any trade.

## Code and command
| What | Path or command |
|---|---|
| Code | `rust/src/trading/premium.rs`; `rust/src/trading/grids.rs`; `rust/src/trading/stats.rs` (`t_mean`, `nw_mean_t`, `deflated_sharpe`); `rust/src/trading/pricing.rs` (`set_price_knobs`); `rust/src/trading/cli.rs` |
| Parameter file | `analysis/macro/stress-windows.csv` (windows). The other values are constants in the code |
| Command | `fa trade premium [--horizon N] [--lag N] [--lags L1,L2,...] [--stress-file PATH]` |
| Command (grids) | `fa trade backtest --bot ID --cut-scan [--step D] [--span D]`; `fa trade backtest --bot ID --price-grid` |
| Command (stage 1b) | `fa trade backtest --bot s05_ic_0dte --trials 20 [--spread-mult X] [--skew-mult X] [--mid-fill] [--sr-var V]` |
| Tests | `cargo test --manifest-path rust/Cargo.toml --locked` |
| Research note | `trading/research/alpha-1-stage1.md` |

## Review status
Not reviewed by the steps of `docs/QUALITY-STANDARDS.md`. The note is the work of the author, with a
dated hypothesis and a gate decision. No reviewer record is known.
