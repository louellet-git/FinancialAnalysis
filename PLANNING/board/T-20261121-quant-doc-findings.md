---
id: T-20261121-quant-doc-findings
title: Defects and hidden choices that the quant model documents found
area: engine
status: backlog
owner: louellet-git
head: quant
order: 3.05
branch:
created: 2026-11-21
requested_by: louellet-git
acceptance: Each item below is fixed with a test, or recorded as a choice in its assumption record, or closed with a reason. Before a new option bot result is used, items B1 to B4 are done.
---
Source: the model documents of `T-20261121-quant-model-docs` (documenters' reports, 2026-11-21; the quant head checked the items marked "checked" against the code).

## Option bots (records A-140 to A-149)
- B1. Dividend yield 0 in `bs_price` and `bs_delta` (`trading/pricing.rs`): calls too high, puts too low; most effect on 20 to 30 day call spreads (s08, s14). Size it.
- B2. The skew is counted two times on the put side: VIX already has the skew, and the pricer adds `SKEW_SLOPE` -0.12 (checked) on top.
- B3. No holiday calendar; daily expiries assumed before 2022 (`trading/calendar.rs`).
- B4. The out-of-sample columns are not a clean test: a split by bar count, no variant count logged by `fa trade backtest`, no Sharpe net of the risk-free rate.
- B5. The equity curve marks open positions at mid (`engine.rs`); exits cross the spread.
- B6. `fa trade paper` has no `--commission`; its account uses the model fill, not the chain price.
- B7. `^IRX` is divided by 100 with no conversion to a continuous rate.

## Option chain check and futures check (records A-120 to A-137)
- O1. Wrong text: `option_arb.rs` and `CHECK_USAGE` call the 0.99/1.01 band "the rule of the volatility fit"; the fit has no band.
- O2. Two forwards for one chain: 3 strikes in the check (`option_iv.rs` `PARITY_STRIKES`, checked), 5 in the fit. Choose one, or record why.
- O3. Step numbers in comments of `option_arb.rs` differ from `CHECK_NAMES`.
- O4. The VX tolerance 0.005 is about 100 times the rounding that its comment cites.
- O5. `fa futures check` compares one date and one ZQ contract; the ZQ price is a Yahoo bar, not known to be the CME settle.
- O6. ZQ can fail on a correct price after a target change early in the month (A-129).
- O7. `--compare` ignores `--time` for day 2.
- O8. Intrinsic, bound and parity checks do not run without a spot, with no message.

## Rates (records A-100 to A-116)
- R1. The tau clamp in `fit.rs` (`from_params`, checked) is silent: no output shows a tau at a clamp.
- R2. Day count: `recon.rs` uses days / 365.25 (checked), `fit.rs` and `richcheap.rs` use 365; bucket edges can differ.
- R3. Old comment `recon.rs` line 4: "through the Python interface" (checked).
- R4. `MR_T_THRESHOLD` 2.86 (a Dickey-Fuller value) is also applied to an ML t-statistic.
- R5. Hull-White mixes a real-world fit (a, sigma) with the market curve (theta).
- R6. The "lasting residual" rule of the step 2 note is in an `awk` script only, not in code.
