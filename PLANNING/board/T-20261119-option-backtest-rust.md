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

## State, 2026-11-19

Done:

- The Rust backtester is in `rust/src/trading/`. Run it with `fa trade backtest [--bot ID] [--out FILE]` (`fa trade backtest --help` shows the options). Rayon runs the bots in parallel.
- Parity with the Python engine holds for the 22 bots and the template bot, for the two accounts (default 3% risk, and `--max-risk 250`). The trades are the same (entry and exit times, structures, strikes, units, exit reasons, tags). Each price and each profit and loss differs by less than 2e-10 dollars. The equity curve of each bot differs by less than 5e-10 dollars, and each summary number by less than 4e-12 (relative).
- Time of all 22 bots on the cached data: 0.9 seconds in Rust (4 processors, and also on 1 processor). The Python command takes 94 seconds. The slowest bot is s20 (0.8 seconds, the model fit).
- Rust tests: pricer, variance clock, structures, risk rules of the engine (size, rejection reasons, position limit, pattern-day-trader rule, exit rules, no lookahead), metrics, data alignment, and one end-to-end run. The expected numbers of the end-to-end test come from the Python backtester on the same files.

Not done on purpose: the Python backtester is NOT deleted. This branch has two copies of the same part. Do not merge it as it is (CLAUDE.md, "One part has one implementation").

What blocks the deletion:

1. Paper trading (`trading/paper.py`, `scripts/trade_paper.py`) uses the Python engine, the Python bots and `LiveChainPricer`. It needs live option chains from yfinance. A Rust source for live chains does not exist, and the data source for Rust is an open owner decision (ticket `python-to-rust`, step 6, decision 1).
2. `python -m scripts.trade_backtest --fetch` is the only command that downloads the market data (`trading/data.py`, yfinance). The download is part of step 6. It must stay until the data source is decided.
3. Decision for the owner: (a) move paper trading to Rust too, with a data source for Rust (direct Yahoo calls or a keyed API), and then delete all Python copies in one change; or (b) move paper trading to Rust first in the cache-only mode (no live chain, no refresh), and keep a small Python program only for the download and the live chain. The deletion list is: `scripts/trade_backtest.py`, `trading/engine.py`, `pricing.py`, `structures.py`, `strategy.py`, `metrics.py`, `registry.py`, `strategies/`, `paper.py`, `scripts/trade_paper.py`, `tests/test_trading.py`.

Doubtful rules in the Python code (kept in Rust, not changed): see the report of this ticket. `trading/research/LEADERBOARD.md` is older than the data cache (dated 2026-10-11): the Python backtester and the Rust backtester both give other numbers today (for example s14b: 519 trades, +312%; the file says 516 trades, +358%). Both new numbers agree. The file was not changed.
