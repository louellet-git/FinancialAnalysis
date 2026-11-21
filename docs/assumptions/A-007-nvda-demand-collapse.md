# A-007: NVDA demand collapse, hazard and recovery multiple (D14)

Updated: 2026-11-21. Draft: from commit 38fbbdb (values as in 5ffe7a0) of `claude/nvda-valuation`.
Source: commit 5ffe7a0, branch `claude/nvda-valuation`. Line numbers below are of that commit.

## Choice
- Hazard: 2% a year (std 1%, cut 0 to 8%). It gives 18% in ten years (toml 317, 323-324). The run has 18.1% of the paths with a collapse (report line 55).
- Recovery multiple: 2.0 times the revenue of the year before the event (`recovery_revenue_multiple`, no unit; toml 318). The event ends the cash flows of its year and later.
- Mode: `sampled` (toml 319).
The engine has no regime state. The failure table of the engine is used as a collapse of demand: the customers cut their capital spend for good. It is not a bankruptcy (toml 320-323).

## Where used
| Model | File | Parameter name |
|---|---|---|
| NVDA path model (a model document does not exist yet) | `analysis/NVDA/driver_model_paths.toml` lines 316-319 | `failure.hazard`, `failure.recovery_revenue_multiple`, `failure.mode` |

## Alternatives considered
P50 in $ a share, seed 7, 100,000 paths. Base: $103.78 (P10 54.92). Source: `COMMANDS.md` lines 44-45, 160-161.
| Alternative | P50 | P10 | Command line |
|---|---|---|---|
| Hazard 0% | 112.50 | a bound of the range | 160 |
| Hazard 4% | 93.91 | a bound of the range | 161 |
| Multiple 0.5 / 1.0 / 1.5 | 101.53 / 102.12 / 102.80 | 41.46 / 46.83 / 51.29 | 44 |
| Multiple 4.0 | 109.98 | 64.31 | 44 |
Each other row is a bound of the range (author answer, 2026-11-21). The first run had no collapse table: P50 145.95 (report line 256).

## Why this choice
JUDGMENT, stochastic (toml 320; report line 87). Reasons of the toml (320-334):
1. No NVDA precedent. The two busts (FY20, FY23) were mild (-7%, +0.2% revenue).
2. One precedent in the cache: MU, revenue $30.8B to $15.5B (FY22 to FY23, -49.5%), operating margin -37%. The 18% is a JUDGMENT with this one case.
3. The multiple 2.0 is a hand calculation (no engine run): 1.8, rounded to 2.0. It assumes revenue 30% lower, margin 30%, return 35%, cost of capital 12.5%.
4. A multiple of 2.0 assumes a bust milder than MU. The lower tail is set by the multiple: "no source" (toml 331; report W4).
The analyst added the table in version 1, after a check of the growth process alone (report lines 256-258).

## Effect on the result
- Move of the P50 for one std of the hazard: +5.09 / -5.04 (report line 138).
- Lower tail: a collapse in year 1 gives about $31 a share ($30.86; `COMMANDS.md` lines 53-54). No path is below $30; 2.8% are below $40 (line 43).
- Bias: the engine cannot impair the bridge on a failed path (see A-006). The guarantees are not in the model.
- The mean-path table of the output counts failed paths as zero revenue (report W4).

## Who decided and when
The Head of Fundamental Research, in the NVDA review rounds of 2026-11-20 and 2026-11-21. D14 (hazard 2%, multiple 2.0) is judgment with no source and no separate dated decision; it comes from version 1, 2026-11-20 (author answer, 2026-11-21).
Under review: step 2 of the valuation review, not yet accepted.

## When to examine it again
- After the skeptic and the reviewer (steps 2 and 3 of the review).
- When a panel measure exists of the share of company-years with revenue -30% or worse after growth of 50% or more (report line 344).
- When an engine feature that impairs the bridge on a failed path exists.
- The 10-Q of Q3 FY27 (the investigation and the guarantees; report W16).
