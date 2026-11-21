# A-008: NVDA terminal return on new capital (D20)

Updated: 2026-11-21. Draft: from commit 5ffe7a0 of `claude/nvda-valuation`.
Source: commit 5ffe7a0, branch `claude/nvda-valuation`. Line numbers below are of that commit.

## Choice
After year 10 the return on new capital is 35% (std 15%, cut 15% to 100%; `terminal.roic`, toml line 303). It is a draw.
The terminal value reinvests growth / return. At the means: 4% / 35% = 11% of NOPAT, not 3% (toml 306-309). The terminal growth is 4.0% (A-001).

## Where used
| Model | File | Parameter name |
|---|---|---|
| NVDA path model (a model document does not exist yet) | `analysis/NVDA/driver_model_paths.toml` line 303 | `terminal.roic` (D20) |

## Alternatives considered
P50 in $ a share, seed 7, 100,000 paths. Base: $103.78. Source: `COMMANDS.md` lines 166-167; report line 188.
| Alternative | P50 | Command | Why not chosen |
|---|---|---|---|
| 15% (the lower cut) | 100.69 | `--pset terminal.roic=0.15` | A bound of the range (author answer, 2026-11-21) |
| 100% (the upper cut) | 107.28 | `--pset terminal.roic=1.0` | Years 1 to 10 imply 110% to 150%: not a base rate for ever (toml 306-309) |
Run: `fa dcf drivers NVDA --model analysis/NVDA/driver_model_paths.toml --sims 100000 --no-attribution --pset terminal.roic=0.15`

## Why this choice
JUDGMENT, stochastic (toml 307-309; report line 93). In years 1 to 10 the model pays for growth at sales to capital 3.0 to 4.0. With a NOPAT margin of 37% to 50% that is a return of 110% to 150% on new capital. The terminal value takes a lower return, because 110% to 150% for ever "is not a base rate". The report says that no data of the repo for the return of large platforms is in the ledger. The value 35% has no source: not known. The report names it a judgment (W9).

## Effect on the result
- P50 at the base: $103.78. The table above gives -$3.09 and +$3.50.
- Move of the P50 for one std of the mean: -2.37 / +1.71 (report line 141).
- The terminal value is 39.9% of the enterprise value at the means and 41.8% over the paths (report line 58).

## Who decided and when
The Head of Fundamental Research, in the NVDA review rounds of 2026-11-20 and 2026-11-21.
Under review: step 2 of the valuation review, not yet accepted. The value can move until the skeptic accepts.

## When to examine it again
- After the skeptic and the reviewer (steps 2 and 3 of the review).
- When a measure of the return on new capital of large companies exists in the repo (the ledger has none).
- When D2, D3 or D12 (sales to capital) change: the return of years 1 to 10 changes with them.
