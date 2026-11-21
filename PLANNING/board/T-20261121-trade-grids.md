---
id: T-20261121-trade-grids
title: Robustness grids of the strategy tests (stress windows from the file, lags, cut scan, price grid)
area: trading
status: in-progress
owner: louellet-git
head: quant
order: 0.86
branch: claude/quant-trade-grids
created: 2026-11-21
requested_by: louellet-git
acceptance: `fa trade premium --stress-file F` reads the windows and the block end from the file (no copy in code); `--lags L1,L2,...` prints one table; `fa trade backtest --cut-scan` and `--price-grid` print one table each; each with fixed-answer tests; the default outputs do not change; records A-150 to A-154 and `docs/models/variance-premium-test.md` are current.
---
Rows 9 to 12 of `T-20261121-quant-engine-integration` (owner request, 2026-11-21). Row 9 fixes P1 of
`T-20261121-quant-doc-findings` (days after 2026-11-19 get a wrong row label).
