# s08_trend_regime — trend-following regime with rolling vertical spreads

**Verdict: NEGATIVE result. No edge survives costs and the $2k / 3%-risk constraints.**

## Hypothesis
Time-series momentum / trend filters (price vs 200-day SMA, 50/200 cross) improve the
risk-adjusted return of equity-index exposure (Faber 2007 "A Quantitative Approach to Tactical
Asset Allocation"; Moskowitz-Ooi-Pedersen 2012; Hurst-Ooi-Pedersen). Expressing the long-trend
leg as a rolling 30-session call debit spread should give cheap, defined-risk participation in
uptrends, with downtrends traded via put debit spreads or skipped.

## Rules
- Regime (completed daily closes): UP = close > SMA200 and SMA50 > SMA200; DOWN = both below;
  otherwise NEUTRAL (no new entries). ADX filter available (`ADX_MIN`, off by default).
- Entry (UP): SPY $1-wide call debit spread, long leg at ~0.50 delta, 30 sessions to expiry,
  one spread at a time. Decision at close, fill at next open (engine rule).
- Exit: roll at 10 sessions to expiry (`dte_exit`; re-entered next session if regime still UP),
  take profit at +80% of debit, close if regime turns DOWN (NEUTRAL does not close by default).
- Sizing: engine-sized to 3% of equity max loss (in practice always 1 unit).
- Defaults: FAST=50, LONG=200, DTE=30, ROLL_DTE=10, CALL_DELTA=0.50, WIDTH=1, TP=0.8, TRADE_DOWN=False, NEUTRAL_EXIT=False. Start 2005-01-01, SPY, 1d.

## Results (final variant, SPY, 2005-01-03 to 2026-10-09)
```
s08_trend_regime: 2005-01-03→2026-10-09  final $1,654
  ALL  ret -17.3%  CAGR -0.9%  maxDD -20.9%  Sharpe -0.24  trades 102  win 49%  PF 0.74
  OOS (from 2020-03-26)  ret +1.9%  maxDD -2.3%  Sharpe 0.25  trades 3  win 67%  PF 2.10
  rejected entries 1918  PDT-blocked exits 0
  rejections: {'too_risky_for_account': 1918}
```
Sample-size caveat: the OOS window holds only 3 trades. SPY is above $300 there and, with
equity already drawn down, a ~$0.52 debit spread (+ $2.60 round-trip commission) exceeds 3% of
equity, so almost every entry is rejected. The OOS +1.9% is statistically meaningless.

## Variants tried (in-sample judged; ~7 runs on full history)
| # | Variant | ALL ret | maxDD | PF | trades |
|---|---|---|---|---|---|
| 1 | calls UP + puts DOWN, delta .50, TP .8, flip-close on NEUTRAL | -27.8% | -29.7% | 0.58 | 86 |
| 2 | same, delta .30 | -58.9% | -60.1% | 0.42 | 144 |
| 3 | delta .30, flat in downtrends | -46.7% | -50.3% | 0.58 | 183 |
| 4 | put credit spread in UP (short delta .25), 45/21 DTE, TP .5 | 0 trades (all rejected) | | | 0 |
| 5 | put credit spread UP + put debit DOWN | -19.6% | -19.9% | 0.25 | 29 |
| 6 | delta .50, flat DOWN, close on NEUTRAL | -19.3% | -23.0% | 0.68 | 94 |
| **7 (final)** | delta .50, flat DOWN, hold through NEUTRAL | **-17.3%** | **-20.9%** | **0.74** | 102 |
| 8 | #7 with delta .40 | -31.3% | -33.9% | 0.69 | 150 |
| 9 | #8 with 45/21 DTE, no TP | -29.3% | -31.1% | 0.61 | 134 |

(More than 6 runs were spent partly on diagnosing a structural infeasibility, see below; the
final default was chosen as the best in-sample, not on OOS.)

## Why it fails / risks
1. **Credit spreads are infeasible**: a $1-wide put credit spread risks $0.75-0.90/share
   ($75-90 + commissions) vs a ~$57-60 budget, so every entry is `too_risky_for_account`.
2. **Debit spreads are near-binary bets with tiny dollar width.** A $1-wide spread on a $100-$600
   underlying only pays if the underlying ends above the long strike + $1; a 0.50-delta spread
   costs ~$0.50 plus bid/ask, so the fair-value edge from the trend premium (a few % annualised
   drift conditioning) is smaller than costs: $2.60 round-trip commission is ~5% of a ~$52 debit,
   per roll, ~12 rolls/year.
3. **Whipsaws**: 50/200 and price/200d filters lag; in 2005-2011 (incl. 2008 and 2011) the regime
   flipped repeatedly and the account shrank; a shrinking account then cannot afford the minimum
   1-unit spread (rejections), so the strategy cannot recover. Path-dependent ruin of the tiny account
   is the dominant effect, not the signal.
4. Model artifact: option prices are Black-Scholes on VIX with skew; a trend filter is not an
   implied-vs-realised play, so the result is not obviously a model artifact, but the 2005-2022
   daily-expiry assumption (monthly/weekly in reality) is approximate for a 30-session tenor
   only mildly.

## Paper-trading notes
Not recommended. If revisited: use cheaper structures (long outright OTM calls are blocked by
the same budget at SPY>$400), QQQ/SPY with 0.5-wide strikes if available, or use trend only as a
filter on another strategy rather than as the P&L driver.
