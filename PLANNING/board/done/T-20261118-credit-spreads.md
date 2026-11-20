---
id: T-20261118-credit-spreads
title: Get corporate bond yields and credit spreads for the cost of debt
area: engine
status: done
owner: merlan1996
branch: bonds-source
created: 2026-11-18
acceptance: `python -m scripts.verdict JNJ --price <price> --rf-date today --rating AAA` prints the market spread for the rating and its date, and the WACC uses that spread.
---
The cost of debt is `kd = rf + kd_spread`. The `kd_spread` of each driver model is a
manual value. The JNJ value (+0.40%) is not calibrated.

Scope:

1. `financial_analysis/bonds.py` gets corporate bond data with no API key. These are
   the ICE BofA spreads and yields for each rating, the Moody's Aaa and Baa yields, and
   the U.S. Treasury HQM corporate bond yield curve.
2. `scripts/verdict.py --rating` sets the mean of `kd_spread` to the market spread for
   that rating. The driver models do not change.
3. `rust/` calls the new commands.
