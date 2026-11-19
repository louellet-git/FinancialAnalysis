# s12_turn_of_month — Turn-of-the-month seasonality via SPY spreads

## Hypothesis
Equity returns are concentrated around the turn of the month (last ~1-4 sessions through first ~3 of the next). A short-dated bullish defined-risk spread held only across that window should capture the drift with bounded risk.

## Evidence / rationale
Ariel (1987), Lakonishok & Smidt (1988), McConnell & Xu (2008: TOM accounts for essentially all of the US equity premium 1926-2005; commonly attributed to pay/pension inflows, window dressing). Own check on repo data (SPY/QQQ 1d, 2005+, window = last PRE + first POST sessions):

| SPY window | TOM mean bp/day | other bp/day | Welch t |
|---|---|---|---|
| last 1 + first 3 | 6.0 | 3.7 | 0.60 |
| last 3 + first 3 | 7.5 | 2.7 | 1.40 |
| last 4 + first 3 | 9.3 | 1.5 | 2.34 |

QQQ is similar (t 0.5-1.9). The excess is ~5 bp/day and roughly halves in the second half of the sample (SPY 7.0bp -> 2.6bp; QQQ 5.7 -> 2.8): real-ish but small and decaying post-publication. ~6 days x 5bp = ~30bp/month of excess underlying return, versus option spread + $0.65/leg commission on a $1-wide spread (~$2.6 round trip on a ~$50 position = ~5%).

## Rules
- Entry: at the close of the session that leaves PRE (=3) sessions in the month, queue a bullish $1-wide SPY spread; fills next open. One trade per month. Month end from a business-day calendar (holidays ignored, +/-1 session).
- Structure: `KIND=call_debit` (long 0.50-delta call, short +$1) by default; `put_credit` optional (short put at DELTA, long $1 below).
- Expiry: 9 sessions out at fill.
- Exit: `ctx.close` at the close of the POST (=3)-th session of the new month; stop loss at 60% of premium; `exit_dte=1` backstop.
- Sizing: engine's (max loss <= 3% equity, normally 1 unit).

## Parameters
PRE=3, POST=3, KIND="call_debit", DELTA=0.50, WIDTH=1.0, DTE=9, STOP=0.6, start 2005-01-01, symbol SPY.

## Results (final config)
```
s12_turn_of_month: 2005-01-03→2026-10-09  final $1,851
  ALL  ret -7.5%  CAGR -0.4%  maxDD -20.0%  Sharpe -0.09  trades 109  win 57%  PF 0.89
  OOS (from 2020-03-26)  ret +4.9%  maxDD -3.4%  Sharpe 0.31  trades 15  win 73%  PF 1.93
  rejected entries 152  PDT-blocked exits 0
  rejections: {'too_risky_for_account': 152}
```
Caveats: 109 trades total, only 15 in OOS; 58% of monthly signals are rejected because the debit exceeds the $60 risk budget, so the traded subset is biased toward cheap-vol months. OOS +4.9% on 15 trades is noise and was not used for choosing. Full sample is negative.

## Variants tried (full-sample ALL ret / PF / OOS PF)
1. Default call debit (final): -7.5% / 0.89 / 1.93 (15 trades)
2. put_credit 0.50d: -8.6% / 0.72 / 0.61 (41 trades, 220 rejected)
3. PRE=4 call debit: -16.1% / 0.75 / 0.51
4. QQQ call debit: -15.0% / 0.81 / 0.55
5. put_credit 0.35d (SPY and QQQ): 0 trades (credit too small, max loss > $60, all rejected)
6. call debit 0.35d no stop: -37% / 0.67; 0.30d: -51% / 0.53; put_credit 0.60d: -19.5% / 0.79
(A few more runs than 6, as feasibility probes for the risk cap; none improved.) The default was kept as least-negative and simplest, not because it works.

## Risks / model artifacts
- Edge ~30bp/month of underlying; commissions and bid/ask drag consume it. Nothing here depends on model mispricing, so artifact risk is low; real fills would likely be worse.
- Pre-2022 daily expiries assumed (approximate). Holiday calendar ignored.
- Pre-holiday effect not tested (TOM already failed the cost hurdle).

## Verdict / paper-trading notes
Negative result: the TOM effect on SPY/QQQ is marginal post-2005 and decaying, and does not survive option-structure costs under a $60 risk cap. Not recommended live. On paper, log the underlying's window return vs the ~30bp hurdle and how often entries are rejected for risk; at most use TOM as a mild bullish tilt filter for other bots.
