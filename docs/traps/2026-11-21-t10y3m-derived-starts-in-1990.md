# T10Y3M from the Treasury curves starts in 1990

*Trap:* `fa rates derived` computes T10Y3M from the Treasury nominal curves. These start in 1990
(`treasury_first_year`). The FRED T10Y3M starts on 1982-01-04. A test of the years 1982 to 1989
with the derived series has no data.

*What to do:* For 1982 to 1989, use the FRED file of the private store, or do not use T10Y3M. DFII10
and T10YIE start in 2003 in both sources. The derived files have no line on a day without a Treasury
value; FRED writes an empty cell on such a weekday.

*Evidence:* `rust/src/rates_derived.rs`; comparison on 2025-01-02 to 2026-11-20 (472 days each,
all equal), 2026-11-21.
