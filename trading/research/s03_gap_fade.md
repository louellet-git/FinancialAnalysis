# s03_gap_fade: overnight gap fade with 0DTE debit spreads

**Verdict: NEGATIVE. No tradable edge survives on SPY/QQQ 1h data, and the structure/commission/sizing constraints make it worse.**

## Hypothesis
Moderate overnight gaps (open vs prior close) partly fill intraday. Normalizing the gap by the VIX-implied daily move (VIX/100/sqrt(252)) separates "noise" gaps (fade) from news gaps (continue, skip).

## Evidence / rationale
Practitioner lore ("gaps get filled") and some older index-futures studies suggest partial fills for small gaps. Opposing evidence: the Lou-Polk-Skouras overnight-vs-intraday work shows overnight returns are persistent, and since ~2015 the opening gap in SPY is mostly information. Before building, I tested the underlying directly (SPY and QQQ 1h, ~2.9 years, gap measured on the 9:30 open, entry at the 10:30 close with at least 50% of the gap still unfilled, return from 10:30 to the close in the fade direction):

| z = |gap| / VIX-daily-move | SPY n | SPY fade bps (t) | QQQ n | QQQ fade bps (t) |
|---|---|---|---|---|
| <0.25 | 164 | +4.0 (0.6) | 141 | +6.9 (0.8) |
| 0.25-0.5 | 91 | -5.3 (-1.1) | 88 | -9.3 (-1.6) |
| 0.5-1.0 | 74 | +0.3 (0.1) | 87 | -4.1 (-0.6) |
| 1.0-2.0 | 15 | +55 (1.3) | 20 | +51 (1.4) |

Moderate gaps (0.25-1.0) show zero-to-negative fade returns after the first hour. The only positive bucket (z>1, the "news gap" zone the thesis says to avoid) has n=15-20 and is not significant. There is no underlying edge to monetize.

## Rules
- Interval 1h, SPY. Decide at the close of the first bar (10:30 ET); the order fills at the next bar's open.
- z = |open - prev_close| / (VIX_atm/sqrt(252) * prev_close). Trade when MIN_Z <= z <= MAX_Z (0.5-1.5) and at least 50% of the gap is still open at 10:30.
- Gap up: 0DTE put debit spread, long ATM, short $1 below. Gap down: 0DTE call debit spread, long ATM, short $1 above.
- Exits: gap fill (spot back at prior close, via ctx.close), take profit +80% of debit, stop -50% of debit, time exit 60 minutes before the close.
- Sizing is the engine's: units = floor($60 / (max loss + commissions)). One position at a time.

## Parameters
MIN_Z=0.5, MAX_Z=1.5, MIN_REMAIN=0.5, WIDTH=1.0, DTE=0, TAKE_PROFIT=0.8, STOP_LOSS=0.5, MINUTES_BEFORE_CLOSE=60, GAP_FILL_EXIT=True.

## Results (final config, SPY, 2023-11-10 to 2026-10-09)
```
s03_gap_fade: 2023-11-10→2026-10-09  final $1,435
  ALL  ret -28.2%  CAGR -10.8%  maxDD -30.6%  Sharpe -1.20  trades 71  win 31%  PF 0.58
  OOS (from 2025-11-24)  ret -3.2%  maxDD -7.3%  Sharpe -0.51  trades 10  win 40%  PF 0.71
  rejected entries 78  PDT-blocked exits 0
  rejections: {'too_risky_for_account': 78}
```
Sample caveat: 71 trades total, only 10 OOS. The OOS number is not evidence of anything. The in-sample result is clearly negative (PF 0.58).

## Variants tried (in-sample judgement; every one lost money)
| # | Variant | ALL ret | ALL PF | trades |
|---|---|---|---|---|
| 1 | SPY, z 0.25-1.0, no gap-fill close | -36.2% | 0.60 | 100 |
| 2 | + gap-fill close | -37.2% | 0.61 | 109 |
| 3 | QQQ, z 0.25-1.0 | -30.5% | 0.74 | 142 |
| 4 | SPY 1DTE, TP 60% | -32.8% | 0.46 | 77 |
| 5 | SPY z 0.5-1.5 (final) | -28.2% | 0.58 | 71 |
| 6 | QQQ z 0.5-1.5 | -28.5% | 0.70 | 111 |

Final choice is the least-bad SPY configuration, consistent with the raw-underlying study that moderate gaps carry no fade drift. It is not a recommendation to trade it. QQQ was equally negative (OOS PF 0.46).

## Risks / failure modes / model-artifact concerns
- Costs: ATM $1 0DTE spreads at 10:30 cost ~$0.50-0.55, so the $60 cap often rejects the trade (78-156 `too_risky_for_account` rejections) and commissions ($2.60 round trip per unit) are ~5% of risk. Both are structural for a $2k account.
- Theta: 0DTE ATM debit spreads need a move of ~half a point or more within hours; with no underlying drift, decay and bid/ask spread dominate.
- Model artifacts: pricing is Black-Scholes on VIX/VIX1D; I do not see a mispricing edge here, since the result is negative before any artifact could help.
- Rejections skew the sample toward high-IV days when debit exceeds the budget; as equity shrinks, more trades are rejected, so late-sample trade counts understate the signal frequency.
- Gap fill needs the 10:30 bar data only; no lookahead (gap uses first-bar open and `ctx.daily()` prior close).

## Paper-trading notes
Do not deploy. If someone revisits: test the z>1 bucket with more data (n=15-20 is too small), try 5m bars for entry within the first 15-30 minutes, and use a wider OTM structure only if the account can bear it. Watch fill rate of the gap-fill exit versus the stop.
