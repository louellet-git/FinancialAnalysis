---
id: OPT-PARITY-UPPER
title: Upper put-call parity bound (American)
class: hard
instruments: IBM and other American chains
statement: bid C - ask P <= S - K df
tolerance: executable prices; EPS 1e-9; S is a last price of the same file, no dividend (A-124)
for: 2
severity: report
status: active
check: fa arb run; option_arb::check_chain (check 4, parity)
source: Merton 1973
limits: Alpaca indicative quotes (modified, not OPRA): a measurement, not a trade. Quotes of one file are not at one time. S has no bid or ask and no time of the quote.
---
Runs for American options only. Needs the spot of the file.
The lower bound needs the dividends and is not tested.
A pair whose call is buy-only is not tested.
