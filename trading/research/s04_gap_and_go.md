# s04_gap_and_go — gap continuation debit spreads

## Hypothesis
A large opening gap (relative to the VIX-implied daily move) that survives the first hour without
filling signals informed order flow / momentum; the day (or next session) should continue in the gap
direction. This is the OPPOSITE bet to the gap-fade bots (gaps fill / overreaction mean-reverts).

## Evidence / rationale
Practitioner lore ("gap and go", opening-range momentum; Gao-Han-Li-Zhou intraday momentum: first
half-hour return predicts last half-hour) versus the academic/empirical norm that most index gaps
partially fill. Prior: weak, regime-dependent edge. Maps to a $1-wide debit spread so max loss fits the
~$60 cap.

## Rules
- Symbol SPY (QQQ tested as variant), interval 1h. Decide at the close of the first 1h bar (10:30), fill
  at the next bar open.
- gap = today's open / prior close - 1; z = |gap| / (VIX/sqrt(252)); require 0.6 <= z <= 3.0.
- "Holds / no fill": first-hour low (gap up) stays above prior close (high below for gap down), first-hour
  close retains >= 60% of the gap and closes in the gap direction.
- Structure: 1DTE, ~0.45-delta long leg, $1-wide vertical debit spread (call for up gaps, put for down).
  One trade per day; sizing by engine (3% risk).
- Exits: take profit +80% of debit, stop -50%, back into prior-day range (spot returns to prior-day
  high for up gaps / prior low for down gaps -> ctx.close), flatten 30 min before close on expiry day.

## Parameters
MIN_GAP_Z 0.6, MAX_GAP_Z 3.0, HOLD_FRAC 0.6, CLOSE_CONFIRM True, DTE 1, WIDTH 1.0, LONG_DELTA 0.45,
TAKE_PROFIT 0.8, STOP_LOSS 0.5, EXIT_RANGE True, EXIT_MINUTES 30.

## Results (final = pre-specified baseline, SPY, 2023-11-10 -> 2026-10-09)
```
s04_gap_and_go: 2023-11-10→2026-10-09  final $1,699
  ALL  ret -15.0%  CAGR -5.4%  maxDD -17.4%  Sharpe -0.69  trades 58  win 34%  PF 0.70
  OOS (from 2025-11-24)  ret -8.3%  maxDD -8.8%  Sharpe -1.45  trades 12  win 25%  PF 0.41
  rejected entries 1  PDT-blocked exits 0
  rejections: {'too_risky_for_account': 1}
```
Sample is tiny (58 trades all, 12 OOS) — any conclusion is low-confidence, but the sign is negative.

Underlying diagnostic (SPY/QQQ 1h, ~2y; day-close return vs first-hour close, in gap direction, for
gaps that held): SPY z>=0.6: n=105 mean -10 bp, hit 53%; z>=1.0: n=29 mean -18 bp. QQQ z>=0.6: n=138 mean
+0.2 bp, hit 59%; z>=1.0: n=41 mean -8.5 bp. Hit rate is mildly >50% but the mean is ~0 or negative:
no continuation edge after the first hour (consistent with, if anything, slight fade / reversion).

## Variants tried (ALL-sample ret / trades / PF; OOS ret / trades)
| variant | ALL | OOS |
|---|---|---|
| base (1DTE, SPY) | -15.0% / 58 / 0.70 | -8.3% / 12 |
| 0DTE | -24.6% / 51 / 0.44 | -7.3% / 8 (9 too_risky rejections) |
| QQQ | -14.1% / 69 / 0.75 | -1.1% / 13 (12 rejections) |
| MIN_GAP_Z 1.0 | -6.3% / 18 / 0.65 | -3.2% / 2 |
| TP 150%, SL 100%, no range exit | -9.8% / 54 / 0.84 | +0.4% / 12 (PF 1.03) |
| QQQ, z>=0.4, TP150/SL100, no range | -30.1% / 68 / 0.64 | -5.6% / 4 (52 rejections) |
No variant is profitable over the full sample; I kept the pre-specified baseline rather than pick the
OOS-best (+0.4%, 12 trades = noise).

## Risks / model artifacts
- Debit spreads pay commissions (4 x $0.65 round trip on 1 unit) plus bid/ask on both legs: on $1-wide
  spreads costing ~$0.40-0.50 that is a large hurdle.
- 0DTE QQQ spreads are often too expensive for the $60 cap (rejections).
- Gap-day VIX/VIX1D inputs make option prices on gap days model-dependent (VIX1D only since 2023).
- Sample is only ~2y of 1h data; tiny n.

## Paper-trading notes
Not recommended for paper trading as is. If studied further, log gap z, first-hour hold quality, and
actual spread cost; test whether the AFTER-FIRST-HOUR drift conditional on news-driven (earnings/macro)
gaps differs from generic gaps, and compare against the gap-fade bots on the same days.
