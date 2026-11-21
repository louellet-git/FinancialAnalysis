---
id: OPT-MONOTONY
title: Call price falls and put price rises with strike
class: hard
instruments: SPX, SPXW, IBM, other chains
statement: call: bid C(K2) <= ask C(K1); put: bid P(K1) <= ask P(K2); K1 < K2
tolerance: executable prices (sell at bid, buy at ask); EPS 1e-9 in option_arb.rs; the same at bid and at ask
for: 2
severity: report
status: active
check: fa arb run; option_arb::check_chain (check 0, monotony)
source: Merton 1973 static bounds; docs/models/options-static-arbitrage-check.md
limits: Alpaca indicative quotes (modified, not OPRA): a measurement, not a trade. Quotes of one file are not at one time.
---
All strike pairs of one series (root, expiry) and type are tested.
A break most likely means a stale or modified quote. One bad quote makes many pair breaks.
A pair whose sold leg is buy-only (bid 0) is not tested.
No costs, no borrow, no early exercise.
