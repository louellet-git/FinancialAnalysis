---
id: OPT-BOX-IDENTITY
title: Box identity inside one European chain
class: hard
instruments: SPX, SPXW (European)
statement: "K2 - K1 = df (C1 - C2 + P2 - P1); df is the same for all box pairs of one expiry"
tolerance: none yet; set from the observed spread of df across pairs, not before. Unit: USD for one share, or a rate in basis points
for: 2
severity: report
status: planned
check: none yet
source: van Binsbergen, Diamond, Van Tassel, Liberty Street Economics, 2023-10-02 (as in chiefs/louellet-git/quant/industry-2026-11-21.md)
limits: Alpaca indicative quotes (modified, not OPRA): a measurement, not a trade. Quotes of one file are not at one time.
---
A box (a call spread and a put spread on the same two strikes) pays K2 - K1 at expiry. Its price gives df.
Hard: it uses one chain copy only.
Method of the source: for each expiry, regress the put minus call mid on the strike. The slope gives df.
Keep bid > 0, 1 month to 5 years, and an expiry only when R2 is 0.99999 or more.
A break most likely means a stale or modified quote.
