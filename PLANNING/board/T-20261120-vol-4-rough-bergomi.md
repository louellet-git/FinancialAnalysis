---
id: T-20261120-vol-4-rough-bergomi
title: Simulate rough Bergomi, fit it, and estimate the roughness
area: engine
status: backlog
owner: louellet-git
head: quant
order: 6
branch:
created: 2026-11-20
acceptance: `fa vol fit rbergomi --symbol SPX` fits H, eta and rho to the live SPX smile in under 30 seconds with the error by expiry in vol points, and `fa vol hurst SPY` repeats the roughness estimate of the study (0.147 on the 5-minute bars) with its bootstrap error.
---
Step 4. Depends on: tickets 1 and 3 (base, variance curve, Monte Carlo layout). Data check: study A.4 (the skew power law), Part B (the 5-minute bars).

## Model (plain text)

dS / S = sqrt(V_t) dB. V_t = xi_0(t) exp( eta W~_t - 0.5 eta^2 t^(2H) ), W~_t = sqrt(2H) integral from 0 to t of (t - s)^(H - 1/2) dW_s, corr(dB, dW) = rho dt.
Parameters: H (0 to 0.5), eta, rho. The curve xi_0(t) is the input from ticket 3. W~ is a Volterra process: it is not a Markov process and not a semimartingale for H under 1/2. Var(W~_t) = t^(2H).
For H = 1/2 the model is the one-factor Bergomi model.

## What the owner learns that the earlier steps did not give

All earlier models are Markov: the future depends on the state only. Here the future depends on the whole past of the noise. The reward is a short-expiry ATM skew that behaves like T^(H - 1/2),
which is what the live chain shows (slope -0.40, study A.4) with 3 parameters; the cost is that there is no PDE, no closed Fourier price, and the simulation needs a special scheme.
The owner also learns why the claim "volatility is rough" is hard to test on time series (study Part B: a smooth two-factor model gives the same scaling estimate), and so why the option
data must decide.

## Method

- Simulation: the hybrid scheme of Bennedsen, Lunde and Pakkanen (2017) with kappa = 1 (exact for the first steps of the kernel; the rest as a Riemann sum), the convolution by FFT; 2^15 to 2^17 paths; 100 to 250 steps per year; rayon over the paths with the stream rule of `ratemodels/rng.rs`.
- Variance reduction (McCrickerd and Pakkanen, arXiv:1708.02563): conditional Monte Carlo given the variance path (the price is a Black-Scholes price with a corrected spot and volatility); antithetic paths; the control variate from the exact mean of V; quasi-random numbers for the first time steps.
- Calibration: fit (H, eta, rho) on a grid of H (0.02 to 0.5) with common random numbers; the loss as in ticket 3. Speed target: one price run (8 expiries, 2^15 paths) in 1 second; full fit in under 30 seconds on 4 cores.
- The roughness estimate in Rust (`fa vol hurst`): the moment regression of Gatheral, Jaisson and Rosenbaum (2018) with the block bootstrap, the noise-aware fit, 1-minute and 5-minute sampling, the range estimators; the same numbers as the scratch code of the study (Part B). The simulated checks of section B.3 become tests.

## Data and filters (from `analysis/priors/volatility_study_2026-11-20.md`, section A.7)

Series = (root, expiry). Time in minutes from the snapshot time. Forward of each series from put-call parity
(median of 5 strikes). Out-of-the-money quotes. Live: bid above 0, ask above bid, spread at most 1 vol point
(5 for VIX options); weight 1 / spread. History of closes: close above 0, volume at least 50, tick error at most 1 vol point,
and no arbitrage test on closes of different times.

## Tests with fixed answers

1. Var(W~_t) = t^(2H) (the exact value; Monte Carlo within 3 standard errors for H = 0.1, 0.3 and 0.5) and E[V_t] = xi_0(t).
2. H = 1/2: the price equals the one-factor Bergomi price of ticket 3 (theta = 0) within 3 standard errors.
3. The reference smile of Bayer, Friz and Gatheral (2016), Quantitative Finance 16(6), 887-904: the test copies the parameters and the volatilities from the paper (read the table at the start of the work; the numbers are not in this ticket because I did not read the paper). The price must be within 0.2 vol point.
4. The Hurst estimator on simulated rough paths and smooth paths gives the numbers of the study (B.3): H = 0.5 gives 0.497 +- 0.025; H = 0.1 gives 0.100 +- 0.011; AR(1) with rho 0.7 gives 0.168 +- 0.015 (100 paths of 1,574 days, seed fixed).
5. The skew of the model at short expiries follows T^(H - 1/2) within the error of the leading order formula (the formula of the paper in the test).

## Validation on real data

- Fit error by expiry in vol points on the live SPX chain, against ticket 3. The question: does 3 parameters (rough) beat 7 (smooth two-factor) on the short expiries? Report the numbers of both.
- Stability of H, eta, rho over the days of the chain copy. H from the options (expect 0.05 to 0.15) against H from the 5-minute bars (0.15); a difference is a result.
- Out of sample: fit D, price D + 1 with a new xi_0.
- The roughness estimate: repeat on the consolidated bars if a second source is available (study B.4), and with the corrected estimators of Fukasawa, Takabatake and Westphal (2022) and Cont and Das (2024) if the work has time. Say what the data cannot show.

## Commands

`fa vol fit rbergomi --symbol SPX [--date D]`, `fa vol price rbergomi --params ...`, `fa vol hurst SPY --interval 5m|1m|1d [--method gjr|noise|range]`, `fa vol check rbergomi`.
