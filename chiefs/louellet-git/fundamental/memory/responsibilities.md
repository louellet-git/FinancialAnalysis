# Responsibilities of the Head of Fundamental Research (louellet-git)
One page: what this head owns, and where its work stops. Written 2026-11-20, after the outline of the
Head of Quant Research of the same date (`chiefs/louellet-git/quant/memory/responsibilities.md`).
The person decides each boundary. A move of a file needs the person.

Definition used: fundamental research is the intrinsic value of one company, in depth, with judgment.

## What this head owns
1. **The value engines.** The classic, driver and path engines (`rust/src/dcf/`), and their correctness:
   the cash flow, the discount, the terminal value, the bridge, the share count.
2. **The automatic model as the value engine.** Its rules, its calibration from the panel, its refusals,
   and its assumptions. Each assumption has a record (`docs/assumptions/`) with a study and a decision.
3. **The tests of the value model.** The point-in-time test of revenue and cash flow (`fa dcf pit`), and
   the measurement of the value against later returns (`fa dcf returns`) as evidence about the model.
4. **What the price implies** (`fa dcf implied`): for each company, sector and industry, the cost of
   capital, the growth and the margin that the price needs. Owner request of 2026-11-20. It is a
   measurement of the market with the value engine, not a signal.
5. **Models for company classes** that a cash-flow DCF does not fit: banks, insurers, REITs, utilities,
   funds, companies with no revenue.
6. **Ticker work.** Driver models and reports of one company (`analysis/<TICKER>/`), with the four-step
   review, and the judgments that are not numbers: the business, the advantage, the management.

## Where the work stops (the boundary with the Head of Quant Research)
- This head makes the value of a company and the numbers that the price implies. The Head of Quant
  Research can use many of these values as one factor, rank them with other factors, test a rule that
  trades them, and size positions. That use is quant work.
- A test of the value against returns is evidence about the value model here. The same test as a test
  of a trading rule is quant work. The commands stay in `rust/src/dcf/`.
- A strategy for the portfolio backtest (`T-20261120-portfolio-backtest`) from this head is a list of
  few companies with a value and a reason, not a factor on many tickers.

## Agreement with the outline of the Head of Quant Research
I agree with each line of it. One addition: `fa dcf implied` (the values that the price implies by sector)
is in this area, because it moves the inputs of the value engine; only its use as a factor is quant.
