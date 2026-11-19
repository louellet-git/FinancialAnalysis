# s02_vwap_revert_0dte — VWAP mean reversion with 0DTE spreads

**Verdict: NEGATIVE. No tradable edge after spread + commission on a $2k account.** Every variant lost money; the final (least-bad) config is -13% over ~3 years (PF 0.81).

## Hypothesis
When SPY/QQQ stretches far from session VWAP in a non-trending session, it tends to revert toward VWAP; a 0DTE vertical expresses this cheaply.

## Evidence / rationale
Practitioner lore (VWAP as institutional benchmark / intraday magnet; published VWAP work on SPY finds trend-following works on strong-momentum days, i.e. a fade can only work when trend is absent). A prior stopped attempt found no unconditional fade edge. Pre-test (hourly, 721 sessions, VIX1D-scaled stretch z = (close-VWAP)/(daily 1-sd), forward return to the close, sign-flipped): SPY mean fade return 0.01-0.04 daily sd (t mostly < 2, hit 50-55%); QQQ 0.03-0.10 daily sd with an efficiency-ratio filter (er<0.4/0.6, t 2.4-4.2, but observations overlap within a day so t is inflated, and ER uses open->now, which is mechanically correlated with z). Magnitude is ~3-10 bp of underlying: smaller than a 0DTE spread's bid/ask plus $2.60/unit round-trip commission. Conditioning on non-trending sessions raised the (small) QQQ effect and did nothing for SPY.

## Rules (final defaults)
- Interval 1h, QQQ. VWAP = cumulative (H+L+C)/3 x volume from ctx.today_bars.
- Stretch z = (close - VWAP)/(close x VIX1D/sqrt(252)); trigger |z| >= K = 0.25.
- Non-trend filter: |close - day open| / (day high - day low) <= 0.6.
- Decision window: closes of the 10:30..14:30 bars (index 1..4); one trade per day.
- Structure (MODE=credit): below VWAP -> sell 0.50-delta put, buy put $1 lower (put credit spread); above VWAP -> mirror call credit spread. 0DTE. (MODE=debit: 0.45-delta $1 debit spread toward VWAP.)
- Exits: close when price re-crosses VWAP (ctx.close), TP 50% of credit, SL 100% of credit, time exit 60 min before close. Sized by the engine ($60 cap).

## Parameters
K, ER_MAX, FIRST_BAR/LAST_BAR, MODE, DELTA, CREDIT_DELTA, WIDTH, TAKE_PROFIT, STOP_LOSS, MINUTES_BEFORE_CLOSE, VWAP_EXIT_BUFFER (class attributes).

## Results (final)
```
s02_vwap_revert_0dte 1h 2023-11-10..2026-10-09 final_equity 1736.45
ALL: return -13.18%  maxDD -23.98%  Sharpe -0.52  trades 91  win 52.75%  PF 0.81
OOS: return  -3.76%  maxDD  -8.13%  Sharpe -0.59  trades 10  win 50.0%   PF 0.64
rejected 217 (too_risky_for_account: 217)   PDT-blocked exits 4
```
OOS has only 10 trades: statistically meaningless; the full sample (91 trades) is the informative number, and its point estimate is negative.

## Variants tried (judged on whole-sample/in-sample behavior; 8 runs incl. final, slightly over the ~6 budget)
| Variant | ALL ret | trades | PF |
|---|---|---|---|
| base QQQ debit 0.45d, K .25, TP100/SL50 | -43.9% | 127 | 0.54 |
| SPY instead of QQQ | -42.8% | 128 | 0.56 |
| no TP/SL (VWAP/time only) | -44.1% | 106 | 0.50 |
| K=0.40 | -39.9% | 77 | 0.37 |
| ER_MAX=0.4 | -40.8% | 90 | 0.41 |
| debit delta 0.30 | -65.4% | 139 | 0.45 |
| credit 0.30d short | 0 trades (all too risky: max loss > $60) | | |
| **credit 0.50d short, TP50/SL100 (final)** | **-13.2%** | 91 | 0.81 |

Debit spreads bleed because spread + commission (roughly 8-10% of a ~$0.50 premium round trip) exceeds the tiny reversion drift, and stops are hit about twice as often as VWAP touches. Selling credit roughly halves the drag but still loses. The final config was chosen only because it is the least bad; it is not a positive-expectancy strategy.

## Risks / model-artifact concerns
- Prices are Black-Scholes on VIX1D with synthetic skew. The credit variant's relative improvement may partly reflect VIX1D usually exceeding realized intraday vol (a model/real-market premium effect), so treat it as model-dependent.
- Hourly bars: VWAP is approximated from 7 bars/day and entries lag by up to an hour; the 5m sample (~60 days) was too small to test.
- $1-wide credit spreads at < ~0.45 delta exceed the $60 risk cap (217 rejections), forcing ATM short strikes, the highest-gamma, least favourable strikes.
- PDT: 4 exits were blocked and rode to expiry.
- Sample limited to ~2.9 years (VIX1D / hourly data availability).

## Paper-trading notes
Not recommended as a standalone strategy. If revisited: use 5m bars with true intraday VWAP over a larger sample, QQQ only with the efficiency-ratio filter, and measure realized slippage on $1-wide ATM 0DTE spreads versus the model's 1-5c half-spread. The only conditional signal seen (QQQ, low efficiency ratio, ~5-10 bp) is too small to survive commissions on a $2k account.
