---
name: point-in-time-test
description: Use when a model, a calibration, a screen or a backtest uses history: it shows that no look-ahead exists. It says how to build the test and how to prove it. Use it for `fa dcf pit`, `fa dcf returns`, `fa backtest`, and for strategy tests.
---

# Point-in-time test

Look-ahead means that the test uses a fact that the real person did not know at the
test date. It makes a model look better than it is.

## Build
1. Set the as-of date. Each input must have been public on that date.
2. Filter facts by the filing date: `filed <= as_of`. Do not use the period end date.
   Note trap #4 of `docs/HIDDEN-ASSUMPTIONS.md`: the stored `filed_date` can come from a
   later filing, so the filter is safe (conservative) but not exact.
3. Prices: use the close of the as-of date or before. Option and estimate data of the
   past exist only if someone saved them. Do not use a present value as a past value.
4. A calibration uses data up to the cut year. The scoring uses the years after it. Use
   `--cut YEAR` of `fa dcf pit`.
5. A universe that lists today's companies has survivorship. Say it in the result.
6. Prices in the store are split-adjusted to today; SEC share counts are as filed. A past value per
   share against a past price needs the split factor between the filing date and the price date. A
   poison test and a truncation test do not find this trap (`docs/traps/`, the split look-ahead). Use the
   engine's share basis (`split_factor`, `docs/models/share-basis.md`); do not write a second one.

## Pre-register
1. Commit the hypothesis, the gate and the decision rule in a commit before the first code commit
   and before the first run output of the study. Count each run and each variant.
2. The reviewer checks the order: `git log --format='%h %ci %s' -- <design file> <code files> <outputs>`.
   A threshold that only a later commit holds is not pre-registered: say so in the result.

## Prove
1. **Poison test.** Change a fact that is dated after the as-of date (make it absurd). The
   output must not change.
2. **Truncation test.** Cut the data at the as-of date and run again. The output must be
   the same as with the full data.
3. **Reproduction.** Run `fa dcf pit --cache data/raw` and compare with the recorded files
   in `analysis/priors/` (`auto_pit_*.txt`).
4. Put the poison test in the test suite when the code is new.

## Do not
- Do not choose a rule on the set that scores it.
- Do not read a model price as a market price in a backtest.

## Check
- The as-of date and the cut year are written in the result.
- The poison test and the truncation test passed.
- The limits (survivorship, filed date) are stated.
