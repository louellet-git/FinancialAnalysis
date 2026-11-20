---
id: T-20261120-vol-1-heston-bates
title: Price and fit the Heston and Bates models with Fourier methods
area: engine
status: in-progress
owner: louellet-git
head: quant
order: 2
branch: claude/upbeat-cori-a4ykf5
created: 2026-11-20
acceptance: `fa vol fit heston --symbol SPX` fits the live SPX chain of one day in under 5 seconds, the Heston price test reproduces the published value 5.785155450 to 1e-8, and the fit error by expiry is printed in vol points.
---
Step 1 of the sequence of `analysis/priors/volatility_study_2026-11-20.md`. Depends on: nothing. The data check is in section A of the study.

## Model (plain text)

Heston: dS / S = (r - q) dt + sqrt(v) dW1; dv = kappa (theta - v) dt + xi sqrt(v) dW2; corr(dW1, dW2) = rho dt; v(0) = v0.
Five parameters: v0, theta, kappa, xi, rho. The Feller ratio 2 kappa theta / xi^2 is printed (a value under 1 means that v can reach 0).
Bates: the same, plus jumps of the log price: a Poisson process of rate lambda, log(1 + J) is normal with mean muJ and standard deviation deltaJ; the drift has the
compensator - lambda (exp(muJ + deltaJ^2 / 2) - 1). Eight parameters.

## What the owner learns that the earlier steps did not give

Black-Scholes has one volatility and no smile. Heston makes the volatility random, so the smile comes from two numbers: rho gives the skew and xi gives the curvature,
and kappa sets how fast the smile flattens with the expiry. The new tool is the characteristic function: the price is an integral, not a formula. The owner sees why a
one-factor Markov model fits one expiry well and the whole term structure badly (the short-expiry skew is too flat), and what the jumps of Bates repair (the short wing).
This is the baseline for all later steps.

## Method

- Price: the characteristic function of ln S_T in the stable form of Albrecher, Mayer, Schoutens and Tistaert (2007) (no branch cut of the complex logarithm); one Lewis or Gil-Pelaez integral with adaptive Gauss-Legendre
  for each strike; a Carr-Madan FFT (1999) as an independent check. Bates: multiply by the characteristic function of the jump part.
- Calibration: Levenberg-Marquardt with a numerical Jacobian; the pricing of all strikes of one expiry is one integral grid; rayon over the expiries; 20 random starts around a start from the ATM variance and the skew.
  Parameter bounds: kappa in 0.05 to 20, xi in 0.05 to 3, rho in -0.99 to 0.
- Speed target: one expiry of 100 strikes in under 2 ms; the fit of 40 expiries (11,000 quotes) in under 5 seconds on 4 cores.
- Code: new module `rust/src/volmodels/` (this ticket makes the base: the model trait, the quote set, the loss). The `ratemodels` simulation base (`rng.rs`, `sim.rs`) is used for the Monte Carlo check only.

## Data and filters (from `analysis/priors/volatility_study_2026-11-20.md`, section A.7)

Series = (root, expiry). Time in minutes from the snapshot time. Forward of each series from put-call parity
(median of 5 strikes). Out-of-the-money quotes. Live: bid above 0, ask above bid, spread at most 1 vol point
(5 for VIX options); weight 1 / spread. History of closes: close above 0, volume at least 50, tick error at most 1 vol point,
and no arbitrage test on closes of different times.

## Tests with fixed answers

1. Heston call, S = 100, K = 100, r = q = 0, v0 = 0.0175, theta = 0.0398, kappa = 1.5768, xi = 0.5751, rho = -0.5711: T = 1 gives 5.785155450, T = 10 gives 22.318945791 (to 1e-8).
   Source: T. L. Chan, arXiv:1706.06709v3, equation 105 and appendix B (the numbers were read in the PDF on 2026-11-20).
2. xi near 0 and v0 = theta: the price equals Black-76 with volatility sqrt(v0) (to 1e-10).
3. Put-call parity of the model prices (to 1e-10). The price of the Carr-Madan FFT equals the price of the integral (to 1e-6).
4. Bates with lambda = 0 equals Heston. Bates with xi near 0 equals the Merton jump-diffusion series (a sum of Black-Scholes prices; to 1e-8).
5. Monte Carlo with the quadratic exponential scheme of Andersen (2008) (`ratemodels` base, 2^18 paths) is within 3 standard errors of the Fourier price.
6. A synthetic surface made from known Heston parameters is recovered (parameters within 1%, fit error under 0.01 vol points).

## Validation on real data

- Fit the live SPX chain (`fa vol fit heston --symbol SPX --date 2026-11-20`). Print the error by expiry in vol points (root mean square and the largest), for Heston and Bates. Expected from the study: Heston fits one expiry well and the term structure badly; report the numbers.
- Stability: the parameters on the dates that exist (the live chain of each day from the ticket `vol-7-chain-copy`; the three history dates only as a coarse check). Report the parameters and the change from one day to the next.
- Out of sample: fit day D, price day D + 1 with the same parameters except v0, and report the error in vol points by expiry. It needs two consecutive daily snapshots of bid and ask.

## Commands

`fa vol price heston --params ... --strikes ...`, `fa vol fit heston|bates --symbol SPX [--date D] [--expiry-max DAYS]`, `fa vol check heston` (the fixed-answer tests as a command).
