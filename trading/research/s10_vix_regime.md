# s10_vix_regime — VIX/VXN regime timing for vertical spreads

## Hypothesis
Short-dated put credit spreads harvest the variance risk premium, but only pay in
"calm but not complacent" vol; when vol breaks out the premium is a trap. Classifying
regimes from the vol index (level vs 50d MA, 252d percentile, 5d change) should let a
bot sell puts in calm and step aside (or buy put debit spreads) in breakouts.

## Evidence / rationale
VRP literature (Bakshi-Kapadia, Carr-Wu; CBOE PUT index) shows implied > realized on
average; vol clusters and spikes mean-revert slowly, so breakouts are the dangerous
state for short vol. Mapping under the $60 cap is the problem: see Risks.

## Rules
- Regime from ctx.vol_history()["atm_close"] (VXN for QQQ): ratio = VXN/MA50, pct = 252d percentile.
- Calm: ratio <= 1.05, 0.20 <= pct <= 0.75, VXN <= 24, price > 0.98 x 50d MA -> sell put credit spread.
- Breakout (ratio >= 1.25, or 5d change >= 30% with price < 20d MA): optional put debit spread (DEBIT flag, off in final). Panic (> 35): flat.
- Structure: $1-wide put credit spread, 21 DTE. Short strike = first strike at/above 0.30 delta whose max loss (incl. commissions) fits 97% of the 3% budget. Credit ~= delta x width, so this forces ~0.45-0.5 delta shorts.
- Exit: TP at 50% of credit, no stop (loss already capped), time exit at 3 sessions to expiry.
- Sizing: engine (1 unit).

## Parameters
UPPERCASE attributes in the strategy file (DTE=21, WIDTH=1, SHORT_DELTA=0.30, PCT 0.20-0.75, CR_TP 0.5, EXIT_DTE 3, DEBIT=False).

## Results (final, QQQ)
```
s10_vix_regime: 2005-01-03→2026-10-09  final $1,564
  ALL  ret -21.8%  CAGR -1.1%  maxDD -26.0%  Sharpe -0.36  trades 52  win 48%  PF 0.58
  OOS (from 2020-03-26)  ret +0.0%  maxDD 0.0%  Sharpe 0.00  trades 0  win 0%  PF 0.00
  rejected entries 9  PDT-blocked exits 0
  rejections: {'too_risky_for_account': 9}
```
52 trades in 21 years is a tiny sample and OOS has ZERO trades, so there is no out-of-sample evidence. Negative result: no edge.

## Variants tried (5 configurations, all negative)
1. SPY, 14 DTE, 0.30-delta $1 put credit, SL 1x, + breakout put debit: -39.8%, 76 trades, PF 0.45 (2037 entries rejected as too_risky).
2. Same, stepping the strike up until loss fits budget: -42.7%, 95 trades, PF 0.52 (credit -8.9/trade, debit -9.1/trade).
3. SPY credit only, no stop: -38.2%, PF 0.47.
4. SPY 7 DTE, tighter calm, debit on: -56.9%, PF 0.39.
5. QQQ 21 DTE credit only, no stop: -21.8%, PF 0.58 (chosen as least bad, not a claimed edge).
Chosen on full-sample behaviour; OOS had 0-1 trades so nothing was cherry-picked on it.

## Risks / failure modes / model-artifact concerns
- Structural: with $1-wide spreads and a ~$58 loss cap, max loss must be <= ~0.57 x width, so credit >= ~43% of width, i.e. short strike ~0.45-0.5 delta: nearly a directional coin flip, not a VRP harvest. Friction ($2.60 round-trip commission plus ~$4-8 of bid/ask on two legs) is ~15-20% of the max win.
- Losses shrink equity, which shrinks the 3% budget, after which nothing fits and the account stops trading (SPY after ~2015, QQQ after ~2018). That is why OOS has 0 trades: a sizing death spiral, not a signal result.
- Pricing is synthetic: 30d VIX/VXN is the IV for 7-21 DTE (flat term structure pre-2023), skew is a fixed quadratic, so the filters cannot show a term-structure/skew edge the model lacks. VIX1D/VIX is unusable pre-2023 (falls back to VIX, ratio = 1).
- Daily expiries assumed pre-2022 (less critical at 7-21 DTE).

## Paper-trading notes
Do not trade as is. If revisited: needs >= $4-5k equity so ~0.25-0.30 delta shorts fit, then re-test the regimes; compare live credit fills with model mid minus ~$0.05; log VIX/MA50 and percentile at each signal.
