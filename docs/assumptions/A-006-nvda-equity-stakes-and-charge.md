# A-006: NVDA acquisition charge and equity-purchase charge, with the share r (D11)

Updated: 2026-11-21. Draft: from commit 38fbbdb (values as in 5ffe7a0) of `claude/nvda-valuation`.
Source: commit 5ffe7a0, branch `claude/nvda-valuation`. Line numbers below are of that commit.

## Choice
One number r = 0.5 (no unit). A purchase of an equity security is partly an investment that keeps its value (the share r) and partly a subsidy to buyers that buys credited revenue (the share 1 - r). The same r applies to:
1. The stakes in the bridge. Marketable equity securities ($44.219B) are at FULL value: r does not apply. Non-marketable ($65.528B) are at r x carrying value ($32.764B). `net_debt` = -$98.124B (toml 42-53).
2. The future purchases, charged at (1 - r) x 19.6% of revenue / 0.79 (tax) = 12.4%. Plus 1.5% for purchases of businesses (A-002, rule F16). `process.charge.start` mean 0.13905, std 0.02, cut 0.015 to 0.40 (toml 197).
3. The fade: the charge falls to the target (mean 0.015, std 0.007, cut 0 to 0.05) with the half-life of D3 (mean 1.0 year, std 0.4; correlation 0.9; toml 198-199, 370-374).

## Where used
| Model | File | Parameter name |
|---|---|---|
| NVDA path model (a model document does not exist yet) | `analysis/NVDA/driver_model_paths.toml` | `net_debt` (53), `[process.charge]` `start`, `target`, `half_life` (197-199) |

## Alternatives considered
P50 in $ a share, seed 7, 100,000 paths. Base: $103.78. Sources: report lines 46-50, 105-113; `COMMANDS.md` lines 82-91, 122-136.
| Alternative | P50 | Command line | Why not chosen |
|---|---|---|---|
| r = 0 (non-marketable out, full charge) | 98.42 | 85 | A bound of the range |
| r = 0.25 / 0.75 | 101.10 / 106.49 | 85 | A bound of the range (author answer, 2026-11-21) |
| r = 1 (all stakes in, no equity charge) | 109.08 | 85 | The "fair price" frame; a bound of the range |
| Fade ou, half-life 2.0 years | 101.06 | 127 | The charge pays for credited growth: same half-life as D3 |
| Ramp 3 / 4 / 5 / 10 years | 104.77 / 103.73 / 102.71 / 98.32 | 122-125 | Version 3 had a ramp of 10 years; the fade did not follow the growth (toml 15-16) |
| Start at 0.39 / 0.55 / 0.24 of the past (panel) | 106.23 / 105.61 / 106.83 | 129-131 | Cross-check only, not the rule (toml 215-218) |
| Q3 rate / net of sales / no tax gross-up | 105.02 / 104.41 / 104.62 | 133-135 | Q3 alone is not used (toml 211); other rows: a bound of the range (author answer) |
| Purchases of businesses at 2.0% (the measured value) | 102.88 | 136 | 1.5% "kept as decided" (toml 224) |

## Why this choice
JUDGMENT, with no data for r (report lines 99, 306). The Head of Fundamental Research decided: there are no terms and no data for r, and 0.5 says that we do not know. The range of r carries the uncertainty. The model is paired (IBM F16): one frame, one r, for the stakes and the purchases (toml 42-49). The marketable stakes have observable marks, $25.5B of net gains in nine months and no impairment (L11). The tax gross-up: the purchases are not tax deductible; the skeptic accepted it (toml 211-213). The fade follows A-002. The mean of years 1 to 3 is 58% of the start. The panel factor is 0.55 (study 2.1): a cross-check, not the rule.

## Effect on the result
- The P50 moves by $10.7 over r = 0 to 1 (report line 52). P10 / P90 at r = 0: 50.47 / 193.89; at r = 1: 59.28 / 207.87.
- Limit: the engine cannot impair the bridge on a failed path. A collapse path keeps the stakes ($3.18 a share at r = 0.5) and has no guarantee claim (up to $4.49 a share, L12). The P10 is up to about $3.2 too high (toml 50-52; report line 57).
- If the marketable stakes lost all value, the P50 is up to $1.83 too high (report W6).
- The terms of the stakes are in the 10-Q notes: not read.

## Who decided and when
The Head of Fundamental Research, 2026-11-21, rounds 2 and 4 (marketable stakes at full value: round 4). The 1.5% for businesses: analyst judgment, version 2, 2026-11-21, from the nine months of FY2027; accepted by the Head of Fundamental Research (toml 224; author answer).
Under review: step 2 of the valuation review, not yet accepted.

## When to examine it again
- The 10-Q of Q3 FY27 (terms of the equity securities, guarantees, convertible debt; report lines 325, W14), the close of 2026-11-20.
- When an engine feature that impairs the bridge on a failed path exists (report line 325, item 2).
- When the review of A-002 is done, and after step 2 of the valuation review.
