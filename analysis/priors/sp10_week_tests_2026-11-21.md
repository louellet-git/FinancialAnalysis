# Evidence for the S&P 10 page: the tests of each reason (week to 2026-11-20)

PRELIMINARY. Ticket `T-20261121-sp100-week-attribution`. Data store 0340bc1; binary of `claude/attribution-reality`.
Runs: `fa dcf attribute --tickers-file rust/mapcheck/universe/sp100.txt --from 2026-11-13 --to 2026-11-20` (the week) and
`--from 2026-10-30 --to 2026-11-13` (the weeks before); daily closes of the store. The sums below are shell calculations
on these outputs (awk), not yet engine code (design R6 of `T-20261121-dcf-engine-integration`). The rest of a stock = its
change less its beta times the market; the equal weight of the S&P 500 (-6.56% in the week) is the market here.
A result has one of three forms: proven, refuted, or tested with no cause found.

## T1. "A stock that needs less growth had less to lose" (BRK-B, and the 10 as a group)
Claim: the implied 10-year growth of 2026-11-13 ranks the falls of the week. 82 S&P 100 companies with a growth value.
- Implied growth against the change: r = -0.47 (t -4.8). Against the rest: r = -0.27 (t -2.5); rank correlation -0.25.
- Thirds by implied growth: lowest (below 4.8%) change -5.4%, rest -0.7; middle (4.8% to 12.4%) -5.5%, -0.5; highest
  (14% and more) -11.2%, -3.1.
- The same without the sector of computers, chips and software (59 companies): r = -0.09 (t -0.7).
Result: REFUTED as a cause of its own. The relation is the sector: outside it, the needed growth does not rank the falls.

## T2. "The price of AAPL had not risen with its annual report of 2026-11-03"
- AAPL closes: 331.90 (11-03), 356.27 (11-13): +7.3%. SPY: 754.35 to 754.37: 0.0%. The price DID rise after the report.
- New facts in the weeks before (10-30 to 11-13) against the rest of the week, 79 companies: r = +0.06 (t 0.5). Only 4
  companies have a facts part above 2% of the price: AAPL +27.8% (rest +3.3), EMR +2.6% (-2.4), SBUX -25.1% (+3.3), QCOM
  -51.4% (-4.2): no pattern.
- Does AAPL fall less than its beta in falling weeks? The 4 weeks of the 103 before with SPY at -2.5% or lower: mean rest
  -1.3% (positive in 2 of 4). No.
Result: REFUTED. AAPL: tested (new facts, a habit in falling weeks, the needed growth of T1), no cause found. Its rest is
+1.0 deviation against the equal weight: the level of chance (about 3 of 10 stocks are outside 1 deviation by chance).

## T3. "NVDA was hit more because of its own news (the report of 2026-11-17)"
- The 7 chip companies of the S&P 100 (SIC 3674): week, rest: MU -23.6%, -13.0; NVDA -22.9%, -11.9; AMD -20.5%, -10.0;
  AMAT -18.3%, -8.4; INTC -14.0%, -4.7; TXN -13.1%, -6.0; AVGO -12.3%, -2.0. Mean -17.8%, rest -8.0. MU and AMD fell as
  much as NVDA with no report in the week.
- The 6 chip companies without NVDA, by day (equal weight): -5.4, +0.2, +1.1, -2.7, -11.0 (Friday). NVDA: -4.1, -0.4,
  -2.9, -3.3, -14.0. The report came after the close of 11-17: the chips rose on 11-18 and fell three days later.
- NVDA less the mean of the other 6 chips: about -5.9 points in the week (-3.0 on Friday), about 1.5 of its deviations.
Result: REFUTED as the main reason. The fall of NVDA is the fall of its sector (PROVEN: 7 of 7 chips, mean rest -8.0).
The part of NVDA alone is near the level of chance. The cause of the chip fall of 2026-11-20: tested (filings of
earnings, rates, the timing of the NVDA report), no cause found in our data.

## T4. AMZN and BRK-B, "hit less"
- Against the equal weight, the rest is +0.3 deviation (AMZN) and +0.4 (BRK-B). In the 4 falling weeks before: AMZN mean
  rest -1.7%, BRK-B +0.9% (positive in 2 of 4 each).
Result: no effect to explain. Both are inside the level of chance when the market does not contain the stock.

## T5. "The 10 fell more than the average stock"
- 6 of the 10 are in the sector of computers, chips and software (S&P 100 sector, value-weighted: -11.9% in the week;
  the chips: -17.8%). The 4 others: AMZN -7.4%, TSLA -11.4%, LLY -6.7%, BRK-B -3.9%.
Result: PROVEN as a sector effect, with T1: the group fell more because of its sector, not because of its size.

## Limits
One week and about 80 companies: a correlation here is a description, not a law. The betas and the deviations are measured
against SPY (104 weeks). The lists are today's names. The value at the means is not the expected value.
