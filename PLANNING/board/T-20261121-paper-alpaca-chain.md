---
id: T-20261121-paper-alpaca-chain
title: An Alpaca chain for the paper trader (in place of the Cboe chain)
area: trading
status: backlog
owner: louellet-git
head: quant
story: E4.S5
order: 3.4
branch:
created: 2026-11-21
requested_by: louellet-git
acceptance: `fa trade paper` reads the live chain (bid, ask, implied volatility, delta) from the Alpaca indicative feed through `options_cli::snapshot_alpaca`, with no Cboe request; a fixture test of the adapter; the signal line says "indicative"; `docs/models/paper-trader.md` and record A-149 are current.
---
The Head of Technology stopped all automatic Cboe reads (Cboe terms, 2026-11-21; branch `claude/cboe-off`). `fa trade
paper` now runs with `--no-live` (model prices) only. This ticket gives it a live chain again, from Alpaca.

Needed by: the live signal line of `fa trade paper` (the account always uses the model fill: A-149). No strategy
has passed its gates (alpha-1 stopped at gate 1), so this waits after E4.S1 and the register. Use the same request
budget as `fa algo` and the capture (one budget for the Alpaca data host).
