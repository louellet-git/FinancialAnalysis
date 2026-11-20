# Responsibilities: Quant Research and Fundamental Research
One page for the split of work between the two research heads. Written 2026-11-20.
Source: the words of the person, 2026-11-20 (`../RECORD.md`), with a screenshot of a search result
that defines the two kinds of research. The file table of `chiefs/ORIENTATION.md`, section 2, stays
correct for the files. This page says who owns which question.

## The definitions that the person gave
- Fundamental research: the intrinsic value of one company or one asset, from its financial
  statements, its business model, its competitive advantages, its management and the economy. Few
  assets, studied in depth, with human judgment.
- Quantitative research: formulas, statistics and models that score or trade large sets of assets
  on numerical factors (valuation ratios, momentum, volatility). Systematic rules, speed, broad
  coverage.

## Quant Research owns
1. **Arbitrage and reconciliation.** Find and measure price differences that the rules of no
   arbitrage do not permit, or that two sources do not agree on:
   - option chains: put-call parity, vertical spread, butterfly and calendar bounds;
   - the rate in a box spread against Treasury bills and SOFR;
   - a future against its spot and the cost of carry (basis);
   - a bond or a bond index against the fitted curve (rich and cheap);
   - one price source against a different price source for the same asset.
2. **Pricing models.** Rates (curves, short-rate models) and volatility (surfaces, stochastic
   volatility). The models give the fair price that the arbitrage checks use.
3. **Signals and factors on many assets.** Systematic scores on a universe (for example the 1,249
   tickers of the store): momentum, volatility, and valuation ratios. A factor from the value gap
   of the automatic DCF model is a quant signal; the model that makes the value is fundamental.
4. **Tests of signals.** Point-in-time backtests, out-of-sample tests, costs, the test of a signal
   against the later returns.
5. **Option strategies.** The option bots, the backtest, the paper trader.
6. **Risk and size.** Risk models and position sizes for a set of positions.

## Fundamental Research owns
1. **The value of one company.** The DCF engines, the driver models, the ticker reports and their
   four-step review.
2. **The judgments that are not numbers in a file.** Business model, competitive advantage,
   management, and the effect of the economy on one company.
3. **Models for company classes.** Banks, insurers, REITs, utilities, funds.
4. **The automatic model as a value engine.** It gives one value range for each ticker. Its
   rules, its calibration and its refusals stay fundamental.

## The boundary
- Fundamental makes the value of a company. Quant uses many such values as one factor, ranks them
  with other factors, tests them against returns, and sizes positions.
- `fa dcf screen`, `fa dcf pit` and `fa dcf returns` are in `rust/src/dcf/` (fundamental files).
  The engine stays fundamental. The use of their output as a factor across the universe is quant.
  A move of a file needs the person and a change of `chiefs/ORIENTATION.md` (the Head of
  Technology owns its structure).
- A trade idea from one ticker report (fundamental) and a trade idea from a screen (quant) are
  different. Each head reviews its own idea.

## Open
- The person has not decided if `fa dcf returns` (a factor test) moves to the quant area.
