---
id: T-20261119-option-backtest-rust
title: Move the option trading code to Rust and delete the Python copy
area: trading
status: review
owner: louellet-git
branch: claude/option-backtest-rust
created: 2026-11-19
acceptance: A Rust command gives the same leaderboard numbers as `python -m scripts.trade_backtest` for each of the 22 bots, in less than 10 seconds, and the Python backtester is deleted in the same change.
---
Owner request, 2026-11-19: a script that takes too long must be made fast with Rust.

Measured on 2026-11-19 (cloud container, 4 processors): `python -m scripts.trade_backtest` takes 94 seconds. Each other command takes less than 5 seconds (verdict 0.3 s, the IBM driver run 0.3 s, `scripts.analyze` 1.2 s, the 361 Python tests 4.2 s, `mapcheck` on the S&P 500 4.8 s).

Where the time goes (profile): the engine loop `trading/engine.py` (`run`), the option pricer `trading/pricing.py` (`quote`, `year_fraction`: 26 s) and `trading/data.py` (`bar_close_time`: 20 s). Most of the cost is date and table operations for each bar, not mathematics.

This is step 8 of `T-20261119-python-to-rust` (3,535 Python lines: engine, pricer, structures, metrics, 22 bots).

## State, 2026-11-19 (second part: data, live chain, paper trading, deletion)

Owner decision, 2026-11-19: the free Yahoo source is allowed for stock prices and option chains, in Rust ("everything code wise should be rust"). The block of the first part is removed.

Done:

- The Rust backtester (`rust/src/trading/`, `fa trade backtest`): parity with the Python engine for the 22 bots and the template, with the 3% risk account and with `--max-risk 250`. Same trades; each price and each profit and loss differs by less than 2e-10 dollars; each summary number by less than 4e-12 (relative). Checked again on 2026-11-19 after the data and engine changes, and before the Python code was deleted. All 22 bots take 1.0 second (4 processors).
- Bars in Rust: `fa trade fetch` (`fetch.rs`). Yahoo chart endpoint, the client of `rust/src/futures/` (the reply reader `parse_raw` is shared with `yahoo.rs`). It writes the same CSV files as the Python code, with the same merge rule. Comparison with the Python files: see "Comparison" below.
- Live option chains in Rust: Yahoo refuses the honest client (the crumb request answers HTTP 429), so the source is the Cboe delayed quotes (`chain.rs`). It gives bid and ask, about 15 minutes late. Not a published API.
- Paper trading in Rust: `fa trade paper` (`paper.rs`). Same options and the same state file format as `scripts/trade_paper.py`.
- The Python copy is deleted: `scripts/trade_backtest.py`, `scripts/trade_paper.py`, `tests/test_trading.py` and in `trading/`: `engine.py`, `pricing.py`, `structures.py`, `strategy.py`, `metrics.py`, `registry.py`, `data.py`, `paper.py`, `strategies/`, `__init__.py`. `trading/research/` stays (notes and `LEADERBOARD.md`). No remaining Python file imports a deleted module. `python -m pytest`: 349 passed (the 12 trading tests are deleted). `cargo test --manifest-path rust/Cargo.toml --locked`: 136 passed.

Comparison of `fa trade fetch` with the Python cache files (into a copy of the cache, 2026-11-19):

- The runs reached 14 of the 16 files at least one time (not reached: ^VIX1D 1h and ^VXN 5m; Yahoo answered HTTP 429). Each old row of each file that was reached has the same text as the Python file: all of SPY 1d from 1993 (8,509 of 8,510 rows), ^IRX from 1960 (16,705 of 16,706), ^VIX 1d, ^VXN 1d, ^VIX1D 1d and QQQ 1d, and the intraday files. The row sets (the times with their offsets) are the same, over the changes of daylight saving time from 1960. The only rows that differ are the rows of 2026-11-18, where Yahoo had corrected the volume (and, for ^VIX and ^VXN hourly, some high and low values). The Python cache had a volume of 59.6 million for one hourly SPY bar; Yahoo gives 5.6 million now. See `docs/HIDDEN-ASSUMPTIONS.md` #91.
- The code changed during the test (the float reading, the filters for a bar that has not ended, and for the early-close row). The last version was checked on the saved real replies of Yahoo for SPY 1h, SPY 5m and ^VIX 1h: the same number of rows as the Python files, with no new row, and only the rows of 2026-11-18 differ. It was not run again against the live endpoint for the other files, because Yahoo gave HTTP 429 to all requests after 15:00 UTC.
- Backtest on the original cache and on the cache with the Rust refresh of SPY 1h, SPY 5m and ^VIX 1h: the leaderboard, the trades and the equity curves of all bots are byte for byte the same with `--end 2026-11-16`. On the full range, no bot differs from the Python reference.
- Yahoo answered HTTP 429 to about half of the requests from the cloud container. The refresh of all 16 files did not finish in one run. See `docs/HIDDEN-ASSUMPTIONS.md` #90.

Differences from the Python code (on purpose):

- A bar that has not ended is not written to the cache (Python wrote the partial bar of the running session; the paper trader used it as a closed bar). The live price row of Yahoo and the closing row of an early-close day are dropped.
- The paper report says where each signal price comes from: the live chain, the model, or a mix. The Python report always said "live chain".
- The state file has the same fields, but the keys are in alphabetical order.

Kept on purpose (not changed, the owner decides): the three doubtful rules of the Python bots (the VIX hourly file has bars on the hour and SPY on the half hour, so seven SPY hourly bots use the VIX close of the day before; `--end` keeps one more day; `LEADERBOARD.md` is older than the cache and its numbers were not changed).

Open:

- The bots assume an expiry on each trading day. The live chain has daily expiries for about two weeks only. A signal with another expiry gets the model price (`docs/HIDDEN-ASSUMPTIONS.md` #92). The owner decides whether a bot should choose the nearest listed expiry.
- `fa trade fetch` can need more than one run when Yahoo limits the rate.
- Research notes in `trading/research/s*.md` still name the Python command in their text (they are records and keep their text).
