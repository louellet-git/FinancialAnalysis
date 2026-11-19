---
id: T-20261119-options-paper-test
title: Paper-test the two best option bots against live quotes
area: trading
status: backlog
owner: louellet-git
branch:
created: 2026-11-19
acceptance: For s14b and s05, a minimum of 30 signals have a live-chain price recorded adjacent to the model price, and the paper result is compared with the backtest.
---
All backtest option prices are model prices (Black-Scholes on VIX, skew and spread), not market quotes. Do not use real money before this test. The paper account also uses model fills; only the new signals get live bid/ask prices. Command: `fa trade paper` (run it on a schedule in market hours; the live chain is the Cboe delayed quotes, and a signal with an expiry that the chain does not list gets the model price: `docs/HIDDEN-ASSUMPTIONS.md` #92). Results: `trading/research/LEADERBOARD.md`.
