---
id: T-20261120-arb-1-chain-check
title: Find the static arbitrage in a stored option chain with bid and ask
area: trading
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: quant
order: 1.1
branch: claude/upbeat-cori-a4ykf5
created: 2026-11-20
acceptance: `fa options check IBM --date 2026-11-19` reads the stored snapshot and gives, for each series (root and expiry), the number and the size of the violations at executable prices, the tests with planted violations pass, and a short findings note gives the result for IBM and JNJ.
---
Owner request, 2026-11-20: start to find arbitrage in our data (`../../chiefs/louellet-git/quant/RECORD.md`).

No check of the no-arbitrage rules exists in `rust/src`. Put-call parity is used only to get a forward
(`option_iv.rs`, `option_history.rs`); the residuals are not kept.

Scope (lean, Rust, one new file `rust/src/market/option_arb.rs`, one command in `options_cli.rs`):
1. Use executable prices: buy at the ask, sell at the bid. A violation at mid prices only is not a finding.
2. For each series: call prices fall and put prices rise with the strike; the slope between two strikes is
   not more than the discount factor; butterfly prices are not below zero.
3. For each root and option type: the price (or the total variance) does not fall when the expiry is later.
4. Put-call parity: for European options (SPX), C - P = df * (F - K) within the bid and ask; for American
   options (IBM, JNJ, SPY), the bounds S - K <= C - P <= S - K * df. Give the implied dividend (or borrow)
   rate of each expiry from the parity forward.
5. Output: a table for each series (tests, violations, largest size in dollars and in ticks), and the ten
   largest violations. A quote with no bid, or a bid of zero, is not used.

Data: the store has `IBM/options/2026-11-19.json` and `JNJ/options/2026-11-19.json` (bid and ask, one day).
SPX and SPY chains come with `T-20261120-vol-7-chain-copy`.

Limits: a violation is not a profit. Before the word "arbitrage" is used for a finding, write the costs, the
early exercise, the borrow and the time of the quotes (`docs/HIDDEN-ASSUMPTIONS.md` #112: the repo rate and the borrow).
Agent: `volatility-derivatives-quant`. Gate: `model-validator`, `code-reviewer`.
