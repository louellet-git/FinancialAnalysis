# s14_spy_qqq_rotation — SPY/QQQ relative-strength rotation (dual momentum), call debit spreads

## Hypothesis
(1) Relative momentum: the QQQ/SPY ratio trends (growth vs broad-market leadership regimes persist for months), so
the recent leader keeps outperforming for weeks. (2) Absolute (time-series) momentum: only be long when the chosen
leader is above its 200-day SMA, which sidesteps most deep bear phases. Express the view with narrow bullish call
debit spreads on the leader.

## Evidence / rationale
Jegadeesh-Titman (1993) cross-sectional momentum; Moskowitz-Ooi-Pedersen (2012) time-series momentum; Antonacci
"dual momentum"; Faber (2007) 10-month SMA timing. Caveat: the evidence is for asset *returns*, not for a
$1-wide option spread, and SPY/QQQ are ~90% correlated so relative momentum has little independent alpha; most of
the P&L below is long-equity-beta with a trend filter.

## Rules
- Evaluated each Friday close (entry fills Monday open): ratio = QQQ/SPY close.
  leader = QQQ if ratio > SMA50(ratio) and 63d ratio return > 0; SPY if both negative; otherwise keep prior leader.
- Risk-on = leader close > SMA200(leader) and leader's ATM IV < 35%.
- Structure: $1-wide call debit spread on the leader, long strike at ~0.50 delta, 20 sessions to expiry. If
  debit + 4 commissions exceeds 95% of the 3% risk budget, the strikes are walked up $1 at a time (cheaper, OTM).
- One new spread per week, max 3 open (engine cap). No pending entries stacked.
- Exit: hold to expiry (settles at intrinsic; max loss = debit, inside the $60 cap). No stop/TP/flip exit.
- Sizing: engine sizes units to the 3% risk budget (1 unit at $2k; scales up as equity compounds).

## Parameters
RS_SMA 50, LOOKBACK 63, TREND_SMA 200, MAX_IV 0.35, DTE 20, LONG_DELTA 0.50, WIDTH 1.0, BUDGET_FRAC 0.95,
TAKE_PROFIT None, STOP_LOSS None, EXIT_DTE None, MAX_POSITIONS 3, CLOSE_ON_FLIP False, MODE "leader".

## Results (2005-01-03 -> 2026-10-09, daily, synthetic pricing)
```
s14_spy_qqq_rotation: 2005-01-03→2026-10-09  final $9,957
  ALL  ret +397.8%  CAGR +7.7%  maxDD -29.0%  Sharpe 0.60  trades 540  win 63%  PF 1.38
  OOS (from 2020-03-26)  ret +63.8%  maxDD -29.0%  Sharpe 0.61  trades 162  win 70%  PF 1.33
  rejected entries 1  PDT-blocked exits 0
  rejections: {'too_risky_for_account': 1}
```
540 trades, 1 contract in most of the first decade (compounding raises units to 3-4 later, so late-sample trades
dominate dollar P&L). Per year: losses in 2006 (-$473), 2008 (-$316, 8 trades all lose; filter was late), 2015, 2022
(-$1.35k, 7 trades all lose at ~3 units), and 2026 YTD (-$957). By symbol: QQQ +$4.7k (380 trades), SPY +$3.2k (160).

## Variants tried (6, judged on in-sample / ALL behaviour; OOS not used to choose)
1. Draft as inherited (TP 0.8x debit, exit at 2 DTE, close on signal flip): ret -90.6%, win 26%. Friction (~$9
   round trip on a ~$50 debit) plus closing whipsaws wiped the account.
2. Hold to expiry, no flip exit (CHOSEN): +397.8%, maxDD -29%.
3. As 2 but LONG_DELTA 0.65: +136%, Sharpe 0.41 (less leverage to the trend, same friction).
4. As 2 but MODE "pair" (also buy put spread on the laggard): -48.9%, maxDD -61% (shorting the laggard fights drift).
5. As 2 but DTE 10: -93.7%, win 9% — see fragility below.
6. As 2 but DTE 30: +202.8%, Sharpe 0.48 (positive but weaker).
(Also tried TREND_SMA=1 to disable the absolute filter: degenerate, produced no trades — not a valid ablation.)

## Risks / failure modes / model-artifact concerns
- **Edge is mostly equity beta, not rotation alpha.** A $1-wide ATM spread on $100-$600 underlyings is near a
  binary bet on "up over the next 4 weeks"; the 63% win rate is the equity drift plus the trend filter. I did NOT
  run a clean SPY-only/no-rotation ablation, so the incremental value of the rotation signal is unproven.
- **Budget fragility / path dependence.** At $2,000 equity the 0.50-delta spread (~$50 + $2.6 commission) barely
  fits the ~$57 budget. After a drawdown the strike-walking pushes entries OTM, win rate collapses, and the account
  can death-spiral (variant 5 did exactly that; variant 2 survives partly by early luck: 2006 and 2008 left it at
  ~$1.6k). Treat the result as one path.
- Any intraperiod exit is expensive (round-trip commission + spread ≈ 18% of debit); hence hold-to-expiry.
- Model artifacts: BS with a call-skew model and fixed $0.01 half-spreads; real $1-wide spreads on 20DTE SPY/QQQ
  quote wider, and pre-2022 expiries were weekly/monthly, not any-day. Spread debit near 0.5 means pricing
  differences are small, but fills at the ask on two legs in real markets will cost more.
- 2022-style bear: the 200d filter stayed on long enough to lose 7 straight trades at 3 units (-20% equity).
  MaxDD -29% is at the edge of the 25-30% target.

## Paper-trading notes
Watch: realised fill vs model mid on the two legs; whether Friday signal flips churn the leader (hysteresis);
equity drifting below ~$1.9k (spreads stop fitting the $60 cap); consecutive-loss clusters when the leader
rolls over below its SMA200; compare against a plain SPY-only version to see whether the rotation adds anything.
