# A-121: strike filter of the chain check (0.99 F and 1.01 F)

Updated: 2026-11-21. Commit: not yet made.

## Choice
With the filter on, a call stays when K >= 0.99 F and a put stays when K <= 1.01 F. F is the parity forward of the series (A-126).
Default: on for a European index, off for an American chain (`--otm` turns it on; `--all-strikes` turns it off).
A series with no forward is not filtered.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Static arbitrage check | `rust/src/market/option_arb.rs:295-317` (factors at 305) | `CheckOpts.otm` |
| Static arbitrage check | `rust/src/market/options_cli.rs:424` | `otm` |
| Option history, fill from mids | `rust/src/market/option_history.rs:186`, `355` | `usable` (same factors) |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| All strikes | Not known. Kept as the default for American chains and as `--all-strikes`. |
| Strictly out of the money (K >= F for a call) | Not known. The Heston fit uses it (`volmodels/chain.rs:105-108`). |

## Why this choice
Reason in the help text: the deep in-the-money quotes of the indicative feed are modified and wide (`CHECK_USAGE`, `options_cli.rs:76`).
The band of 1 % is the band of the option history (`option_history.rs:185`: the early-exercise premium of an in-the-money American option is not in the model).
The help text says the rule is "the rule of the volatility fit". This is not right: the fit of `volmodels` keeps out-of-the-money quotes with no 1 % band. A person must correct the text or the rule.
No study of the width 1 %: author choice.

## Effect on the result
The report prints the quotes removed and the breaks that have a removed leg (`removed_leg_breaks`). Run: `fa options check SPX --date D --all-strikes` against the default on a stored SPX file. No number is recorded here.

## Who decided and when
Agent `volatility-derivatives-quant` (ticket `T-20261120-arb-1b-chain-check-depth` family), date not known. No person named.

## When to examine it again
When a check on a European index gives many breaks in deep in-the-money quotes; when the text of the fit rule is corrected.
