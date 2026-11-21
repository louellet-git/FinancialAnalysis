# A-100: Nelson-Siegel-Svensson family for the fitted curve

Updated: 2026-11-21. Commit: not yet made.

## Choice
The fitted discount curve is a Nelson-Siegel-Svensson (NSS) curve with six parameters
(b0, b1, b2, b3, tau1, tau2). The zero rate is continuous, actual/365.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Treasury fit | `rust/src/fit.rs` lines 28-61 | `Nss` |
| Rich and cheap, history | `rust/src/richcheap.rs` (`analyse`) | `nss` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Spline | More freedom. It fits the local errors that the measure must show (`fit.rs` lines 8-10). |
| Treasury par curve (bootstrap, `curve.rs`) | Fitted to on-the-run points, not to all securities (`fit.rs` lines 3-5). |

## Why this choice
Code comment `rust/src/fit.rs` lines 3-11: few parameters, so one security cannot bend the
curve. The Federal Reserve Board (GSW) publishes the same family, so `fa forwards` gives an
independent curve of the same kind to compare. No study of other families. Not known: a
test of NSS against a richer family on these data.

## Effect on the result
Not known as a sensitivity. The fit error is the model error of the residual:
mean fit RMSE 1.90 bp (range 1.46 to 2.50) over 58 days, and 2.77 bp for 10Y to 20Y on
2026-11-18 (`analysis/priors/rates_recon_step2_2026-11-20.md`, sections 3.3 and 5).
A synthetic test recovers a known NSS curve (`fit.rs`, test
`recovers_known_parameters_from_synthetic_prices`).

## Who decided and when
Not known. The comment in the code gives the reason. Ticket
`T-20261120-arb-2-rates-recon` (owner `louellet-git`, created 2026-11-20).

## When to examine it again
When the residual of a whole maturity region has one sign for many days (it can be a
curve shape that NSS cannot make), or when a second curve family is added.
