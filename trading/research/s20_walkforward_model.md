# s20_walkforward_model — walk-forward logistic direction model, 5-DTE debit spreads

**Verdict: NEGATIVE.** The model's out-of-sample hit rate is indistinguishable from "always up"; the traded strategy returns about 0 after commissions. Do not paper-trade.

## Hypothesis
A small, regularised statistical model on well-known daily features (momentum/reversal at several horizons, distance from moving averages, realised vol, VIX level/change, VIX-minus-realised, day of week) can predict the sign of the next 5-session SPY/QQQ return well enough that, when it is highly confident, a defined-risk 5-DTE debit spread has positive expectancy.

## Evidence / rationale
Weak-form predictability in index returns is small but documented: short-horizon reversal and medium-horizon trend (time-series momentum, Moskowitz-Ooi-Pedersen 2012), trend-vs-MA filters (Faber 2007), VIX level and VIX-realised gaps as risk-premium proxies, calendar effects. Literature effect sizes are a few percent of variance at best, so the realistic expectation was a small lift over the ~60% unconditional 5-day up rate. The test is honest only if the model is strictly walk-forward and judged against that base rate, not against 50%.

## Rules
- **Model** (`WalkForwardModel`, numpy only): L2 logistic regression (Newton, intercept unpenalised), 14 features, standardised with TRAIN-window mean/std only. Target = close[t+5] > close[t]. Expanding window, refit every 21 sessions, min 750 labelled samples (first prediction ~2008-10). A sample from bar t is admitted to training only when t+5 <= now, so no label is used before it is observable. Features use only `ctx.bars()` / `ctx.vol_history()` (<= current close). Stored predictions are in `strategy.pred_log` for audit.
- **Entry** (decided at close, filled next open): P(up) >= 0.62 -> call debit spread; P(up) <= 0.38 -> put debit spread. One position at a time.
- **Structure**: $1-wide vertical, long leg at ~0.50 delta, expiry = DTE 5 sessions after fill (engine sizes to the $60 cap; commissions included).
- **Exit**: hold; exit_dte=0 (expiry day close / settlement). No TP/SL.

## Parameters
HORIZON 5, LAMBDA 300, RETRAIN 21, MIN_TRAIN 750, P_HI 0.62, P_LO 0.38, DTE 5, DELTA 0.50, WIDTH 1.0, EXIT_DTE 0. Symbol SPY (QQQ checked for model quality only), start 2005-01-03.

## Results (SPY, final config)
```
s20_walkforward_model: 2005-01-03→2026-10-09  final $1,944
  ALL  ret -2.8%  CAGR -0.1%  maxDD -24.7%  Sharpe 0.00  trades 91  win 51%  PF 0.97
  OOS (from 2020-03-26)  ret +6.8%  maxDD -7.8%  Sharpe 0.26  trades 16  win 56%  PF 1.38
  rejected entries 91  PDT-blocked exits 0
  rejections: {'too_risky_for_account': 91}
```
Only 16 OOS trades: that +6.8% is noise, not evidence.

### Model-only out-of-sample quality (the key number)
All predictions are walk-forward (every prediction from 2008-10 on is out-of-sample for the model). SPY, N = 4,518 predictions:

| Slice | n | Hit rate | Always-up baseline |
|---|---|---|---|
| All predictions (p > 0.5 => up) | 4518 | 59.3% | 60.1% |
| Pre-split (first 70%) | 2879 | 58.9% | 60.1% |
| Last 30% | 1639 | 60.0% | 60.2% |
| High-confidence (p>=.62 or <=.38) | 297 | 63.6% | (up-rate in those days 64.3%) |
| High-conf, last 30% | 90 | 68.9% | 68.9% |

Interpretation: the model almost never predicts "down" confidently (32 cases p<0.38; 46.9% hit), and its high-confidence calls are "up" calls made in calm, trending regimes where the 5-day up rate is higher anyway. Hit rate of confident calls (63.6%) is ~3.5 pp above the unconditional 60.1% with n=297, i.e. ~1.2 standard errors: not significant. The signal is mostly an "uptrend/low-vol regime" filter, which is beta, not forecasting skill. QQQ: all 59.8% vs 60.3% base; high-conf 62.3% (pre 64.4%, last-30% 57.6%, n=92) — also no edge; the QQQ spread backtest lost ~$240 over 70 trades.

## Variants tried (6 runs, chosen on pre-split behaviour; none cherry-picked on OOS)
1. **Final**: LAMBDA 300, DELTA 0.50 — ALL -2.8%, PF 0.97.
2. DELTA 0.40 — ALL -6.2%, PF 0.96 (cheaper spread, lower win rate).
3. LAMBDA 30 (less regularisation) — model hit rate falls to 57.8% overall (overfits); strategy -54%.
4. LAMBDA 30, thresholds .58/.42 — -57%, 166 trades, 28% win.
5. LAMBDA 3000 — model collapses to the base rate (60.0%); only 8 trades in 21 years.
6. QQQ with the default config (model-quality diagnostics above, strategy loses money).
Pattern: more flexibility = worse out-of-sample hit rate; maximal regularisation = intercept only. There is no regularisation level that beats the trivial predictor. Final kept at the a priori defaults.

## Risks / failure modes / model artifacts
- **Base-rate trap**: SPY's 5-day up rate is ~60%; any "hit rate" quoted without that baseline is misleading. Always-up on a debit spread is also not free: ~50% of the spread's premium is lost to the bid/ask + $2.60 round-trip commission on a $0.50 spread (~5% of premium per trade).
- **Cost drag**: a $1-wide spread on a ~$0.50 debit with $1.30 per side commission makes the edge hurdle large. At SPY ~$780, a $1-wide spread is ~0.13% of spot, so the structure is very sensitive to tiny moves, while at SPY ~$100 (2008-2012) it is a very different bet; results mix regimes.
- `too_risky_for_account` rejects 91 entries (high-VIX states where the 0.5-delta $1 debit exceeds the $60 budget) — the trades the account skips are disproportionately the volatile ones.
- Pre-2022 daily expiries are assumed; prices model-priced from VIX/VXN. The model uses VIX for QQQ's features as well as VXN-based pricing; any "edge" here would not come from option mispricing, which is why none was found.
- Overlapping 5-day labels make training samples highly autocorrelated; effective sample size is ~1/5 of N. Another reason the confidence estimates are unreliable.

## Paper-trading notes
Not recommended. If anyone wants to monitor: log `pred_log` live and compute rolling hit rate of confident calls against the always-up baseline for the same days; require significance (>200 confident calls) before risking capital. Nothing here suggests it will pass.
