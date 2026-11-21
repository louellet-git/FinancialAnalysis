# A-007: NVDA demand collapse, hazard and recovery multiple (D14)

Updated: 2026-11-21. Draft: commit baf6358 (pull request 106) of `claude/nvda-valuation`.
Source: commit baf6358, branch `claude/nvda-valuation`. Line numbers below are of that commit.

## Choice
- Hazard: 2% a year (std 1%, cut 0 to 8%). It gives 18% in ten years (toml 318, 324). The run has 18.1% of the paths with a collapse (report line 69).
- Recovery multiple: 2.0 times the revenue of the year before the event (`recovery_revenue_multiple`, no unit; toml 319). The event ends the cash flows of its year and later.
- Mode: `sampled` (toml 320).
The engine has no regime state. The failure table of the engine is used as a collapse of demand: the customers cut their capital spend for good. It is not a bankruptcy (toml 321-324).

## Where used
| Model | File | Parameter name |
|---|---|---|
| NVDA path model (a model document does not exist yet) | `analysis/NVDA/driver_model_paths.toml` lines 318-320 | `failure.hazard`, `failure.recovery_revenue_multiple`, `failure.mode` |

## Alternatives considered
P50 in $ a share, seed 7, 100,000 paths. Base: $103.78 (P10 54.92). Source: `COMMANDS.md` lines 44-45, 163-164.
| Alternative | P50 | P10 | Command line |
|---|---|---|---|
| Hazard 0% | 112.50 | a bound of the range | 163 |
| Hazard 4% | 93.91 | a bound of the range | 164 |
| Multiple 0.5 / 1.0 / 1.5 | 101.53 / 102.12 / 102.80 | 41.46 / 46.83 / 51.29 | 44 |
| Multiple 4.0 | 109.98 | 64.31 | 44 |
Each other row is a bound of the range (author answer, 2026-11-21). The first run had no collapse table: P50 145.95 (report lines 260, 266).

## Why this choice
JUDGMENT, stochastic (toml 321; report line 102). Reasons of the toml (321-335):
1. No NVDA precedent. The two busts (FY20, FY23) were mild (-7%, +0.2% revenue).
2. One precedent in the cache: MU, revenue $30.8B to $15.5B (FY22 to FY23, -49.5%), operating margin -37%. The 18% is a JUDGMENT with this one case.
3. The multiple 2.0 is a hand calculation (no engine run): 1.8, rounded to 2.0. It assumes revenue 30% lower, margin 30%, return 35%, cost of capital 12.5%.
4. A multiple of 2.0 assumes a bust milder than MU. The lower tail is set by the multiple: "no source" (toml 332-335; report W4).
The analyst added the table in version 1, after a check of the growth process alone (report line 266; `COMMANDS.md` lines 294-295). The reason is in ledger L27: a brief of the Head of Fundamental Research of 2026-11-20, before the first run. It asked for a downside of the cycle. The brief is from the session and is not in git. The ticket line 21 also names the cycle of the capital spend of the customers. The first run without the table gave P50 145.95, near the close by chance. No input was set from it (report line 266).

## Effect on the result
- Move of the P50 for one std of the hazard: +5.09 / -5.04 (report line 160).
- Lower tail: a collapse in year 1 gives about $31 a share ($30.86; `COMMANDS.md` lines 55-56). No path is below $30; 2.8% are below $40 (line 43).
- Bias: the engine cannot impair the bridge on a failed path (see A-006). The guarantees are not in the model.
- The mean-path table of the output counts failed paths as zero revenue (report W4).
- The hand calculation of the multiple uses a cost of capital of 12.5% (toml 330). The model has 12.55% at the means (report line 112). It uses the ERP 4.41%: a REPO-tier convention (the implied premium of Damodaran, from `analysis/ANT/driver_model_paths.toml`). A premium of 3.5% gives $122.41 and 5.5% gives $88.29 (report line 20).

## Who decided and when
The Head of Fundamental Research, in the NVDA review rounds of 2026-11-20 and 2026-11-21. D14 (hazard 2%, multiple 2.0) is judgment with no source and no separate dated decision; it comes from version 1, 2026-11-20 (author answer, 2026-11-21).
Review: the four steps of the valuation review are complete (validator, document reviewer, auditor; commit baf6358). The owner approval of the new ticker is open.

## When to examine it again
- After the skeptic and the reviewer (steps 2 and 3 of the review).
- When a panel measure exists of the share of company-years with revenue -30% or worse after growth of 50% or more (the 18% rests on one precedent, toml 327).
- When an engine feature that impairs the bridge on a failed path exists.
- The 10-Q of Q3 FY27 (the investigation, report W14; the guarantees, report W6).
