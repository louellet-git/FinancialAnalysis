---
id: T-20261120-vol-5-quintic-pdv
title: Fit the quintic Ornstein-Uhlenbeck model and the path-dependent volatility model
area: engine
status: backlog
owner: louellet-git
branch:
created: 2026-11-20
acceptance: `fa vol pdv SPY` fits the Guyon-Lekeufack model to the daily SPY and VIX files since 1990 with an out-of-sample R-squared, and `fa vol fit quintic --symbol SPX` fits the live SPX smile with the error by expiry in vol points.
---
Step 5. Depends on: tickets 3 and 4 (base, variance curve, simulation). Data check: study A.7 (the Guyon-Lekeufack part needs no option history).

## Model (plain text)

Quintic OU (Abi Jaber, Illand and Li): dS / S = sigma_t dB; sigma_t = sqrt(xi_0(t)) p(X_t) / sqrt(E[p(X_t)^2]), where p is a polynomial of degree five (odd powers and a constant), dX = - kappa X dt + dW with a fast mean reversion kappa, corr(dB, dW) = rho dt.
The VIX squared is a polynomial of degree 10 in X (an integral of the conditional second moments, in closed form). Parameters: the polynomial coefficients, kappa, rho (6 effective, as the paper says).
The exact form of the normalization is to be copied from arXiv:2212.10917 at the start of the work (I did not read the paper in the study; only the abstract).

Path-dependent volatility (Guyon and Lekeufack, 2023): sigma_t = beta0 + beta1 R1_t + beta2 sqrt(R2_t), R1_t = sum over past days of K1(t - s) r_s, R2_t = sum of K2(t - s) r_s^2,
r_s the daily return, kernels of the form K(tau) = (tau + delta)^(-alpha) / Z (time-shifted power law, Z makes the sum 1). Parameters: beta0, beta1 (negative: the leverage effect), beta2, alpha1, delta1, alpha2, delta2.
The paper reports that past returns explain up to 90% of the variance of the implied volatility of equity indexes. An optional stochastic component adds an exogenous part (the second stage of the work).

## What the owner learns that the earlier steps did not give

Steps 1 to 4 are about the price of options given a model of the variance. Step 5 asks a different question: what drives the variance? The Guyon-Lekeufack model says that the volatility is a function of the past
SPX path, found by a regression, with almost no free noise. The owner learns the Zumbach effect (past returns, not past volatility, drive the volatility), why it matches the leverage effect and the skew of the
smile, and how little data (two daily files) it needs. The quintic model is the opposite design: a single fast OU factor and a polynomial, which makes the VIX squared explicit and gives a fit of SPX and VIX smiles with 6 numbers.
Both are alternatives to the rough models of step 4 that need no roughness.

## Method

- PDV regression: the features R1 and R2 for given (alpha, delta) are linear sums; fit beta0, beta1, beta2 by least squares for each (alpha1, delta1, alpha2, delta2) on a grid, then refine with Nelder-Mead (`ratemodels/math.rs`). The target is the VIX close (the paper uses the VIX for the implied volatility). The sums are exact (a convolution of 9,000 days: milliseconds); for the simulation use a sum of 3 exponentials per kernel so that one step costs O(1).
- PDV option pricing: Monte Carlo of the path-dependent diffusion with the state (R1, R2, spot) per path; rayon and the stream rule of `rng.rs`; the VIX is computed from simulated paths by a nested or regression estimator (decide in the first step).
- Quintic: simulate X exactly (Gaussian), the price by Monte Carlo with the conditional trick of ticket 4; the VIX squared from the closed polynomial; calibrate with the variance curve from ticket 3. Speed target: one price run in 1 second, a fit in under 30 seconds.

## Data

`data/raw/trading/SPY_1d.csv` (from 1993) and `_VIX_1d.csv` (from 1990); the Cboe VIX history file. For the PDV smile step and the quintic fit: the live SPX and VIX chains (filters of study A.7).

## Tests with fixed answers

1. PDV: the kernel sums to 1 (to 1e-12). With beta1 = beta2 = 0 the volatility is the constant beta0 and the price equals Black-Scholes (to 1e-10, conditional price).
2. PDV on synthetic data: generate returns and a VIX from known parameters with noise 1% of the level; the fit recovers each parameter within 5% (seed fixed, 20 samples).
3. Quintic: the moments of X are the Gaussian moments (E[X^2] = 1 / (2 kappa) in the stationary state; the code compares the 10th moment with the exact formula to 1e-10). With a constant polynomial the price equals Black-Scholes. E[sigma^2] equals xi_0 (martingale of the variance swap).
4. Quintic VIX squared: the polynomial formula equals a Monte Carlo of the integral of the variance over 30 days within 3 standard errors.
5. Reference: the R-squared of the VIX regression in the paper. The paper states "up to 90%" (abstract); the test records our value and the value of the paper's table (read at the start of the work).

## Validation on real data

- PDV: out-of-sample R-squared and the error in VIX points: fit 1990 to 2015, test 2016 to 2026; and by sub-period (2008, 2020, 2022, 2025). Report the fit of the SPX smile of the live chain with the PDV model (short expiries).
- Quintic: the error by expiry in vol points on the SPX live chain and the VIX futures error; stability of the 6 parameters over days; out of sample D to D + 1.
- Compare with ticket 4: the roughness of the fitted PDV volatility measured by the `fa vol hurst` estimator on its own simulated paths (does a smooth path-dependent model give an estimate near 0.15, as in study B.3?).

## Commands

`fa vol pdv SPY [--train 1990-2015] [--test 2016-2026]`, `fa vol fit quintic --symbol SPX [--date D]`, `fa vol price pdv|quintic --params ...`, `fa vol check quintic`.
