# s01_orb_0dte - Opening-range breakout on 0DTE debit spreads

## Hypothesis
A decisive break of the first 30 minutes' range, held for a confirmed close beyond it, marks the day's directional drift; a narrow 0DTE debit spread in that direction captures the continuation cheaply and with defined risk.

## Evidence / rationale
Crabel (1990) "Day Trading with Short Term Price Patterns and Opening Range Breakout"; Zarattini, Barbon & Aziz (2024) "Can Day Trading Really Be Profitable? Evidence of Sustainable Long-term Profits from Opening Range Breakout (ORB) Strategies on QQQ" (5-min ORB, mostly on the underlying and with leverage); intraday momentum (Gao, Han, Li, Zhou 2018: first half-hour return predicts last half-hour). Caveat: published ORB edges are measured on the underlying with small costs. A 0DTE debit spread adds theta, spread cost and commission ($0.65 x 2 legs x 2 sides = $2.60 per $1-wide round trip against a ~$50 debit, about 5%).

## Rules
- Underlying SPY, interval 5m. Opening range = high/low of first 6 bars (09:30-10:00).
- Entry: after 10:00 and with >= 120 min to close, first bar close above range high + 0.25 x range width (call debit spread) or below range low - 0.25 x width (put debit spread). Skip if range width > 2% of spot. One trade/day; max 3 entries per rolling 7 calendar days (self-limit to PDT budget).
- Structure: ATM (rounded) long strike, short strike $1 further OTM, same-day expiry. Engine sizes to the 3% ($60) risk cap: 1 unit.
- Exits: take profit +100% of debit, stop -50% of debit, close if spot closes back inside the opening range (`range_reentry`), time exit 20 min before close.

## Parameters
OR_BARS=6, BUFFER=0.25, MAX_RANGE_PCT=0.02, WIDTH=1.0, TAKE_PROFIT=1.0, STOP_LOSS=0.5, REENTRY_STOP=True, EXIT_MINUTES_BEFORE_CLOSE=20, ENTRY_CUTOFF_MIN=120, MAX_DAY_TRADES_5D=3.

## Results (final variant, SPY 5m, 2026-07-17 -> 2026-10-09, ~60 sessions)
```
s01_orb_0dte: 2026-07-17→2026-10-09  final $1,998
  ALL  ret -0.1%  CAGR -0.5%  maxDD -5.0%  Sharpe 0.40  trades 34  win 35%  PF 0.99
  OOS (from 2026-09-16)  ret -0.2%  maxDD -4.2%  Sharpe 0.36  trades 10  win 30%  PF 0.96
  rejected entries 1  PDT-blocked exits 0
  rejections: {'too_risky_for_account': 1}
```
Verdict: NO demonstrable edge - breakeven before any slippage beyond the model's. 34 trades is far too small to distinguish PF 0.99 from anything; standard error on win rate is ~8 pts.

## Variants tried (all-sample numbers; 5m has only ~60 sessions so ALL is mostly in-sample)
| Variant | ret % | maxDD % | trades | PF |
|---|---|---|---|---|
| 5m, BUFFER 0, re-entry stop | -2.7 | -7.3 | 37 | 0.86 (OOS PF 0.33) |
| 5m, no re-entry stop | -11.4 | -15.1 | 35 | 0.63 |
| 5m, TP +50% | -9.3 | -9.6 | 36 | 0.48 |
| **5m, BUFFER 0.25 (final)** | -0.1 | -5.0 | 34 | 0.99 |
| 1h (first-hour range), SPY, re-entry stop | -37.4 | -38.8 | 127 | 0.63 |
| 1h, QQQ | -34.3 | -35.5 | 132 | 0.68 |
| 1h, no re-entry stop | -38.2 | -40.2 | 133 | 0.65 |
| 1h, buffer 0.25, no re-entry | -31.1 | -32.4 | 173 | 0.76 |
| 1h, $2-wide | -8.5 | -9.4 | 19 | 0.58 (394 rejected too_risky) |
The final variant was chosen because the confirmation buffer cut false breaks (the dominant loss: re-entry exits) and the 1h interval was clearly worse. This selection was made on essentially the whole 60-day sample, so even the near-zero result is optimistic.

The 1h interval does give ~2.9 years (2023-11 -> 2026-10, ~750 sessions) but is a poor fit: hourly bars mean entries only at 10:30+ (after a full hour of the move), exits are only evaluated once an hour so stops gap, and 0DTE theta bleeds heavily. It loses 30-40% consistently, which at this sample (127+ trades) is meaningful, though the exit granularity is part of the cause. 1h variants also had 260-390 entries rejected `too_risky_for_account` (an ATM $1 spread late in a high-vol day exceeding $60 incl. commission, or $2-wide spreads).

## Risks / model-artifact concerns
- Pricing is Black-Scholes on VIX1D (VIX before 2023) with synthetic skew: 0DTE gamma/theta is entirely a function of that input. Model has no intraday IV response to the breakout move (real IV tends to rise on breakouts, helping long premium; but real 0DTE spreads are also wider than $0.01-0.05/leg when price moves fast).
- Entries fill at next-bar open; a 5m signal is chased 5 min late.
- PDT: 3 day trades per 5 sessions caps the trade count; no PDT blocks occurred thanks to self-limiting.
- Tiny sample; regime (Jul-Oct 2026) may be range-bound or trending.

## Paper-trading notes
Log whether confirmed-break days continue vs. fail back into the range, real spread cost on $1-wide 0DTE spreads at 10:00-10:30, and fill slippage. Most losses are `range_reentry` exits: measure how often price re-enters within 15 min of the break. Consider testing the idea on underlying shares / with gap-direction filters before options. Do not trade live without the evidence this backtest lacks.
