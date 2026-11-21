---
id: T-20261120-vol-2-sabr-localvol
title: Fit SABR smiles and a local volatility surface
area: engine
status: in-progress
owner: louellet-git
head: quant
order: 4
branch: claude/quant-vol-2
created: 2026-11-20
acceptance: `fa vol fit sabr --symbol SPX` gives the alpha, rho and nu of each expiry of the live chain with the error in vol points, and `fa vol localvol` reprices the quoted options with a PDE to within 0.1 vol point.
scope_now: "Part 1 in review (gate PASS-WITH-NOTES, 2026-11-21); part 1 only (louellet-git, quant head, 2026-11-21): `fa vol fit sabr` (Hagan 2002 with the Obloj 2008 correction, beta fixed, alpha rho nu for each expiry, error in vol points). SVI, local volatility and the PDE are later parts, one at a time."
---
Step 2. Depends on: ticket `vol-1-heston-bates` (the model trait, the quote set, the loss). The data check is in section A of the study.

## Model (plain text)

SABR, for one expiry: dF = alpha F^beta dW1; d alpha = nu alpha dW2; corr(dW1, dW2) = rho dt. Parameters alpha, beta, rho, nu. For an equity index beta is fixed (1, or 0.5) and the three others are fitted.
Local volatility (Dupire): dS / S = (r - q) dt + sigma_loc(S, t) dW, with
sigma_loc^2(K, T) = (dC/dT + (r - q) K dC/dK + q C) / (0.5 K^2 d2C/dK2).
In total variance w(k, T) with log-moneyness k: sigma_loc^2 = (dw/dT) / (1 - (k / w) dw/dk + 0.25 (-0.25 - 1/w + k^2 / w^2) (dw/dk)^2 + 0.5 d2w/dk2) (the formula of Gatheral's book; check it against the book in the first step of the work).

## What the owner learns that the earlier steps did not give

Heston makes the smile of all expiries from five numbers, and it fails on the term structure. SABR gives up the term structure: one smile for each expiry, with a closed approximation, so the
fit is exact enough and takes microseconds. The owner learns how the three parameters move the smile, and what the price of the freedom is (no dynamics between expiries). Local volatility is the opposite: it
reproduces the whole surface exactly by construction, but the dynamics are wrong (the future smile flattens: the skew moves the wrong way against the spot). The owner learns the difference between
"fits today's prices" and "has the right dynamics", which motivates the stochastic volatility models of the next steps. The interpolation of the surface without arbitrage (SVI, or the finite-difference
method of Andreasen and Huge) is the practical part: the live chain has 4.7% butterfly noise beyond the spread (study, A.3), so the raw surface cannot be inverted.

## Method

- SABR: the formula of Hagan, Kumar, Lesniewski and Woodward (2002) with the correction of Obloj (2008) for the lognormal-like regime. Fit alpha, rho, nu to the quotes of one expiry with beta fixed: Levenberg-Marquardt, 3 parameters; all expiries in parallel with rayon.
- Surface: fit a raw SVI slice (5 parameters) to each expiry with the conditions of Gatheral and Jacquier (2014) for the absence of butterfly arbitrage, and check the calendar condition. Or the single-step finite-difference method of Andreasen and Huge (2011). Decide in the first step by the number of violations after the fit.
- Local volatility: the Dupire formula on the arbitrage-free surface; or the Andreasen-Huge local volatility directly. The repricing is by a Crank-Nicolson PDE (implicit, 400 x 400 grid) and by Monte Carlo with rayon (antithetic variates, a Black-Scholes control variate) as the independent check.
- Speed target: the SABR fit of 40 expiries in under 0.1 second; the surface and the local volatility grid in under 2 seconds; the PDE repricing of all quotes in under 5 seconds.

## Data and filters (from `analysis/priors/volatility_study_2026-11-20.md`, section A.7)

Series = (root, expiry). Time in minutes from the snapshot time. Forward of each series from put-call parity
(median of 5 strikes). Out-of-the-money quotes. Live: bid above 0, ask above bid, spread at most 1 vol point
(5 for VIX options); weight 1 / spread. History of closes: close above 0, volume at least 50, tick error at most 1 vol point,
and no arbitrage test on closes of different times.

## Tests with fixed answers

1. SABR with nu = 0 and beta = 1: the implied volatility is the constant alpha (to 1e-12). With beta = 0, nu = 0: the normal model (Bachelier) volatility converted to the lognormal volatility (to 1e-8).
2. SABR ATM volatility: the leading term alpha / F^(1 - beta) times (1 + the known correction in T) from the paper (the equation is copied into the test with its number from the paper).
3. The reference values for the Obloj correction: the cases of arXiv:0708.0998 (the test copies the parameters and the volatilities of the paper; read them at the start of the work and cite the table).
4. Local volatility of a flat surface is flat. The local volatility of a Heston surface (ticket 1, known parameters) reprices the Heston call prices with the PDE to within 0.05 vol point (the check of the book of Gatheral, chapter 3, section "Local volatility in the Heston model").
5. A surface that has a butterfly violation is repaired by the SVI conditions: the density is non-negative on the grid (a test with the live-chain slice that has the largest violation).

## Validation on real data

- Fit error by expiry in vol points for SABR (expect under 0.2 vol point for the expiries from 5 days to 2 years on the live chain) and for the SVI slices.
- Stability: alpha, rho, nu of the 30-day expiry over the days of the chain copy; the ATM skew implied by rho and nu against the measured skew of the study (A.4).
- Out of sample: fit day D, move the smile with the "sticky strike" and the "sticky local volatility" rules to day D + 1, and report which rule predicts the next day better (the answer shows the wrong dynamics of local volatility).

## Commands

`fa vol fit sabr --symbol SPX [--beta 1]`, `fa vol surface --symbol SPX --method svi|ah`, `fa vol localvol --symbol SPX`, `fa vol reprice --method pde|mc`.
