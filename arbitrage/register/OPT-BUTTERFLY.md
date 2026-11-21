---
id: OPT-BUTTERFLY
title: Convexity in strike
class: hard
instruments: SPX, SPXW, IBM, other chains
statement: w = (K3-K2)/(K3-K1); bid X(K2) <= w ask X(K1) + (1-w) ask X(K3)
tolerance: executable prices; EPS 1e-9
for: 2
severity: report
status: active
check: fa arb run; option_arb::check_chain (check 2, butterfly)
source: Breeden and Litzenberger 1978; Carr and Madan 2005 (no static arbitrage)
limits: Alpaca indicative quotes (modified, not OPRA): a measurement, not a trade. Quotes of one file are not at one time.
---
Consecutive strike triples of one series and type.
A break most likely means a wide or stale middle quote.
A triple whose middle quote is buy-only is not tested.
