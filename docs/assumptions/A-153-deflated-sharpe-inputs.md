# A-153: deflated Sharpe inputs (number of trials and variance V)

Updated: 2026-11-21. Commit: not yet made.

## Choice
The deflated Sharpe ratio (DSR) of `fa trade backtest --trials N` uses N trials and a variance V of the
daily Sharpe ratios of the trials. The note of 2026-11-21 uses N = 20 (6 runs of that note plus about 14
earlier variants of s05_ic_0dte). V is 1/T by default (T = days of the out-of-sample part), a proxy for the
sampling variance of one Sharpe ratio under no edge. The second figure uses V = 0.00096 (the variance
of the 6 runs, from `--sr-var`). The formula has the Euler constant and returns the probability that the true
Sharpe ratio is above SR0 (Bailey and Lopez de Prado, 2014, as cited in the code comment).

## Where used
| Model | File | Parameter name |
|---|---|---|
| Variance premium test | `rust/src/trading/stats.rs` | `deflated_sharpe` |
| Variance premium test | `rust/src/trading/cli.rs` | `--trials`, `--sr-var`, `print_robust` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| N = 21 | Same run, other N: DSR 0.306 against 0.32 (note) |
| N of all bots and variants seen (about 21 bots, 14 variants: 20 to more than 100) | Not counted. Listed as open item |
| V measured from all trials | Not known. No such count exists |

## Why this choice
N = 20 is a lower bound (note, stage 1b). A larger N gives a lower DSR. So the DSR is an UPPER bound.
V = 1/T is a proxy. V from 6 close variants is too low and makes the DSR too high.
Neither V is measured from all trials (`cli.rs` help text).

## Effect on the result
R1 (baseline): DSR 0.32 with V = 1/T, 0.76 with V = 0.00096. Both are below 0.95 at N = 20
(`trading/research/alpha-1-stage1.md`, stage 1b). No run with N above 20 is known.

## Who decided and when
Not known. The ticket asks for "the trial count" and the DSR. The values are in the note of 2026-11-21.

## When to examine it again
When the count of all bots and variants exists, or when V can be measured from all trials.
