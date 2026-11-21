---
id: OPT-BOX-RATE
title: Box rate against the Treasury curve
class: soft
instruments: SPX with the Treasury par curve
statement: K2 - K1 = df (C1 - C2 + P2 - P1); the implied rate is near the Treasury or SOFR rate
tolerance: none yet; a spread, not a bound
for: 2
severity: report
status: planned
check: none yet
source: NY Fed method (regress put-minus-call mid on K); analysis/priors/arb_invariants_industry_2026-11-21.md
limits: Alpaca indicative quotes (modified, not OPRA): a measurement, not a trade. Quotes of one file are not at one time. The Treasury curve is a different source and date; the box rate is not a Treasury rate.
---
A soft relation: two sources. It tests the rate input of all the other checks.
Compare as a separate relation, not as an input.
