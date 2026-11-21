# Model: option bot backtest and the model price of the option legs

Updated: 2026-11-21. Commit: not yet made.

## Purpose
WARNING: every option price in this backtest is a model price (Black-Scholes on a volatility index, with an
invented skew and spread). No real option quote is used. A result is a test of a rule on a model, not a
record of fills. The repo sends no order to a broker.

`fa trade backtest` runs rule-based bots (SPY and QQQ option structures) on cached bars. Each bot is one
isolated paper account. The command gives one leaderboard row for each bot. It is for research, to find
a rule that deserves a forward test (`docs/models/paper-trader.md`).

## Inputs
| Input | Source | Note |
|---|---|---|
| Bars (5-minute, hourly, daily) of SPY and QQQ | data cache, `Store` in `rust/src/trading/data.rs` | Yahoo bars from `fa trade fetch` |
| 30-day volatility | `^VIX` (SPY), `^VXN` (QQQ), `data.rs` | In percent in the file, divided by 100 |
| 1-day volatility | `^VIX1D`; for QQQ scaled by VXN/VIX (`data.rs`) | No value: the 30-day value is used |
| Short rate | `^IRX`, close of the day before (`data.rs`) | No file: 0.04 (`DEFAULT_RATE`) |
| Account rules | `RiskConfig::default()` (`engine.rs`) and the command options | See the table of records |

## Method (short)
1. The price of a leg is Black-Scholes (`pricing.rs`, European, settled at intrinsic value at expiry).
2. The ATM volatility is the 1-day value at one session or less and the 30-day value at 30 sessions or
   more. In between it is linear in time (`pricing.rs`).
3. The skew multiplies the ATM volatility by a quadratic in the standard log-moneyness (`pricing.rs`).
4. Time to expiry is a variance clock, not calendar time (`pricing.rs`). One close-to-close day is 1/252 year.
5. Half-spread is 0.5% of mid, at least $0.01 and at most $0.05 for each leg (`pricing.rs`).
   An entry pays the ask on long legs and receives the bid on short legs. An exit does the opposite.
6. For each bar (`engine.rs`): fill the entries that the bot queued on the last bar, at this bar open.
   Then mark the positions at the close and test the exits. Then settle the positions that expire today at
   intrinsic value. Then call the bot. The bot sees data up to the bar close only.
7. The engine sizes each entry from the maximum loss of the structure and the account rules
   (`engine.rs`). The structure has defined risk only (`structures.rs`).
8. Exits (`strategy.rs:ExitRules`) use the liquidation value on the bar close. No exit fills inside a bar.
9. `summarize` (`metrics.rs`) cuts the bars in time: the first 70% are in-sample, the rest are
   out-of-sample. Sharpe uses the daily equity returns times the square root of 252, with no risk-free rate.

## Outputs
A table with, for each bot: return, maximum drawdown, Sharpe, trades, win rate, profit factor, for all bars
and for the out-of-sample part, and the count of rejected entries and of blocked exits.
`--out FILE` writes a CSV file. `--trades-dir DIR` writes the trades and the equity curve of each bot.
The notes of each bot and the table of 2026-10-11 are in `trading/research/` (not a source of the numbers:
run the command again).

## Assumptions
| Record | Name | Value | One line |
|---|---|---|---|
| A-140 | Volatility input | VIX, VXN, VIX1D, linear in time | The index is used as the ATM volatility |
| A-141 | Skew | slope -0.12, curve 0.03, 0.6 to 2.5 | Fixed smile for all days |
| A-142 | Spread and fill rule | clip(0.5% mid, 0.01, 0.05) | Cross the half-spread, entry and exit |
| A-143 | Commission | $0.65 for each contract, each side | Expiry has no exit fee |
| A-144 | Risk-free rate | `^IRX` of the day before; 0.04 | Only for discounting |
| A-145 | Dividend yield | 0 | Not modelled |
| A-146 | Variance clock and calendar | overnight 0.25, U profile, no holidays | Daily expiries assumed |
| A-147 | In-sample split | 0.7 of the bars | One split in time, not a rolling test |
| A-148 | Account rules | $2,000, 3%, 3 positions, 12%, 6%, 20 units, PDT | Default account of the owner |

## The bots
A bot is a `Strategy` (`strategy.rs`). The registry is `rust/src/trading/bots/mod.rs`. The notes of each
bot are `trading/research/<name>.md`. Bars: D daily, H hourly, 5m 5-minute. Symbol: SPY if not stated.

| Bot | Bars | Rule (one line) |
|---|---|---|
| baseline_weekly_call_spread | D | Template. Friday: 0.40-delta call debit spread, 5 sessions. Runs only with `--include-templates` |
| s01_orb_0dte | 5m | Break of the 30-minute opening range: 0DTE debit spread in the break direction |
| s02_vwap_revert_0dte | H, QQQ | Far from VWAP in a quiet session: 0DTE credit spread back to VWAP |
| s03_gap_fade | H | Moderate opening gap: 0DTE debit spread against the gap |
| s04_gap_and_go | H | Large gap that holds: 1DTE debit spread with the gap |
| s05_ic_0dte | H | Calm regime: narrow 0DTE iron butterfly at mid-morning |
| s06_last_hour_momentum | H | Return of the first bar sets the side: 0DTE debit spread at 15:30, to expiry |
| s07_overnight_drift | H | Late call debit spread, exit the next morning |
| s08_trend_regime | D | Close above SMA(200), SMA(50) above SMA(200): $1 call debit spread, 30 sessions |
| s09_rsi2_pullback | D, SPY+QQQ | Uptrend and RSI(2) below 5: call debit spread, 10 sessions |
| s10_vix_regime | D, QQQ | Calm VXN: put credit spread |
| s11_vrp_condor | D | Monday: $1-wing iron condor, 28 sessions |
| s12_turn_of_month | D | Call debit spread across the turn of the month |
| s13_fomc_drift | D | Call debit spread over the day before a scheduled FOMC statement |
| s14_spy_qqq_rotation | D, SPY+QQQ | Relative-strength leader above SMA(200): call debit spread, 20 sessions |
| s14b_spy_trend_calls | D | The same, SPY only (ablation of s14) |
| s15_squeeze_breakout | H, SPY+QQQ | First bar out of a Bollinger-in-Keltner squeeze: debit spread, 3 sessions |
| s16_donchian_breakout | D, SPY+QQQ | 20-session channel break: debit spread with the trend, rolled |
| s17_capitulation_bounce | D | 3% drop, VIX spike, rising SMA(200): call debit spread, 10 sessions |
| s18_pin_butterfly | H | Quiet afternoon near VWAP: cheap 0DTE call butterfly at VWAP |
| s19_opex_cycle | D | Calendar of the OPEX weeks: weekly call or put debit spread |
| s20_walkforward_model | D | L2-logistic model of the 5-day direction, refit every 21 sessions: debit spread if P at or beyond 0.62 or 0.38 |
| s21_bollinger_fade | D | First close outside the 20-day 2-sigma band: 0.25-delta credit spread against it. Needs `--max-risk 250` |

## Limits
- All prices are model prices. A real chain has other volatility, other skew and wider spreads.
  `trading/research/s11_vrp_condor.md` and `s05_ic_0dte.md` say where the bias is in favor of the bot.
- No early assignment, no dividend, no holiday calendar. The model assumes daily expiries on every
  weekday, also in years when these expiries did not exist. The equity curve marks open positions at mid (`engine.rs`). Exits fill at bid and ask.
- Exits fill at a bar close only. A stop inside a bar is not seen. Strike grid: $1.
- The split is by bar count. The authors of the bots saw all bars; several notes say that they tried
  more variants than planned. The command logs no variant count (the run log is for `fa portfolio backtest`).
  The out-of-sample columns are not a clean test. The forward test is `fa trade paper`.
- A bot result with few trades (the leaderboard of 2026-10-11 has bots with 17 or fewer) is noise.
- Do not use a result for a trade without the gate of `docs/QUALITY-STANDARDS.md`.

## Code and command
| What | Path or command |
|---|---|
| Code | `rust/src/trading/` : `engine.rs`, `pricing.rs`, `structures.rs`, `strategy.rs`, `metrics.rs`, `stats.rs`, `data.rs`, `calendar.rs`, `cli.rs`, `bots/` |
| Parameter file | None. The values are constants in the code |
| Command | `fa trade backtest [--bot ID] [--max-risk N] [--risk F] [--equity N] [--commission N] [--start D] [--end D] [--is-fraction F]` |
| Tests | `cargo test --manifest-path rust/Cargo.toml --locked` |
| Research notes | `trading/research/LEADERBOARD.md` and one note for each bot |

## Review status
Not reviewed by the steps of `docs/QUALITY-STANDARDS.md`. The Rust code is a port of the earlier Python
backtester (`engine.rs` and `pricing.rs` comments say results match to the last digits).
`stats.rs` exists for that match (sums in the order of numpy).
