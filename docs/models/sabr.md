# Model: SABR (one smile for each expiry)

Updated: 2026-11-21. Commit: not yet made.

## Purpose
WARNING: the fitted parameters are calibrated values of option prices. They are not measurements of the company.
Do not use them as an input of a DCF or of a forecast.

The model fits one smile for each expiry of a stored option snapshot, in implied volatility. It shows how
alpha (level), rho (skew) and nu (curvature) move the smile. A model volatility is not a market price.
Ticket: `PLANNING/board/T-20261120-vol-2-sabr-localvol.md`, part 1.

## Inputs
| Input | Source | Note |
|---|---|---|
| Quotes, forward, rate, time | The same as `docs/models/heston-bates.md` (`chain.rs`) | The same filters and counts. Real bid and ask only. |
| Beta | `--beta B`, default 1 | Fixed, not fitted. 0 to 1. |

## Method (short)
1. For each series (root and expiry), the implied volatility is `I0 (1 + I1 T)`.
   `I0` is the zero order term of Obloj (2008, arXiv:0708.0998, Table 1, row "I0(x), beta < 1"), `I1` is
   eq. (3) of Obloj (2008), the same as eq. (A.65) / (2.17b) of Hagan, Kumar, Lesniewski and Woodward (2002).
   The ATM form is eq. (2.18) of Hagan et al.
2. The loss is `(model vol - mid vol) / spread in vol`, with the spread floor of 0.05 vol points of the Heston loss.
3. Levenberg-Marquardt (`calib::lm_core`, shared with Heston) for alpha, rho, nu from six fixed starts. The best loss wins.
   Series run in parallel (rayon).
4. Output for each series: days, forward, alpha, rho, nu, the status, the number of quotes, the error in vol points
   (RMS and largest of the model vol minus the mid vol). A parameter within 0.1% of its range from a bound is marked.

## Tests and reference values
- The formula is tested against fixed answers of an independent implementation (Quality and Risk, 2026-11-21; six cases, 1e-12), not against a published value.
  Obloj (2008) has formulas (Table 1) and no numeric cases, so there is no published value to test against.
- Other tests: nu = 0 and beta = 1 gives alpha; the ATM expansion by hand; beta 0 and nu 0 against the Bachelier price inverted with the Black inversion; a round trip of the fit; a fit that cannot converge; the alpha bounds.
- Backtest: none applies, because the model moves no value and has no history.

## Limits of use
- Calibrated values, not measurements. A fit that is "NOT converged" or has a parameter "at bound" is not a result.
- On Alpaca indicative quotes (`quote_kind: indicative`) a difference between the model and a quote is a measurement, not a tradable price.
- Expiries under about 10 days fit badly (SPX 2026-11-20, `--rate 0.04`, SPXW 3 to 7 days: rms 1.4 to 2.5 vol points, largest error 6 to 12.4, nu 4.7 to 7.9), and the status is still "converged".
  The command prints "converged, poor fit" when the largest error is more than 5 times the median spread of the series.
- Fits of 28 days and more have rms 0.14 to 0.41 vol points (same run). Some are above the 0.2 of the ticket.
- No dynamics between expiries: each expiry has its own alpha, rho, nu. Do not read them as one model of the surface.
- The Hagan formula is an asymptotic expansion in T. It can give a negative density at low strikes and long expiries.
- Alpha depends on beta (alpha is about the ATM volatility times `F^(1-beta)`). Compare rho and nu only for the same beta.
- The Obloj term was taken from the note; the paper of Hagan et al. was not read for this code (time term checked against eq. (3) of the note).
- American options (stocks, ETF): the fit is an approximation. The snapshot has no clock time.

## Code and command
| What | Path or command |
|---|---|
| Code | `rust/src/volmodels/sabr.rs`, `cli.rs` |
| Command | `fa vol fit sabr --symbol T --date D [--time HHMM] [--rate R] [--beta B] [--expiry-max DAYS] [--max-spread V]` |
| Tests | `rust/src/volmodels/tests.rs` (`cargo test --manifest-path rust/Cargo.toml --lib volmodels sabr`) |

## Review status
Not reviewed.
