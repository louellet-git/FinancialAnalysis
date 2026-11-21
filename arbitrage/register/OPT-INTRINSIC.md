---
id: OPT-INTRINSIC
title: Ask is not below the intrinsic value (American)
class: hard
instruments: IBM and other American chains
statement: ask C >= S - K; ask P >= K - S
tolerance: EPS 1e-9 at the ask
for: 2
severity: report
status: active
check: fa arb run; option_arb::check_chain (check 5, intrinsic)
source: Merton 1973
limits: Alpaca indicative quotes (modified, not OPRA): a measurement, not a trade. Quotes of one file are not at one time. S is a last price.
---
Runs for American options only. A quote with a break leaves the other checks.
A break most likely means a spot time gap or a dividend that S does not hold.
