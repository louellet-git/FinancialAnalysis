# s06_last_hour_momentum — last-half-hour intraday momentum via 0DTE debit spread

**Verdict: NEGATIVE.** The momentum effect is absent in the available SPY/QQQ 1h sample (Nov 2023 - Oct 2026), and the options structure loses heavily after spread/commission/theta.

## Hypothesis
Gao, Han, Li & Zhou (2018, JFE, "Market intraday momentum"): the return from the prior close to the first 30 minutes predicts the return of the last 30 minutes (same sign; also the 13:30-14:00 return, and the effect is stronger on high-vol / high-volume / macro-news days). Mechanisms: late-day informed/institutional trading, rebalancing of leveraged ETFs (Cheng-Xu / Shum et al.), gamma hedging by dealers short 0DTE options. Maps to: at 15:30 ET buy a $1-wide 0DTE debit spread in the signalled direction, hold to the 16:00 expiry (settles at intrinsic; expiry is not a PDT day trade).

## Evidence / rationale
Literature: 1993-2013 SPY, R^2 ~1-2%, hit rate a few points above 50%. Published effects are small (a couple bp on a ~20-25bp last-half-hour stdev) — an option buyer pays bid/ask + commission on ~$0.3-0.5 of premium, so a thin edge must be large to survive.

Model-free check on this repo's data (SPY and QQQ 1h, 720 full sessions each, last bar = 15:30-16:00; script in session scratchpad):

| signal | SPY corr / hit / mean signed ret (bp) / t | QQQ |
|---|---|---|
| first bar (prev close -> 10:30) | -0.04 / 49.4% / -0.59 / -0.74 | -0.01 / 48.5% / -0.64 / -0.64 |
| day so far (prev close -> 15:30) | -0.04 / 45.1% / -2.05 / -2.55 | -0.03 / 48.1% / -1.96 / -1.97 |
| 10:30 -> 15:30 | -0.02 / 45.0% / -1.64 / -2.04 | -0.03 / 49.0% / -1.69 / -1.69 |

No momentum; day-so-far shows a mild reversal (t about -2, but zeros count as misses and this is one 2-year regime, so do not trade the reversal on this evidence). 5m data (~60 days) was not used: too small to estimate a 1-2bp effect; 1h is the finest interval with ~2 years. Limitation: first bar is 1 hour (9:30-10:30), not the paper's 30 minutes.

## Rules
- Symbol SPY, interval 1h. Decision when the 14:30 bar closes (15:30 ET); order fills at the 15:30 bar open.
- Signal: z = return / (VIX/sqrt(252)). SIGNAL="first": z of (10:30 close / prior close - 1). Enter if |z| >= Z_MIN (0.3) and |day-so-far z| <= 3.
- Structure: z>0 -> call debit spread long at ceil(spot), short +$1; z<0 -> put debit spread long floor(spot), short -$1. Expiry today (0DTE).
- Exit: none (holds to expiry settlement, intrinsic). TAKE_PROFIT/STOP_LOSS unused (only 30 minutes / one bar remain, so intrabar exits can't fire).
- Sizing: engine (3% of equity max loss; typically 1 unit, ~$25-55 risk).

## Parameters
SIGNAL="first", Z_MIN=0.3, WIDTH=1.0, LONG_OFFSET=0.0, MAX_DAILY_Z=3.0, TAKE_PROFIT=None, STOP_LOSS=None.

## Results (final config, SPY, 2023-11-10 to 2026-10-09)
```
s06_last_hour_momentum: 2023-11-10→2026-10-09  final $570
  ALL  ret -71.5%  CAGR -35.0%  maxDD -71.5%  Sharpe -1.68  trades 149  win 19%  PF 0.53
  OOS (from 2025-11-24)  ret -22.7%  maxDD -22.7%  Sharpe -3.57  trades 11  win 0%  PF 0.00
  rejected entries 214  PDT-blocked exits 0
  rejections: {'too_risky_for_account': 214}
```
OOS has only 11 trades (the harness's "last 30%" window is short because 1h history is ~2 yrs and signals are sparse) — statistically meaningless; the full-sample 149 trades are the informative number. PDT is not a constraint (0 blocks).

## Variants tried (7 runs; none profitable)
| variant | ALL ret | win | PF |
|---|---|---|---|
| ATM long strike (nearest $1), first | -56.4% | 29% | 0.61 |
| **first, long at ceil/floor (final)** | -71.5% | 19% | 0.53 |
| day | -76.6% | 13% | 0.43 |
| both agree | -71.2% | 18% | 0.41 |
| day, QQQ | -76.7% | 8% | 0.16 |
| day, Z_MIN 0.6 | -70.7% | 17% | 0.53 |
| both, Z_MIN 0.15 | -75.2% | 16% | 0.41 |

The ATM variant was the least bad but triggered many `too_risky_for_account` rejections ($1-wide ATM spread near $0.55-0.60 + commissions exceeds the $60 cap), so the OTM-side strike was kept as final to stay inside the risk limits, chosen for the paper's original signal (first period), not on OOS. The conclusion is insensitive to choice: every variant loses.

## Risks / model-artifact concerns
- Result is dominated by cost drag, not a model artifact: a $1 spread bought at ask and settled at intrinsic with ~19% wins; the underlying signal itself shows ~0 edge model-free.
- The synthetic pricer's 0DTE short-vol (VIX1D) at 15:30 controls spread cost; real 0DTE ATM spreads late in the day are often priced richer (gamma premium) and wider — real results likely no better.
- Equity decay shrinks the $60 budget -> many rejections late in the run; max drawdown of -70% breaches the 25-30% target.
- Sample covers only ~3 years in one regime; the literature effect may exist but has weakened post-2013 (widely noted: alpha decay as 0DTE/leveraged-ETF flows grew).

## Paper-trading notes
Not recommended. If monitored anyway: log the first-hour return and 15:30-16:00 SPY return daily (free, no option risk) and revisit only if the signed-return t-stat turns positive over 250+ sessions; use 30-minute bars to match the paper. Check actual 15:30 spread prices for $1-wide ATM 0DTE (likely > $0.5).
