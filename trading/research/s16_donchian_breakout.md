# s16_donchian_breakout — Turtle Donchian breakout as debit spreads

**Verdict: NEGATIVE.** The edge does not survive expression as capped debit verticals inside a $2,000 / $60-per-trade account. All variants lose money; the account is effectively dead (equity too small to afford even a $1-wide spread) before the out-of-sample window starts.

## Hypothesis
Price channel breakouts (Turtle System 1: 20-day entry / 10-day exit, 2N ATR stop) capture time-series momentum in equity indices. Expressing the trade as a 20-session debit spread in the breakout direction gives defined risk and some leverage; rolling keeps exposure while the trend persists.

## Evidence / rationale
Time-series momentum / trend following (Moskowitz-Ooi-Pedersen 2012; Hurst-Ooi-Pedersen 2017; Turtle rules, Faith) has long-run support, but mostly on diversified futures baskets, not on a single equity index in the post-2000 era, where breakout systems on SPY have had low win rates (~30-40%) and live off a few large winners. That payoff shape is exactly what a debit spread destroys: losers cost the full debit (~-70 to -100%), winners are capped at width (+100-130% at best).

## Rules
- Signals on daily closes, per symbol (SPY, QQQ), fills at next open.
- Long trend: close > prior 20-day high. Short trend: close < prior 20-day low. Trend ends when close crosses the opposite 10-day channel, or moves 2 x ATR(20) against the breakout close, or an opposite breakout fires.
- Structure: one debit vertical per symbol in the trend direction (call spread up, put spread down), 20 sessions to expiry. Long strike = rounded spot, stepping OTM (0, 0.3, 0.6, 1.0, 1.5, 2.0 %) until max loss (debit + commissions) fits 95% of the 3%-equity budget; width $2 else $1 (first affordable combination, ATM first).
- Exits: close all on trend end; roll (close + reopen) when <= 5 sessions remain or the spread is worth >= 85% of width; engine `exit_dte=1` as backstop.
- Sizing: engine-determined (almost always 1 unit).

## Parameters
ENTRY_N=20, EXIT_N=10, ATR_N=20, STOP_ATR=2.0, REGIME_SMA=0 (off), ALLOW_SHORT=True, DTE=20, ROLL_DTE=5, ROLL_CAPTURE=0.85, OTM_STEPS=(0,.003,.006,.010,.015,.02), WIDTHS=(2,1), BUDGET_FRAC=0.95.

## Results (final config, `--start` default 2005-01-01)
```
s16_donchian_breakout: 2005-01-03→2026-10-09  final $864
  ALL  ret -56.8%  CAGR -3.8%  maxDD -57.3%  Sharpe -0.66  trades 111  win 25%  PF 0.41
  OOS (from 2020-03-26)  ret +0.0%  maxDD 0.0%  Sharpe 0.00  trades 0  win 0%  PF 0.00
  rejected entries 7  PDT-blocked exits 0
  rejections: {'too_risky_for_account': 7}
```
Exit reasons: 31 trend_end (mostly full-debit losses of $20-40), 29 roll_dte; roll_capture never triggered in the final run. OOS has ZERO trades: equity decayed below the level where a $1-wide spread on ~$500+ SPY/QQQ (debit ~$0.4-0.5 + $2.6 commission round trip) fits under 3% of equity (~$57), so the strategy goes silent in about 2013 and the OOS number is not evidence of anything. In-sample behaviour is the only real evidence, and it is clearly negative.

## Variants tried (all full-sample, ~6 max; none helped)
| # | Config | Return | MaxDD | Trades | Win | PF |
|---|---|---|---|---|---|---|
| 1 | draft: 20/10, delta-0.50 strike, widths 1-5 | -24% | -27% | 52 | 31% | 0.49 (stalled after losses: unaffordable) |
| 2 | 20/10, ATM-stepping-OTM, widths 2/1 (FINAL) | -57% | -57% | 111 | 25% | 0.41 |
| 3 | 55/20, DTE 30, roll at 7 | -46% | -48% | 104 | 32% | 0.48 |
| 4 | 20/20 + 200d SMA regime filter | -54% | -56% | 128 | 31% | 0.48 |
| 5 | variant 4 settings, shorts off | -53% | -56% | 135 | 34% | 0.51 |
(Variants 4/5 ran in one process so 5 inherited 4's params; labels approximate.)
Diagnostics on final signal: zero commission at $2k: -57% (commission is NOT the cause); $50k account with widths up to $5 (cap 20 units): PF 0.68-0.72, -76% to -78% (so the loss is in the signal/structure, not just account size).
The v1 draft's `strike_for_delta` bisection made one run ~4 minutes; replaced by closed-form strike selection (~25-45 s).

## Risks / failure modes / model artifacts
- Capped payoff vs. low win rate trend system: structural mismatch.
- Whipsaw: index breakouts since 2005 frequently reverse within days (e.g. repeated Dec-Mar 2005-07 failures).
- Theta + bid/ask on ~20-DTE ATM spreads; each roll re-pays the spread.
- Account too small for SPY/QQQ at $400-600: only $1-wide, often slightly OTM, one-lot spreads possible; sequence of losses locks the account out (strategy self-terminates).
- Model-pricing: BS with VIX-derived vol; pre-2022 daily expiries assumed (20-DTE is fine, but strike grid $1 for QQQ pre-2010s is approximate). No artifact is helping this strategy; if anything, vol-model smile makes OTM long strikes slightly cheaper.

## Paper-trading notes
Not recommended for paper. If revisited: use long outright calls/puts in a larger account, 55/20 channels with a 200-day filter, or use the Donchian signal as a regime filter for another strategy rather than as a standalone spread trigger.
