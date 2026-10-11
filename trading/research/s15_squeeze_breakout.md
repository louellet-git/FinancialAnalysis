# s15_squeeze_breakout — volatility-compression breakout with 1-5 DTE debit spreads

**Verdict: NEGATIVE. The edge does not survive in the model-priced harness. Do not paper-trade as is.**

## Hypothesis
Volatility clusters and mean-reverts. A period of unusually compressed range (Bollinger
Bands inside Keltner Channels, the "TTM squeeze", or Crabel's NR7) tends to be followed by
range expansion. If expansion has directional persistence, a cheap short-dated debit
spread bought in the breakout direction should have positive expectancy.

## Evidence / rationale
- Volatility clustering and mean reversion (Engle ARCH; Mandelbrot) is well documented. It
  supports "compression is followed by expansion" in magnitude, not in direction.
- Crabel (1990) documents NR4/NR7 range expansion. Practitioner sources (Carter, TTM
  Squeeze) are anecdotal, with no rigorous peer-reviewed support for direction.
- The structural problem: expansion in magnitude is not an edge when the option price
  already reflects it. Implied vol (VIX/VXN, here model-priced) is mostly independent of
  the local 1h range compression. A debit spread pays only if direction is right, and
  direction after a release is close to a coin flip on SPY/QQQ.
- Structure under the $60 cap: $1-wide debit vertical, long leg ~0.40 delta, 3 DTE, debit
  about $0.40-0.50, so 1 unit (max loss incl. commissions about $48-55).

## Rules
- Symbols SPY, QQQ; 1h bars; warmup 40 bars; one position per symbol.
- SQUEEZE: BB(20, 2.0 sd) inside KC(20, 1.5 x mean true range) for at least 6 consecutive
  bars. The current bar is the first bar out of the squeeze. Direction is the sign of
  close minus the mean of (Donchian mid, SMA); with CONFIRM_RANGE the close must also
  clear the high/low of the squeeze window.
- NR7: previous daily range is the narrowest of 7 sessions; the first hourly close beyond
  its high/low gives the direction.
- Entry: decided at the bar close and filled at the next bar open. No entries off the
  session's last bar. Call debit spread if up, put debit spread if down, long strike at
  0.40 delta, 3 sessions to expiry.
- Exit: take profit at +80% of debit, stop at -50%, 14-bar max hold, on expiry day exit
  30 minutes before the close.
- Sizing: engine risk rules (1 unit at ~$60 max loss; larger debits are rejected).

## Parameters
MODE=squeeze, BB_LEN=20, BB_K=2.0, KC_MULT=1.5, MIN_SQUEEZE_BARS=6, CONFIRM_RANGE=True,
ALLOW_LAST_BAR=False, DTE=3, LONG_DELTA=0.40, WIDTH=1.0, TP=0.8, SL=0.5,
MAX_HOLD_BARS=14, EXIT_MIN_BEFORE_CLOSE=30.

## Results (final = default parameters)
```
s15_squeeze_breakout: 2023-11-10→2026-10-09  final $1,476
  ALL  ret -26.2%  CAGR -9.9%  maxDD -29.5%  Sharpe -1.47  trades 43  win 30%  PF 0.38
  OOS (from 2025-11-24)  ret +3.0%  maxDD -2.9%  Sharpe 0.90  trades 4  win 75%  PF 2.34
  rejected entries 7  PDT-blocked exits 0  (9.9s)
  rejections: {'too_risky_for_account': 7}
```
In-sample (first 70%): 39 trades, -28.3%, PF 0.31. The OOS result is 4 trades: not
evidence of anything and not used for selection. About two-thirds of the trades exit on
the -50% stop; the winners (take profit, max hold) are smaller than the losers once the
bid/ask spread and commissions ($2.60 round trip on a ~$45 risk) are included.

## Variants tried (in-sample tuning only; 6 variants + base)
| Variant | Trades (all) | IS return | IS PF |
|---|---|---|---|
| base (squeeze, 3DTE, TP .8 / SL .5, 14 bars) | 43 | -28.3% | 0.31 |
| NR7 only | 103 | -34.1% | 0.54 |
| squeeze + NR7 ("either") | 87 | -41.1% | 0.45 |
| no stop, TP 1.0, 28 bars | 35 | -31.4% | 0.34 |
| DTE 5, 0.50 delta, no stop, 21 bars | 21 | -16.1% | 0.25 |
| looser squeeze (4 bars, no range confirm), SL .7, 21 bars | 40 | -39.0% | 0.25 |

Every variant lost money in-sample, so none was promoted; the default (the lowest
complexity and the best-documented variant) is reported as final. A 1-DTE variant was
drafted but not run, since the trend across DTE/stop/hold settings showed no sign of an edge.

## Risks / failure modes / model artifacts
- Direction after a squeeze release is not reliably persistent; hourly releases frequently
  reverse (false breakouts), which the stop locks in.
- Costs dominate on a $1-wide spread: tight modeled spread plus $1.30 per spread per side.
- Few signals (~15 per year for the squeeze), so the sample is small; any positive OOS
  slice is noise. Also many NR7/either signals are rejected as too_risky_for_account
  (debit on 5 DTE 0.50 delta or higher-vol days exceeds the ~$60 cap): the cap filters
  entries toward cheaper (lower-vol) days, a selection effect.
- Model concern: the synthetic pricer uses VIX/VXN regardless of local compression, so
  it cannot capture any real "cheap vol before expansion" mispricing. A negative result
  here does not prove the idea fails with real quotes, but there is no support for it.

## Paper-trading notes
Not recommended. If revisited, test with real option quotes, add a higher-timeframe trend
filter (breakouts aligned with the daily trend), and compare against a random-direction
control on the same entry timestamps to isolate whether the squeeze timing adds anything.

## Harness notes
The assigned worktree did not contain the `trading/` package or `scripts/trade_backtest.py`;
the backtest was run from a copy of the harness extracted from harness.tar in the scratchpad
(`TRADING_DATA_DIR=/home/user/FinancialAnalysis/data/raw/trading`).
