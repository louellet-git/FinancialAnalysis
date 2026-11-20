---
id: T-20261120-vol-3-bergomi-2f
title: Fit a two-factor Bergomi forward variance model
area: engine
status: backlog
owner: louellet-git
head: quant
order: 5
branch:
created: 2026-11-20
acceptance: `fa vol fit bergomi2f --symbol SPX` fits the forward variance curve from the VIX strip of each live expiry, then the volatility of volatility and the correlations to the SPX smile in under 10 seconds, with the error by expiry in vol points.
---
Step 3. Depends on: tickets 1 and 2 (the base), and the variance strip code of this ticket. The data check is in section A.5 of the study.

## Model (plain text)

The state is the forward variance curve xi_t^T (the price at time t of the variance of the instant T). Two Gaussian factors:
dX = - k1 X dt + dW1, dY = - k2 Y dt + dW2, corr(dW1, dW2) = rho12 dt.
xi_t^T = xi_0^T exp( omega ( (1 - theta) e^(-k1 (T - t)) X_t + theta e^(-k2 (T - t)) Y_t ) - 0.5 Var_t ), with Var_t the variance of the exponent, so that xi_t^T is a martingale.
Spot: dS / S = sqrt(xi_t^t) dB, corr(dB, dW1) = rho1, corr(dB, dW2) = rho2. Parameters: omega, theta, k1, k2, rho12, rho1, rho2 (7). The curve xi_0^T is an input, not a parameter.
(These are the equations in the form of the two-factor model of Bergomi (2005, 2016). The first step of the work is to check each equation against the book, chapter on forward variance models, and to write the page in the code comment.)

## What the owner learns that the earlier steps did not give

In Heston and SABR the variance is a state with its own dynamics; the market curve of variance swaps can be matched only approximately. In a forward variance model, the market curve xi_0^T is the
input, so the model matches every variance swap and the VIX futures level exactly, and only the dynamics are left to fit. The owner learns how the term structure of the skew and the term structure of the
volatility of volatility come from two exponentials, why one factor cannot fit both (the one-factor skew falls as 1 / T at long expiries, the market's falls more slowly), and how to read the forward variance curve out of the option strip with the VIX formula.

## Method

- The curve: for each series, the model-free variance of the strip with the formula of the Cboe VIX document (study A.5), but for the expiry of the series (not 30 days). Interpolate the total variance in the expiry (linear in w, monotone). The strikes: all live strikes (the wings carry 18% of the VIX, study A.5).
- Simulation: (X, Y) have an exact Gaussian transition; one path of 250 steps per year; the spot in log-Euler with the variance of the step. Variance reduction: antithetic paths; a control variate from the exact variance swap strike (the model price of the variance swap is known: the integral of xi_0); the conditional Monte Carlo of Romano and Touzi (integrate the part of the spot that is independent of the variance in closed form). Common random numbers in the calibration. rayon over the paths with the `ratemodels` stream rule (`Rng::for_path`).
- Calibration: Nelder-Mead from `ratemodels/math.rs` for the first run, Levenberg-Marquardt after that; the loss is the weighted vol error of the SPX quotes at 6 to 10 expiries (5 days to 1 year).
- Speed target: 2^15 paths on 8 expiries: 0.5 second for one price run; a full calibration in under 10 seconds on 4 cores (about 20 price runs with common random numbers).

## Data and filters (from `analysis/priors/volatility_study_2026-11-20.md`, section A.7)

Series = (root, expiry). Time in minutes from the snapshot time. Forward of each series from put-call parity
(median of 5 strikes). Out-of-the-money quotes. Live: bid above 0, ask above bid, spread at most 1 vol point
(5 for VIX options); weight 1 / spread. History of closes: close above 0, volume at least 50, tick error at most 1 vol point,
and no arbitrage test on closes of different times.

## Tests with fixed answers

1. omega = 0: the model is Black-Scholes with the integrated forward variance; the price equals Black-76 with the volatility sqrt(mean of xi_0) (to 1e-10, not Monte Carlo: the conditional price).
2. Martingale: E[xi_t^T] = xi_0^T within 3 standard errors for 10 pairs (t, T). Variance swap: the model strike of any expiry equals the integral of xi_0 (to 3 standard errors; with the control variate to 1e-4 relative).
3. One factor (theta = 0): the variance of ln xi_t^T is omega^2 e^(-2 k1 (T - t)) (1 - e^(-2 k1 t)) / (2 k1) (the exact formula, to 1e-10 on the grid).
4. With no vol-of-vol the VIX future equals the square root of the forward variance over 30 days (to 1e-10).
5. The ATM skew at short expiries against the leading order formula of the book (the formula and its page are written in the test; to 5%).

## Validation on real data

- Fit error by expiry in vol points on the live SPX chain; expect a good fit for 2 to 3 expiries and a clear failure on the short expiry (under 10 days) skew: report it as the motivation for ticket 4.
- Stability of omega, theta, k1, k2, the correlations over the days of the chain copy.
- Out of sample: fit day D, move xi_0 with the new strip of day D + 1 (the model does not refit the dynamics), and compare the SPX smile of D + 1 with the model smile. The error by expiry in vol points.
- The two-factor model is over-parameterized in practice (a lecture source says so); report the correlation matrix of the parameter estimates and which parameters are not identified.

## Commands

`fa vol varcurve --symbol SPX` (the forward variance from the strip), `fa vol fit bergomi2f --symbol SPX [--date D]`, `fa vol price bergomi2f --params ...`, `fa vol check bergomi2f`.
