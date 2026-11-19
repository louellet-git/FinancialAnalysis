# s09_rsi2_pullback — Connors RSI(2) pullback in an uptrend, via short-dated call debit spreads

**Verdict: NEGATIVE on a $2,000 / $60-risk account.** The underlying signal has a real, small edge; it does not survive $1-wide spread costs, commissions and the 3% risk cap.

## Hypothesis
Short-term mean reversion in broad equity indices: after a sharp 1-3 day drop inside a long-term uptrend (close > 200-SMA), the next few sessions tend to bounce (Connors & Alvarez, *Short Term Trading Strategies That Work*, 2008; practitioner RSI(2)<10 studies; liquidity-provision / short-term reversal literature). Option expression: a bullish defined-risk spread of 10 sessions' tenor.

## Evidence (own check, underlying only, SPY/QQQ 2004-2026, 5-day max, no costs)
Signal: close>SMA200 and Wilder RSI(2)<10; enter next open, exit first close>SMA5 (min 1 held day, max 5 bars).
- SPY: 321 signals, mean +0.28% per trade, median +0.53%, 71% win, avg hold 2.4 days (unconditional 3-day mean +0.09%).
- QQQ: 365 signals, mean +0.40%, median +0.69%, 70% win (unconditional +0.14%).
- Edge is positive in most years but small and fat-left-tailed (2008, 2011, 2018, 2022 negative). A $1-wide spread on a ~$500 index turns +0.28% (~$1.4/share) into at most a few cents of net value change.

## Rules
- Universe SPY, QQQ (independent, one position per symbol). Signal at the close, fill next open.
- Entry: close > SMA(200) and RSI(2) < `RSI_ENTRY`. Structure: buy ~0.50-delta call, sell call $1 higher, expiry 10 sessions out (from the fill day).
- Exit (first of): close > SMA(5) or RSI(2) > 65 (never on the fill day, avoids PDT day-trades); `max_hold_bars`=7; `exit_dte`=1.
- Sizing: engine's (1 unit always: ~$45-52 debit vs. $60 cap). Entries whose max loss + commissions > $60 are rejected.

## Parameters
TREND_LEN 200, RSI_LEN 2, RSI_ENTRY 5, RSI_EXIT 65, EXIT_MA_LEN 5, STRUCTURE call_debit, DTE 10, LONG_DELTA 0.50, WIDTH 1, MAX_HOLD 7, EXIT_DTE 1, no TP/SL.

## Results (final variant, `python -m scripts.trade_backtest s09_rsi2_pullback`)
```
s09_rsi2_pullback: 2004-01-02→2026-10-09  final $1,781
  ALL  ret -11.0%  CAGR -0.5%  maxDD -15.4%  Sharpe -0.21  trades 81  win 57%  PF 0.67
  OOS (from 2019-12-06)  ret +0.8%  maxDD -3.5%  Sharpe 0.08  trades 10  win 70%  PF 1.29
  rejected entries 181  PDT-blocked exits 0
  rejections: {'too_risky_for_account': 181}
```
Exit breakdown: 72 recovery exits averaged +$1.5 (commission-dominated); 9 time-stops averaged -$36. OOS has only 10 trades: statistically meaningless; its slight positive is not evidence of an edge.

## Variants tried (6-7 runs; ALL/in-sample judged, OOS not used to choose)
| Variant | ALL ret | trades | PF |
|---|---|---|---|
| Draft: put credit $1, 0.30d short, DTE7, RSI<10 | +1.1% | 1 (684 rejected: credit ~0.30 -> max loss 0.70 > cap) | n/a |
| put credit 0.45d | -10.3% | 14 | 0.35 |
| put credit 0.55d | -29.9% | 36 | 0.23 |
| call debit 0.50d DTE7 hold5 (SPY+QQQ) | -33.4% | 96 | 0.47 |
| call debit 0.40d | -44.3% | 129 | 0.44 |
| call debit 0.50d, SPY only | -12.1% | 85 | 0.72 |
| call debit 0.50d DTE10 hold7 RSI<10 | -28.0% | 94 | 0.44 |
| **call debit 0.50d DTE10 hold7 RSI<5 (final)** | -11.0% | 81 | 0.67 |
No variant is profitable over the full sample. "Final" was chosen as the least-bad IS result with both symbols and more trades than the 1-14 trade put-credit variants. Note put credit spreads cannot be sized at $1 width unless the short is near ATM (credit >= ~0.40).

## Why it fails / risks
- Cost structure: round-trip commission $2.60 plus ATM bid/ask on a ~$50 position; recoveries yield only ~$5-7 gross while time-stop losers lose ~$35 (win/loss asymmetry ~1:5).
- Oversold days are high-IV days; debit spreads buy elevated vol, and IV crush on the bounce offsets part of the delta gain. The credit-spread side (sells that vol) is blocked by the risk cap.
- 181 of ~260 signals were rejected as `too_risky_for_account` (high-vol regimes inflate premiums), so the strategy systematically skips the best-bounce (high-VIX) setups and trades the calm ones where the edge is weakest.
- Model-artifact concerns: synthetic BS pricing with VIX-based IV and skew; real fills in a sharp selloff would be wider. Pre-2022 daily expiries assumed (10-session expiry is usually available as a weekly anyway).

## Paper-trading notes
Not recommended for live paper deployment as-is. If revisited: test on a larger account (multi-unit sizing so commissions amortise), share-like exposure (long ITM call/LEAPS-style is excluded by the 10-DTE brief) or the equity itself; watch fill-day IV vs. exit-day IV and the fraction of signals rejected by the risk cap.
