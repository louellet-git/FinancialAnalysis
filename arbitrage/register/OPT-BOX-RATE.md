---
id: OPT-BOX-RATE
title: Box rate against the Treasury curve
class: soft
instruments: SPX box rate; Treasury par curve (data/raw/rates)
statement: "the box rate from OPT-BOX-IDENTITY is near the Treasury rate of the same tenor"
tolerance: none yet; a spread in basis points, not a bound
for: 2
severity: report
status: planned
check: none yet
source: van Binsbergen, Diamond, Van Tassel, Liberty Street Economics, 2023-10-02 (as in chiefs/louellet-git/quant/industry-2026-11-21.md)
limits: Alpaca indicative quotes (modified, not OPRA): a measurement, not a trade. Quotes of one file are not at one time. The Treasury curve is another source and another time. The box rate is not a Treasury rate.
---
Soft: it compares two sources. The source found the box rate about 35 bp above Treasuries (1996-2023).
It tests the rate input of all other checks.
Compare as a separate relation. Do not use it as an input.
