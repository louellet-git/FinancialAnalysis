---
id: OPT-CALENDAR
title: Longer expiry is worth at least as much (American)
class: hard
instruments: IBM and other American chains; not a European index
statement: bid X(K,T1) <= ask X(K,T2), T1 < T2, same root, type, strike
tolerance: executable prices; EPS 1e-9
for: 2
severity: report
status: active
check: fa arb run; option_arb::check_chain (check 3, calendar)
source: Merton 1973 (American options)
limits: Alpaca indicative quotes (modified, not OPRA): a measurement, not a trade. Quotes of one file are not at one time. The index case (total variance form) is not tested.
---
Runs for American options only. Same strike, all expiry pairs; the pair is counted on the series of T2.
A break most likely means a stale far leg or a time gap between expiries.
