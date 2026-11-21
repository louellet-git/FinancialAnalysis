# A-140: volatility input of the model price of the option bots

Updated: 2026-11-21. Commit: not yet made.

## Choice
The ATM volatility of each priced option is a volatility index, with no multiplier (1.00 times the index):
- 30-day: `^VIX` for SPY, `^VXN` for QQQ (`rust/src/trading/data.rs:258`).
- 1-day: `^VIX1D`. For QQQ the value is VIX1D times VXN / VIX (`data.rs:399-406`). No VIX1D value: the
  30-day value is used.
- At one session or less to expiry: the 1-day value. At 30 sessions or more: the 30-day value. In
  between: linear in time (`rust/src/trading/pricing.rs:127-138`). Floor of the result: 0.01 (`pricing.rs:141`).
- An intraday index series with no value at a bar uses the previous daily close (`data.rs:320`).

## Where used
| Model | File | Parameter name |
|---|---|---|
| Option bot backtest (`docs/models/option-bot-backtest.md`) | `rust/src/trading/pricing.rs` | `SyntheticPricer::atm_vol` |
| Option bot backtest | `rust/src/trading/data.rs` | `vol_symbol`, `vol_frame`, `load_market` |
| Paper trader (model legs) | `rust/src/trading/paper.rs` | `price_signals` (uses `md.vol`) |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Real option quotes of the past | Not known to exist free (`trading/research/LEADERBOARD.md`: "no free historical option quotes exist") |
| Index times a factor below 1 (true ATM implied volatility is about 1 to 2 points below VIX) | Not known. No record of a decision. The note `trading/research/s11_vrp_condor.md` lists it as a bias |
| Realized volatility | Not known. No record |

## Why this choice
Convenience: VIX, VXN and VIX1D are the free daily series of implied volatility. No review record or study
gave the choice. Source: author choice, no study. The earlier Python pricer did the same (`pricing.rs:1`).
The VIX is a variance-swap number that includes the skew. The pricer then adds a skew (A-141), so the
put side counts the skew two times (`s11_vrp_condor.md`, "What IS model-assumed", item 1).
VIX1D is not usable before 2023: the ratio falls back to 1 and the term structure is flat
(`trading/research/s10_vix_regime.md`).

## Effect on the result
For the bot s11 only (SPY, no costs, start $6,000), scaling the whole pricer surface (`s11_vrp_condor.md`,
sensitivity table): 1.00x gives +153.9% and Sharpe 1.20. 0.93x gives +95.0% and Sharpe 0.84. 0.82x
gives +23.2% and Sharpe 0.25. No such run exists for the other bots. Run date and seed of that table: not known.

## Who decided and when
Not known. The pricer came from the Python code; the Rust port is in commit e692700 (2026-11-19).
The variance-clock change: commit ec78ccf (2026-10-11).

## When to examine it again
When a source of real historical SPY or QQQ option quotes exists. Before a trade that depends on a bot
whose result is sensitive to the level of volatility (s05, s11, s18, s21).
