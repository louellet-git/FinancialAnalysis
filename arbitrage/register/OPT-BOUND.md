---
id: OPT-BOUND
title: Bid is not above the upper bound (American)
class: hard
instruments: IBM and other American chains
statement: bid C <= S; bid P <= K
tolerance: EPS 1e-9 at the bid
for: 2
severity: report
status: active
check: fa arb run; option_arb::check_chain (check 6, bound)
source: Merton 1973
limits: Alpaca indicative quotes (modified, not OPRA): a measurement, not a trade. Quotes of one file are not at one time. S is a last price.
---
Runs for American options only. A quote with a break leaves the other checks.
A buy-only quote has no bid and is not tested.
