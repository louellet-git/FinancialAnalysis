---
id: T-20261121-quant-doc-findings
title: Defects and hidden choices that the quant model documents found
area: engine
status: backlog
owner: louellet-git
head: quant
story: E4.S4
order: 3.05
branch:
created: 2026-11-21
requested_by: louellet-git
acceptance: Each item below is fixed with a test, or recorded as a choice in its assumption record, or closed with a reason. Before a new option bot result is used, items B1 to B4 are done.
---
Source: the model documents of `T-20261121-quant-model-docs` (documenters' reports, 2026-11-21; the quant head checked the items marked "checked" against the code).
Owner of all items: quant head.
Each item says "changes a value" (the fix changes a number that a model or a check gives) or "text only" (the fix changes a text or an output line).

## Option bots (records A-140 to A-149)
- B1. `bs_price` and `bs_delta` (`rust/src/trading/pricing.rs`) have dividend yield 0. Calls are too high and puts too low. The effect is largest for 20 to 30 day call spreads (s08, s14). Size it. Changes a value (bot model prices).
- B2. The put-side skew is counted two times (checked). VIX has the skew already, and the pricer adds `SKEW_SLOPE` -0.12 (`rust/src/trading/pricing.rs`). Changes a value (bot model prices).
- B3. No holiday calendar, and daily expiries are assumed before 2022 (`busday_count` in `rust/src/trading/calendar.rs`, `Ctx::expiry` in `rust/src/trading/engine.rs`). Changes a value (variance clock and expiry set).
- B4. The out-of-sample columns are not a clean test (`summarize` in `rust/src/trading/metrics.rs`). The split is by bar count, `fa trade backtest` logs no variant count, and the Sharpe is not net of the risk-free rate. Text only (output and notes).
- B5. The equity curve marks open positions at mid (`equity_now` in `rust/src/trading/engine.rs`). Exits cross the spread. Changes a value (equity curve and drawdown).
- B6. `fa trade paper` has no `--commission` option (`rust/src/trading/cli.rs`, `rust/src/trading/paper.rs`). Its account uses the model fill, not the chain price. Link: `T-20261119-options-paper-test`. Changes a value (paper account).
- B7. `^IRX` is divided by 100 with no conversion to a continuous rate (the `rate` series in `rust/src/trading/data.rs`). Changes a value (bot model prices).

- B8. Measure the model price of the option legs against real quotes: for each SPY condor that s05 would sell on a copied day (the daily Alpaca chain copies from 2026-11-23), the real credit at bid and ask against the model credit (`trading/pricing.rs`). From `T-20261121-alpha-1-pipeline` stage 2 (stopped at gate 1, so it runs as a check of the backtest prices, not as a strategy stage). Needs two or more copied days. Changes a value (it sizes B1 and B2).

## Option chain check and futures check (records A-120 to A-137)
- O1. Wrong text: `rust/src/market/option_arb.rs` and `CHECK_USAGE` (`rust/src/market/options_cli.rs`) call the 0.99/1.01 band "the rule of the volatility fit". The fit has no band. Text only.
- O2. One chain has two forwards: 3 strikes in the check (`PARITY_STRIKES` in `rust/src/market/option_iv.rs`, checked) and 5 in the fit (`PARITY_STRIKES` in `rust/src/volmodels/chain.rs`). Choose one, or record why. Link: `T-20261120-arb-3-index-parity-box`. Changes a value (check break counts).
- O3. Step numbers in comments of `rust/src/market/option_arb.rs` differ from `CHECK_NAMES` in the same file. Text only.
- O4. `VX_TOLERANCE` 0.005 (`rust/src/futures/check.rs`) is about 100 times the rounding that its comment cites. Changes a value (a check's pass or fail).
- O5. `fa futures check` compares one date and one ZQ contract (`check_zq` in `rust/src/futures/check.rs`). The ZQ price is a Yahoo bar, and it is not known to be the CME settle. Links: `T-20261120-arb-4-futures-basis`, `T-20261120-arb-3-index-parity-box`. Changes a value (a check's pass or fail).
- O6. ZQ can fail on a correct price after a target change early in the month (`expected_average` and `ZQ_TOLERANCE` in `rust/src/futures/check.rs`, A-129). Changes a value (a check's pass or fail).
- O7. `--compare` ignores `--time` for day 2 (`--compare` code in `check` of `rust/src/market/options_cli.rs`). Link: `T-20261120-arb-3-index-parity-box`. Changes a value (day 2 breaks).
- O8. Intrinsic, bound and parity checks do not run without a spot, and no message says so (`spot` in `rust/src/market/option_arb.rs`). Link: `T-20261120-arb-4-futures-basis`. Text only (a message).
- O9. A missing VX history file stops the futures check. `cboe::history` (`rust/src/futures/cboe.rs`) accepts HTTP 403 for a file that does not exist. `is_not_found` (`rust/src/futures/mod.rs`) accepts only HTTP 404, so `compare_vx` (`rust/src/futures/check.rs`) returns the error and `check_vx` stops. The test fixture in `check.rs` returns 404 and hides this. The step 1 note of `T-20261120-arb-4-futures-basis` names the same gap. Records: A-130. Changes a value (a check's pass or fail).

## Rates (records A-100 to A-116)
- R1. The tau clamp in `Nss::from_params` (`rust/src/fit.rs`, checked) is silent. No output shows a tau at a clamp. Changes a value (fitted taus), if the bounds change.
- R2. Day count: `rust/src/recon.rs` uses days / 365.25 (checked), and `rust/src/fit.rs` and `rust/src/richcheap.rs` use 365. Bucket edges (`bucket` in `recon.rs`) can differ. Changes a value (bucket edges).
- R3. Old comment in the header of `rust/src/recon.rs`: "through the Python interface" (checked). Text only.
- R4. `MR_T_THRESHOLD` 2.86 (`rust/src/ratemodels/calib.rs`) is a Dickey-Fuller value. The code also applies it to an ML t-statistic. Changes a value (a warning flag).
- R5. Hull-White mixes a real-world fit (a, sigma) with the market curve (theta) (`HullWhite` branch in `rust/src/ratemodels/calib.rs`). Text only (record the choice).
- R6. The "lasting residual" rule of the step 2 note is in an `awk` script only (`analysis/priors/rates_recon_step2_2026-11-20.md`), not in code. Text only. FIXED 2026-11-21: `fa recon history --lasting` (A-117, ticket `T-20261121-recon-lasting`).

## Variance premium test (records A-150 to A-154; added 2026-11-21 from `docs/models/variance-premium-test.md`)
- P1 (FIXED in T-20261121-trade-grids: the windows come from the file, the block ends at the last day of the data). The 2016-2026 block ends at a fixed date: `report` in `rust/src/trading/premium.rs` sets `w16` to 2016-01-05 to 2026-11-19 (checked). A day after 2026-11-19 goes into the row "1993 to 2015 (no windows defined)" with a wrong label. The window list is a hand copy of `analysis/macro/stress-windows.csv`, and no test links them. Changes a value (a row of the table).
- P2. The row label "all days 1990s-2026": SPY starts in 1993 (`report`, `premium.rs`). Text only.
- P3. The 1-day table uses a block step of 1, so its "t_blk" is the plain t; its window rows print no t (`premium.rs`). Text only.
- P4. The median is `v[len / 2]`, the upper middle value for an even count (`row`, `premium.rs`, checked). Changes a value (small).
