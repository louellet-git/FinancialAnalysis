# A-004: NVDA half-life of the growth fade (D3)

Updated: 2026-11-21. Draft: from commit 5ffe7a0 of `claude/nvda-valuation`.
Source: commit 5ffe7a0, branch `claude/nvda-valuation`. Line numbers below are of that commit.

## Choice
The half-life of the slow part of the growth is 1.0 year (std 0.4, cut at 0.4 and 3.0 years). It is a draw.
Each year the growth moves half-way (one half-life) to the target of 4.0% (A-001). Version 1 had a mean of 1.25.

## Where used
| Model | File | Parameter name |
|---|---|---|
| NVDA path model (a model document does not exist yet) | `analysis/NVDA/driver_model_paths.toml` line 72 | `process.growth.half_life` (D3) |
| The same file, charge fade | line 199, `[[corr]]` lines 370-374 | `process.charge.half_life`: same mean, correlation 0.9 (A-006) |

## Alternatives considered
P50 in $ a share, seed 7, 100,000 paths. Base: $103.78. Source: `COMMANDS.md`.
| Alternative | P50 | Command line | Why not chosen |
|---|---|---|---|
| 0.4 year (the cut; a model that follows the cell above $100B) | 88.75 | 149 | Report: the cell is small and has busts (toml 117-124) |
| 0.58 year (year 2 at +16.4%, the guide pace) | 92.02 | 72, 149 | Report line 178: the charge half-life stays at 1.0 |
| 0.6 year (the panel of all sizes) | 92.45 | 149 | Panel has survivor bias (toml 110-111) |
| 1.25 years (version 1) | 112.75 | 149 | Not known (the report gives no reason for the change) |
| 2.5 years (own momentum, version 1) | 163.14 | 149 | The guide breaks that bound (toml 111-113) |

## Why this choice
JUDGMENT, stochastic (report line 76; toml 108). The report gives two bounds (toml 108-114).
1. The panel of 996 companies (L19, `analysis/priors/process_priors_2026-11-19.toml`): the excess over the long run halves in 0.6 to 0.7 years. This is a lower bound. It has survivor bias.
2. The own momentum was an upper bound in version 1. The guide for Q4 FY27 (+20% to +4% in one quarter) breaks it.
So 1.0 is above the panel and above the speed of the guide. The report says that D3 is above the base rate of scale by $9 to $13 of P50 (toml 122-124) and above the guide pace for year 2 (+24.4% against +15% to +18%; report lines 33, 178). It calls this the price of reading the guide and the sequential growth as evidence.

## Effect on the result
- P50 at the base: $103.78. The range of the table above is $88.75 to $163.14.
- Move of the P50 for one std of the mean: -11.33 / +14.76 (report line 136). Only D2 and the equity risk premium move it more.
- Price implied: the close of $145.98 (2026-11-19) needs 2.08 years (report line 224; `COMMANDS.md` line 244).
- Command for the table: `fa dcf drivers NVDA --model analysis/NVDA/driver_model_paths.toml --sims 100000 --no-attribution --sensitivity process.growth.half_life=0.4,0.58,0.6,1.0,1.25,2.5`

## Who decided and when
The Head of Fundamental Research, in the NVDA review rounds of 2026-11-20 and 2026-11-21.
Under review: step 2 of the valuation review, not yet accepted. The value can move until the skeptic accepts.

## When to examine it again
- After the skeptic and the reviewer (steps 2 and 3 of the review).
- When the Q4 FY27 result exists: add the pair to the guide table (report line 325, L26) and read the sequential growth.
- When the 10-Q of Q3 FY27 is in the data store, and for each new guide of NVDA.
- At the next calibration of the panel (L19), and when a base rate without survivor bias exists.
