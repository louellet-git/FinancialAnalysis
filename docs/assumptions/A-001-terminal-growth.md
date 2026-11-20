# A-001: terminal growth of the automatic model

Updated: 2026-11-20.

## Choice
The mean of the terminal growth is 2.25% a year, with a standard deviation of 0.75%.
The engine draws it for each path. It caps the draw by the target of the growth process,
by the risk-free rate, and by the cost of capital less 2%.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Automatic path model (`docs/models/auto-path-model.md`) | `rust/src/dcf/auto.rs` (`Rules::default`) | `terminal_growth` = (0.0225, 0.0075) |
| Model file that the builder writes | made by `fa dcf auto TICKER --write FILE` | `[terminal] growth = { mean, std }` |
| Reviewed IBM model | `analysis/IBM/driver_model.toml` | terminal growth driver |

## Alternatives considered
| Alternative | Command | Why not chosen |
|---|---|---|
| 3.0% | `--rules terminal_growth=0.03` | No evidence for a higher long-run growth than 2.25%; it is a convention too |
| The 10-year zero yield (5.13% on 2026-11-13) | `--rules terminal_rf=1` | It makes the fade and the terminal value use one number, but it gives a growth above the long-run economy. The owner has not decided (see below) |

## Why this choice
It is the repo convention of the reviewed IBM driver model. In the IBM driver review, the
mean 2.25% "survives its own mix-weighted-fade math", and the standard deviation was
widened to 0.75% (`analysis/IBM/DRIVER-REVIEW.md`, row `terminal_growth`; closed
decision). The automatic model takes the value from that convention
(`rust/src/dcf/auto.rs`: "repo convention (IBM and ANT files)").
The investigation of 2026-11-20 judged this part a convention, not a defect
(`analysis/priors/dcf_investigation_2026-11-20.md`, section 2). It did not set the value
from the market price.

## Effect on the result
Screen of 438 tickers, change of the median ratio P50 / price (stable set in brackets):
- 3.0%: +0.029 [+0.031]; median change of the P50 +6%.
- The 10-year zero yield: +0.123 [+0.115]; median change of the P50 +20%.
Source: the investigation file, section 2. The effects of several changes do not add.

## Who decided and when
Review of the IBM driver model, 2026-07-16 (round 1, closed decision). Carried to the
automatic model on 2026-11-19. Not changed on 2026-11-20.

## When to examine it again
When the owner decides section 4.7 of the investigation: the fade of the growth goes to
the 10-year zero yield (5.13%), and the terminal growth is 2.25%. The two numbers do not
agree. Examine this record in the same review. A change needs a review of the same
weight as the IBM review (`CLAUDE.md`, source rule 3).
The conventions study of 2026-11-20 gives the evidence and recommends one number, the
lower of the 10-year zero yield and the long-run nominal growth of the economy (3.8%
today, `--rules long_run=0.038`): `analysis/priors/dcf_conventions_study_2026-11-20.md`.
The owner has not decided.
