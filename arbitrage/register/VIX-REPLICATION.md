---
id: VIX-REPLICATION
title: The VIX from the SPX chain against the published VIX
class: soft
instruments: SPX, SPXW chain; published VIX (data/raw/vix/VIX_History.csv, close)
statement: VIX^2 = 2/T sum (dK/K^2) e^(rT) Q(K) - (1/T)(F/K0 - 1)^2, 30-day interpolation, times 100
tolerance: none yet; set from the observed gaps, not before. Unit: VIX points
for: 2
severity: report
status: planned
check: none yet
source: Cboe Global Indices, Cboe Volatility Index Mathematics Methodology, v5.0, revised 2026-02-26 (analysis/priors/arb_invariants_industry_2026-11-21.md, S1)
limits: Alpaca indicative quotes (modified, not OPRA): a measurement, not a trade. Quotes of one file are not at one time. The VIX close is at 16:15 ET; our copy has one time stamp, so the gap is a field of the result.
---
A soft relation: our chain against the published index.
Steps of the method: forward from parity, K0, out-of-the-money strips, stop after two zero bids, 30-day weights.
