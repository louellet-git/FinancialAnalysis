---
id: T-20261120-vol-6-joint-spx-vix
title: Fit SPX and VIX smiles together and compare the models
area: engine
status: backlog
owner: louellet-git
head: quant
story: E4.S3
order: 8
branch:
created: 2026-11-20
acceptance: `fa vol joint --model rbergomi|bergomi2f|quintic --date D` fits the SPX smile, the VIX futures and the VIX options of one live day with one parameter set, and `fa vol compare` prints one table of the models with the error in vol points for each instrument.
---
Step 6. Depends on: tickets 3, 4, 5 and the chain copy of ticket 7 (same-time SPX and VIX chains, with the snapshot time and the VX settlement of the day). Data check: study A.5 and A.6.

## Model (plain text)

No new model. One parameter set, one loss. The loss is the sum of three parts, each as squared errors divided by the half spread (in vol points for the options, in VIX points for the futures):
(1) SPX out-of-the-money quotes of 6 to 10 expiries; (2) the VIX futures (the monthly VX settlements; the weekly contracts in the Cboe file carry the monthly price, #73, so they are left out); (3) the VIX options of the monthly expiries (the VIX forward is the future of the same expiry).
The model VIX at T is sqrt( (1 / D) integral from T to T + D of E_T[V_u] du ) with D = 30 days; the future is its expectation, the option is a call or put on it.

## What the owner learns that the earlier steps did not give

The SPX smile fixes the law of the spot. The VIX futures and options fix the law of the future variance. A model must fit both with one set of numbers. This is the hard problem of the field. Classical
models fail it (the VIX smile is too steep for Heston-type models with an SPX fit); the rough models help but the short VIX expiry stays hard; the quintic model is made for it. The owner learns the
VIX convexity (Jensen: the future is below the root of the expected variance), how the VIX smile constrains the volatility of volatility that the SPX smile hides, and why one fits both (the paper of Guyon, 2020, shows a no-arbitrage condition between the two markets).
He also sees which published claim survives on our data: that the joint fit needs roughness (Gatheral, Jusselin, Rosenbaum, 2020) or that a classical two-factor model suffices (Romer, 2022).

## Method

- Pricing: the VIX squared of each model at the dates T of the VIX expiries; for the models without a closed VIX (ticket 4), a nested Monte Carlo is too slow, so use the regression or the Markovian approximation of the paper (decide in the first step; target: a VIX future in 0.5 second).
- Calibration: the same optimizer for all models, 5 random starts, common random numbers. Speed target: one model, one day, in under 60 seconds on 4 cores.
- The table `fa vol compare` has one line for each model: the number of parameters, the SPX error (root mean square in vol points, by expiry bucket), the VIX futures error (points), the VIX option error (vol points), the time of the fit, and the change of the parameters from one day to the next.

## Data and filters (from `analysis/priors/volatility_study_2026-11-20.md`, section A.7)

Series = (root, expiry). Time in minutes from the snapshot time. Forward of each series from put-call parity
(median of 5 strikes). Out-of-the-money quotes. Live: bid above 0, ask above bid, spread at most 1 vol point
(5 for VIX options); weight 1 / spread. History of closes: close above 0, volume at least 50, tick error at most 1 vol point,
and no arbitrage test on closes of different times.

## Tests with fixed answers

1. Synthetic data: make SPX and VIX prices from each model with known parameters; the joint fit recovers the parameters within 2% and the error under 0.05 vol point.
2. No vol-of-vol: the VIX future is the square root of the forward variance over 30 days (to 1e-10); the VIX option price is the Black-76 price on it.
3. Jensen: for each model and expiry, VIX future is at most the square root of the model variance-swap forward over 30 days (the inequality holds in each run; a failure is a code error).
4. Put-call parity of the model VIX options with the model future (to 3 standard errors).
5. The SPX options of the joint fit equal the SPX options of the single fit (ticket 3, 4 or 5) when the weight of the VIX part is 0 (to 1e-6).

## Validation on real data

- Run on every day of the chain copy that has the SPX chain, the VIX chain and the VX settlement. Report by model: the errors, the parameters, the failures.
- Stability: the parameters over the days. A model that needs different parameters each day is not a model.
- Out of sample: fit day D, price D + 1 (SPX smile, VIX futures, VIX options) with a new xi_0 (or the new state).
- The joint-fit test of the history of closes is only a coarse check (study A.6: 5 to 8 VIX series on the history dates): do not draw a conclusion from it.
- The result in one sentence for the owner: which model fits the SPX and VIX smiles together on our data, with what error, and which one is the best for each instrument.

## Commands

`fa vol joint --model M --date D`, `fa vol compare [--dates D1,D2,...]`, `fa vol check joint`.

Needed by (2026-11-21): none; the model-free VIX relations of the register (VIX-REPLICATION, VX against the variance swap) do not need a model. No check or strategy of the owner needs this model now, so it goes after the work that serves a decision of the owner (Head of Staff, 2026-11-21).
