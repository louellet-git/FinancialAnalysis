# A-004: NVDA half-life of the growth fade (D3)

Updated: 2026-11-21. Draft: commit baf6358 (pull request 106) of `claude/nvda-valuation`.
Source: commit baf6358, branch `claude/nvda-valuation`. Line numbers below are of that commit.

## Choice
The half-life of the slow part of the growth is 1.0 year (std 0.4, cut at 0.4 and 3.0 years). It is a draw.
Each year the growth moves half-way (one half-life) to the target of 4.0% (A-001). Version 1 had a mean of 1.25.

## Where used
| Model | File | Parameter name |
|---|---|---|
| NVDA path model (a model document does not exist yet) | `analysis/NVDA/driver_model_paths.toml` line 72 | `process.growth.half_life` (D3) |
| The same file, charge fade | line 200, `[[corr]]` lines 371-375 | `process.charge.half_life`: same mean, correlation 0.9 (A-006) |

## Alternatives considered
P50 in $ a share, seed 7, 100,000 paths. Base: $103.78. Source: `COMMANDS.md`.
| Alternative | P50 | Command line | Why not chosen |
|---|---|---|---|
| 0.4 year (the cut; a model that follows the cell above $100B) | 88.75 | 152 | Report: the cell is small and has busts (toml 116-125) |
| 0.58 year (year 2 at +16.4%, the guide pace) | 92.02 | 75, 152 | Only D3 changes: the charge half-life stays at 1.0 (`COMMANDS.md` line 75) |
| 0.6 year (the panel of all sizes) | 92.45 | 152 | Panel has survivor bias (toml 109-112) |
| 1.25 years (version 1) | 112.75 | 152 | The upper bound of the old range (own momentum, 2.5 years) was void: the guided sequential growth fell from about +20% to +4% a quarter. 1.0 is the analyst judgment inside the range, accepted by the Head of Fundamental Research (version 2, 2026-11-21). |
| 2.5 years (own momentum, version 1) | 163.14 | 152 | The guide breaks that bound (toml 112-114) |

## Why this choice
JUDGMENT, stochastic (report line 91; toml 109). The report gives two bounds (toml 109-115).
1. The panel of 996 companies (L19, `analysis/priors/process_priors_2026-11-19.toml`): the excess over the long run halves in 0.6 to 0.7 years. This is a lower bound. It has survivor bias.
2. The own momentum was an upper bound in version 1. The guide for Q4 FY27 (+20% to +4% in one quarter) breaks it.
So 1.0 is above the panel and above the speed of the guide. The report says that D3 is above the base rate of scale by $11.3 to $15.0 of P50 (report line 211; toml 124-125) and above the guide pace for year 2 (+24.4% against +15% to +18%; report lines 52, 282). It calls this the price of reading the guide and the sequential growth as evidence.

## Effect on the result
- P50 at the base: $103.78. The range of the table above is $88.75 to $163.14.
- Move of the P50 for one std of the mean: -11.33 / +14.76 (report line 158). Only D2 and the equity risk premium move it more.
- Price implied: the close of $145.98 (2026-11-19) needs 2.08 years (report line 230; `COMMANDS.md` line 247).
- The base of D2 (year-1 growth) is the guide (41.0%) plus half the median beat of 19 pairs (+2.78%): 44.9% (toml 89-102). The mean of the 12 pairs of Q4 FY24 to Q3 FY27 is 6.13% (`COMMANDS.md` line 207; report line 185).
- The cost of capital has the ERP 4.41%. It is a REPO-tier convention (the implied premium of Damodaran, from `analysis/ANT/driver_model_paths.toml`; ledger L17). A premium of 3.5% gives $122.41 and 5.5% gives $88.29 (report lines 20, 170).
- Command for the table: `fa dcf drivers NVDA --model analysis/NVDA/driver_model_paths.toml --sims 100000 --no-attribution --sensitivity process.growth.half_life=0.4,0.58,0.6,1.0,1.25,2.5`

## Who decided and when
The Head of Fundamental Research, in the NVDA review rounds of 2026-11-20 and 2026-11-21.
Review: the four steps of the valuation review are complete (validator, document reviewer, auditor; commit baf6358). The owner approval of the new ticker is open.

## When to examine it again
- After the skeptic and the reviewer (steps 2 and 3 of the review).
- When the Q4 FY27 result exists: add the pair to the guide table (report line 309, L26) and read the sequential growth.
- When the 10-Q of Q3 FY27 is in the data store, and for each new guide of NVDA.
- At the next calibration of the panel (L19), and when a base rate without survivor bias exists.
