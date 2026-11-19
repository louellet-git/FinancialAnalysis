---
id: T-20261119-option-backtest-rust
title: Move the option backtester to Rust (the Python command takes 94 seconds)
area: trading
status: in-progress
owner: louellet-git
branch: claude/option-backtest-rust
created: 2026-11-19
acceptance: A Rust command gives the same leaderboard numbers as `python -m scripts.trade_backtest` for each of the 22 bots, in less than 10 seconds, and the Python backtester is deleted in the same change.
---
Owner request, 2026-11-19: a script that takes too long must be made fast with Rust.

Measured on 2026-11-19 (cloud container, 4 processors): `python -m scripts.trade_backtest` takes 94 seconds. Each other command takes less than 5 seconds (verdict 0.3 s, the IBM driver run 0.3 s, `scripts.analyze` 1.2 s, the 361 Python tests 4.2 s, `mapcheck` on the S&P 500 4.8 s).

Where the time goes (profile): the engine loop `trading/engine.py` (`run`), the option pricer `trading/pricing.py` (`quote`, `year_fraction`: 26 s) and `trading/data.py` (`bar_close_time`: 20 s). Most of the cost is date and table operations for each bar, not mathematics.

This is step 8 of `T-20261119-python-to-rust` (3,535 Python lines: engine, pricer, structures, metrics, 22 bots).
