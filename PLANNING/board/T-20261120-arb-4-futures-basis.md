---
id: T-20261120-arb-4-futures-basis
title: Compare futures with their spot and carry, and the VIX options with the VX futures
area: trading
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: quant
order: 3.2
branch: claude/upbeat-cori-a4ykf5
created: 2026-11-20
acceptance: `fa futures check` gives the VX future against the VIX option parity forward of the same expiry and the ZQ and SR3 strip against the New York Fed rates, each check has a test with a fixed answer, and one real day is in the findings note.
---
Owner request, 2026-11-20: start to find arbitrage in our data.

Today `fa futures check` compares two VX settlement files and the ZQ spot month against EFFR
(`rust/src/futures/check.rs`); `check_vx` and `check_zq` have no test.
1. Add tests for the two checks.
2. VIX options are priced on the VX future of their expiry (#280): the parity forward of the VIX chain and
   the VX settlement of the same date must agree within the bid and ask.
3. The ZQ and SR3 implied rates of each month against the EFFR and SOFR path.
4. Later, only when a spot and a dividend source exist: ES against the SPX parity forward (carry).

Depends on: `T-20261120-store-arb-data` (futures, VIX and New York Fed files) and, for step 2, the VIX chain
copy (`T-20261120-vol-7-chain-copy`).
Agent: `rates-credit-quant` (rates), `volatility-derivatives-quant` (VIX). Gate: `model-validator`,
`code-reviewer`.

Step 1 is done (tests of `check_vx` and `check_zq`; review PASS-WITH-NOTES, 2026-11-21). Two notes for the next
step: a missing VX history file returns "HTTP 403 ... (no history file)" in `cboe.rs` (~183, ~195), but
`compare_vx` skips only "HTTP 404": make the test fixture use the real 403 text and state the rule (skip or error);
and name the skipped contracts in the output, so that "N VX compared" shows the coverage.
