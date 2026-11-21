---
id: OPT-SLOPE
title: Price difference is not more than the strike difference
class: hard
instruments: SPX, SPXW, IBM, other chains
statement: call: bid C(K1) - ask C(K2) <= K2 - K1; put: bid P(K2) - ask P(K1) <= K2 - K1
tolerance: executable prices; EPS 1e-9; no discount in the bound (A-137), so the bound is loose
for: 2
severity: report
status: active
check: fa arb run; option_arb::check_chain (check 1, slope)
source: Merton 1973; docs/assumptions A-137
limits: Alpaca indicative quotes (modified, not OPRA): a measurement, not a trade. Quotes of one file are not at one time.
---
All strike pairs of one series and type.
The bound uses no discount factor, so it is weaker than the exact bound df (K2 - K1).
A pair whose sold leg is buy-only is not tested.
