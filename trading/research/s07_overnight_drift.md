# s07_overnight_drift — overnight drift via 1DTE call debit spreads (SPY / QQQ, 1h bars)

**Verdict: NEGATIVE.** The stock-level overnight drift is real in this sample, but a $1-wide call
debit spread bought at 15:30 and sold next morning does not monetize it after model spreads and
commissions. Not recommended for paper trading.

## Hypothesis
Most equity-index return accrues close-to-open (Cooper/Cliff/Gulen "Return Premiums and Daily
Seasonality"; Lou/Polk/Skouras "A Tug of War: Overnight vs Intraday Expected Returns"; Kelly &
Clark on S&P futures). Express it with a near-ATM 1DTE call debit spread held overnight.

## Evidence in the data (SPY 1h, 720 sessions, 2023-11 to 2026-10)
Mean return from 15:30 open to next-day 09:30 open: +6.8 bp (std 64.5 bp); to next-day 10:30
close: +7.2 bp (std 72.8 bp). On SPY ~$600-780 this is ~$0.45-0.55 per share of mean drift
against a ~$5 daily-range noise: t-stat about 2.7 over the full sample, a modest, noisy edge.

## Rules
- Interval 1h, symbol SPY (QQQ variant tested). Signal on the 14:30 bar (known at 15:30) ->
  engine fills at the 15:30 bar's open. The 15:30 bar is the latest entry available on 1h bars.
- Entry: buy 0.50-delta call / sell call $1 higher, expiry = next session (DTE=1).
- Filters: spot > 50-day SMA of daily closes; VIX (VXN for QQQ) <= 30; skip Fridays.
- Exit: `max_hold_bars=2` -> exits at the CLOSE of the next day's 09:30 bar (10:30). No stop.
- Sizing: engine (3% of equity max loss; effectively 1 spread).

## Parameters
DTE=1, DELTA=0.50, WIDTH=1.0, SMA_DAYS=50, VIX_MAX=30, SKIP_FRIDAY=True, STOP_LOSS=None.

## Results (final config, SPY, defaults)
```
s07_overnight_drift: 2023-11-10→2026-10-09  final $1,578
  ALL  ret -21.1%  CAGR -7.8%  maxDD -22.6%  Sharpe -0.90  trades 118  win 53%  PF 0.72
  OOS (from 2025-11-24)  ret +0.0%  maxDD 0.0%  Sharpe 0.00  trades 0  win 0%  PF 0.00
  rejected entries 341  PDT-blocked exits 0
  rejections: {'too_risky_for_account': 341}
```
OOS has ZERO trades: SPY rose to ~$780, equity fell, and the $1-wide ATM spread's debit plus
round-trip commissions ($2.60) exceeded the 3% risk budget. So OOS is uninformative. 118 trades
is a small sample.

## Variants tried (ALL-sample; OOS mostly empty so decisions were in-sample only)
| variant | return | maxDD | PF | trades |
|---|---|---|---|---|
| base (above) | -21.1% | -22.6% | 0.72 | 118 |
| no filters, Fridays included | -21.7% | -23.3% | 0.62 | 80 |
| delta 0.35 | -44.4% | -49.4% | 0.72 | 248 |
| delta 0.65 | 0 trades (all too risky) | | | 0 |
| DTE 2 | -17.6% | -18.8% | 0.57 | 78 |
| QQQ, delta 0.50 | -14.0% | -20.6% | 0.72 | 78 |
| QQQ, delta 0.35 | -45.9% | -48.0% | 0.75 | 265 |
None is profitable; base kept as the least bad on SPY with sensible filters (QQQ marginally
less bad, OOS 4 trades PF 1.11, which is not meaningful).

## Why it fails / risks
- Cost vs. edge: a ~7 bp mean drift is ~$0.5/share on the underlying, but a ~0.5-delta $1-wide
  spread captures only ~$0.2-0.25 of that, while the model's 1c-minimum leg spreads (4 legs
  crossed round trip) plus $2.60 commissions cost ~$0.05-0.10/share, and the capped payoff
  gives away large-move upside. The remaining per-trade expectancy is about zero with +/-$30 noise.
- Harness limits: exits fill only at bar CLOSE, so the position is held to 10:30 (not the open),
  adding an hour of noisy intraday exposure.
- Model-artifact concern: pricing uses trading-time only (`year_fraction` skips nights), so the
  model charges almost no overnight theta while the realized gap hits fully. This FLATTERS the
  strategy; the strategy still lost, so the negative result is robust to that bias.
- Affordability: as SPY/QQQ price rises and equity falls, narrow ATM spreads exceed the $60 cap
  (341 rejections), so the strategy self-starves.

## Paper-trading notes
Not recommended. If tested live: record real quote bid/ask on 1DTE $1 spreads at 15:30 and the
real overnight theta, which the model omits; check whether the real spread cost is below ~3c/leg.
A better vehicle for the premium would be shares or futures, not capped-payoff spreads.
