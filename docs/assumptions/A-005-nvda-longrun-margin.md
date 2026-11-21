# A-005: NVDA long-run operating margin (D7)

Updated: 2026-11-21. Draft: from commit 5ffe7a0 of `claude/nvda-valuation`.
Source: commit 5ffe7a0, branch `claude/nvda-valuation`. Line numbers below are of that commit.

## Choice
The operating margin before SBC goes to a target with mean 48.5% (std 8%, cut at 25% and 65%). The target is a draw.
It is a GAAP operating margin of 45% plus an SBC ratio of 3.5% (toml line 155). The start is 66% (D6). The half-life of the fade is 3 years (D8, std 1; its own judgment).

## Where used
| Model | File | Parameter name |
|---|---|---|
| NVDA path model (a model document does not exist yet) | `analysis/NVDA/driver_model_paths.toml` line 141 | `process.margin.target` (D7) |

## Alternatives considered
P50 in $ a share, seed 7, 100,000 paths. Base: $103.78. Source: `COMMANDS.md` line 148.
| Alternative | P50 | Why not chosen |
|---|---|---|
| 35% before SBC (GAAP 31%, the mean of FY17 to FY22) | 85.31 | A bound of the range (author answer, 2026-11-21) |
| 60% before SBC (GAAP 56%) | 119.06 | Report: a margin above 60% for ten years is not in the NVDA history (toml 158-160) |
The GAAP values (31%, 45%, 56%) are in toml lines 164-165.

## Why this choice
JUDGMENT, stochastic (report line 80). The report gives these reasons (toml 155-165).
1. 45% GAAP is the middle between the mean GAAP operating margin of FY17 to FY22 (31%) and of FY24 to FY26 (59%) (ledger L3: 30.7% and 59.0%).
2. The reason for a fade at all is a JUDGMENT with no source in the ledger: few large buyers, and a high margin attracts other sellers and own designs of the buyers.
3. Comparators in the cache (company-years above $100B revenue): MSFT 41.6% to 46.8%, META 34.7% to 42.2%. Only MU (74.6%, one year) and NVDA (60.4%, 62.4%) are above 47%. So 45% is at the level of the best sustained large company.
4. The model has $509B of operating profit in year 10 at the means (revenue $1,079.5B x (50.7% - 3.5%); the $472B of toml line 164 is stale, version 3; author answer, 2026-11-21). The largest in the cache is $158.1B (AAPL FY26) (toml 163-164).
The report says D3 and D7 sit above the base rate of scale (report lines 210, 306).

## Effect on the result
- P50 at the base: $103.78. The table above gives -$18.47 and +$15.28.
- Move of the P50 for one std of the mean: -11.21 / +11.10 (report line 137).
- Price implied: no solution up to 78% (report line 227; `COMMANDS.md` line 248). A margin alone cannot reach the close.
- Command: `fa dcf drivers NVDA --model analysis/NVDA/driver_model_paths.toml --sims 100000 --no-attribution --sensitivity process.margin.target=0.35,0.485,0.60`

## Who decided and when
The Head of Fundamental Research, in the NVDA review rounds of 2026-11-20 and 2026-11-21.
Under review: step 2 of the valuation review, not yet accepted. The value can move until the skeptic accepts.

## When to examine it again
- After the skeptic and the reviewer (steps 2 and 3 of the review).
- At the next 10-K (2027-02, ledger L3) and the 10-Q of Q3 FY27.
- When the Q4 FY27 result shows the real operating margin against the guided 63.0% GAAP (toml 150-152).
- When a source for the fade of the margin exists (the ledger has none).
