---
id: OPT-PARITY-STRIKE
title: European put-call parity, spread of the forward across strikes
class: hard
instruments: SPX, SPXW, XSP
statement: C - P = df (F - K) for each strike; F is the same for all strikes of one expiry
tolerance: none yet; set from the observed spread of F, not before. Unit: index points of F
for: 2
severity: report
status: planned
check: none yet; planned in option_iv.rs (forward from parity) with a per-strike spread
source: Cboe VIX methodology v5.0 (forward from the smallest |C - P|); analysis/priors/arb_invariants_inventory_2026-11-21.md
limits: Alpaca indicative quotes (modified, not OPRA): a measurement, not a trade. Quotes of one file are not at one time.
---
Today the parity gives only the forward (median of 3 strikes). This relation tests each strike against F.
The spread of F is a quote-quality score. It also needs the rate (see OPT-BOX-RATE).
