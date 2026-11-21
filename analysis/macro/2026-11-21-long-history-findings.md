# Findings: long-history test of the market-level DCF (2026-11-21)

Ticket `T-20261121-dcf-engine-backtest` (Fundamental Research) and the design
`analysis/macro/2026-11-21-long-history-design.md` (frozen at 2ffd392; records R1, R2 before the runs,
R3 after the first run). This file gives the result. It is not a model input and not a trade signal.

## Main finding

Both premiums pass the frozen primary rule: the dividend baseline P1 and the engine premium predict the
later 10-year real excess return of the US index, 1893-01 to 2016-09, in sample and out of sample. **The
strength is limited:** each p is one test on about 12 independent 10-year windows; the effect sits in
the blocks after 1953 (B3, B4), and the first block (1893-1922) has a negative slope.

## Question

Does the premium that the price implies (the engine premium, and the baseline P1) forecast the later
10-year real excess return, by the rule fixed before the run (one-sided bootstrap p < 0.05 AND
out-of-sample R2 > 0)?

## Data and commands

| Item | Value |
|---|---|
| Run of record | `claude/dcf-market-level` 3f5a6e3 (the complete run) |
| Superseded run | 1c4cb49 (first run; P2 and the January phase not built) |
| Data | store `louellet-git/fa-data` 8952a5a. SHA-256 prefixes, checked by the Head of Economics in a separate clone: shiller_monthly.csv 0837dc59a619cbae, CPIAUCNS.csv 0e1fc35d62c4b766, TB3MS.csv 33fae406051a658c, GS10.csv 018e609eb98c52ec, M13002US35620M156NNBR.csv b027324556f781b0 |
| Bootstrap | circular blocks of 120 months, K = 10,000, seed 20261121, PCG64 |
| Sample | 1,484 pairs for H = 120 (1893-01 to 2016-09, less 2015-10 by R2); 1,544 for H = 60 |
| Score texts | `analysis/priors/market_score_complete_2026-11-21.txt` (record) and `analysis/priors/market_score_2026-11-21.txt` (superseded), on branch `claude/dcf-market-level`; the command is in `analysis/priors/market_score_2026-11-21.command.txt` |

## Evidence

| Test | Run of record 3f5a6e3 | Superseded 1c4cb49 | Verdict |
|---|---|---|---|
| Primary, P1 | b 0.6374, p 0.0309, OOS R2 0.0865 | b 0.637, p 0.0319, OOS R2 0.087 | PASS (both runs) |
| Primary, engine (L 2%, N 10) | b 0.6945, p 0.0114, OOS R2 0.1194 | b 0.695, p 0.0115, OOS R2 0.119 | PASS (both runs) |
| Engine-failure rule (engine OOS R2 below that of P1) | 0.1194 > 0.0865 | 0.119 > 0.087 | not failed |
| P1 family, Holm m = 6 | rejects S1 (adj p 0.038), B3 (0.033), B4 (0.033); not S2 / P2 (0.339), B1, B2 | P2 not built, so the family was not complete; B4 p 0.0070 (record run: 0.0055) | 3 of 6 reject |
| Engine family, Holm m = 5 | each adjusted p 0.0555 | — | no rejection; descriptive only |
| Blocks (slope) | B1 (1893-1922) negative for both (P1 -0.21, engine -0.10); B2 about 0.8, p 0.11 to 0.14; B3 and B4 strong | — | the effect is after 1953 |
| January phase (1893-01, every 120 months; descriptive) | P1 0.771, P2 0.342, engine 0.820 | not built | agrees in sign |
| Regime periods | no period has 3 independent windows | — | descriptive only (by design) |

No verdict changes between the two runs.

## Audit

Quality and Risk (`auditor`, 2026-11-21): the rerun gives the score text byte for byte; the input hashes
match; the counts are right; a hand recalculation of DY, g, r10, P1, X120, X60 and erp_implied in seven
months (1893-01, 1913-01, 1936-04, 1953-04, 1980-01, 2008-12, 2016-09) matches to 6 digits, and the
whole CSV within 5e-7; the hand bootstrap p of P1 is 0.0310 (one tie draw from 0.0309). The bootstrap,
OOS, rejection rule, families and periods follow the design.

## What the result does not show

- It is one country (the US, a market that did well: survivorship) and one index (the Shiller
  composite, a reconstruction before 1926).
- About 12 independent windows: a p near 0.01 to 0.03 is weak evidence alone.
- The effect is not stable in time: B1 is negative, B2 is weak. The "works at any time" claim is not
  supported for 1893-1952.
- The engine variants (L 1% and 3%, N 5 and 20, fixed-L Gordon) do not pass after Holm. Read them as
  description: not as support and not as a failure.
- The GS10 check (D5) is empty for the sample; it is not a robustness test.
- R3 was written after the first run was seen (D1-D5 in the design). No rule was changed on purpose,
  and the verdicts are the same in both runs.

## What stays open

| Point | Owner |
|---|---|
| A second country (JST data) as an out-of-sample test: pre-register it before the download | Head of Economics; data: Head of Technology |
| `docs/models/market-level-dcf.md` line 8 still says "the command does not score" | Head of Fundamental Research |
| Any use of the premium for a decision: the person decides; this file sets no input | the person |
