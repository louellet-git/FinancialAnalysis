# s18_pin_butterfly — late-day 0DTE pin butterfly

## Hypothesis
In the last 1-2 hours, a quiet tape that sits near a "magnet" level (session VWAP, round number, afternoon-range midpoint) tends to finish near it. A $1-wide call butterfly (or iron butterfly) centred there costs ~$0.10-0.35 for a $1.00 max payoff, a 3-10x payout, held to expiry.

## Evidence / rationale
Practitioner lore: strike pinning from dealer gamma hedging and open-interest concentration (Ni, Pearson & Poteshman 2005 document pinning on optionable stocks at expiry); VWAP as an intraday anchor. **Caveat, stated plainly: real pinning is a dealer-hedging / open-interest effect. The synthetic data has no OI, no dealer flow, no gamma, so this backtest cannot see the mechanism. Any edge here is purely price-path based**: intraday vol persistence (quiet afternoon stays quiet) versus the VIX1D-based model price.

## Rules
- Instrument: SPY (QQQ via `symbols`), interval 1h (default; ~3 years, ~725 days). 5m tested separately (60 days only).
- Decision: once per day at the bar closing 90 min before 16:00 (the 13:30 bar on 1h; fills at next open, 14:30).
- Centre = session VWAP (typical-price x volume, today's bars) rounded to the $1 strike.
- Filters: (1) |spot - centre| <= 0.35 x remaining 1-sigma move (VIX1D-implied); (2) quiet: last-3-bar range <= 0.8 x 2 x implied sigma over the same span; (3) fly debit <= 0.35 x width.
- Structure: call butterfly (c-1, c, c+1), 1 unit = 4 contracts; sizing by engine (3% of equity max loss, commission included).
- Exit: none; held to expiry (settles at intrinsic, not a day trade, so no PDT slot used). Optional `STRUCT="iron"`: iron butterfly credit version.

## Parameters
`DECISION_MIN_TO_CLOSE=90, CENTER_MODE="vwap" ("round"|"mid"), ROUND_STEP=5, WIDTH=1, STRUCT="fly"|"iron", MAX_DIST_SIGMA=0.35, QUIET_RATIO=0.8, MAX_COST_FRAC=0.35, MIN_CREDIT_FRAC=0.55`.

## Results (SPY 1h, default)
```
s18_pin_butterfly: 2023-11-10→2026-10-09  final $5,531
  ALL  ret +176.5%  CAGR +41.8%  maxDD -23.1%  Sharpe 1.27  trades 265  win 44%  PF 1.26
  OOS (from 2025-11-24)  ret +1.1%  maxDD -23.1%  Sharpe 0.20  trades 93  win 39%  PF 1.01
  rejected entries 0  PDT-blocked exits 0
```
In-sample PF 1.52 (172 trades); OOS PF 1.01 (93 trades) = essentially flat out of sample. Full-sample return is inflated by compounding (position size scales with equity). Win rate 44% with PF>1 as expected for a lottery-shaped payoff; per-trade avg +$13 full sample.

## Variants tried (judged on in-sample; OOS shown for honesty, not used for selection)
| Variant | ALL PF | IS PF | OOS PF | n |
|---|---|---|---|---|
| V0 base: VWAP fly, 90m | 1.26 | 1.52 | 1.01 | 265 |
| V1 no filters (control) | 0.71 | 0.71 | - | 270 |
| V2 round-$5 centre | 0.95 | 1.02 | 0.79 | 248 |
| V3 range-midpoint centre | 1.19 | 1.43 | 0.91 | 235 |
| V4 iron butterfly | 1.23 | 1.47 | 1.00 | 274 |
| V5 decide 30m before close | 1.79 | 1.88 | 1.69 | 83 |
| V6 decide 150m before close | 1.44 | 1.46 | 1.43 | 339 |
| QQQ 1h base | 1.29 | 1.46 | 1.08 | 238 |
| SPY 5m base (60 days) | 1.42 | 2.11 | 0.59 | 29 |
| QQQ 5m base | 0.61 | 0.55 | 0.77 | 26 |
| SPY 5m, 60m | 2.04 | 2.60 | 0.85 | 25 |

Base kept: it matches the brief (last 1-2h), V0 IS was best among centre/structure choices. V5/V6 look better but V5 is tiny-sample and V6 is outside the "last 2h" frame; they are worth a forward look, not adoption. More than 6 variants were run (extra QQQ/5m cross-checks), noted for honesty.

Findings: the filters carry everything (unfiltered control loses; commission 4 legs x $0.65 = $2.60 per unit entry is a heavy drag on a $0.20 debit). Centring on the VWAP works better than centring on round numbers (round-number centre is ~breakeven, consistent with no dealer-pinning information in the data). 5m results are 20-30 trades: no conclusion.

## Risks / model-artifact concerns
- **Most likely a model artifact.** Fly price comes from Black-Scholes with VIX1D-based vol; if VIX1D systematically exceeds realized afternoon vol (it includes the full-day/event premium), a quiet-afternoon centred fly is "cheap-looking-expensive" relative to realized, and OOS flattening suggests fragility. Skew model mispricing near ATM is small but real.
- Fills: model spreads of $0.01-$0.05 per leg x 4 legs are tight; real butterflies on SPY 0DTE can be 5-15 cents wide on a $0.20 fly, which would erase most of the edge.
- Pin risk / early assignment are ignored; real pinning (gamma) is not modelled either way.
- Max DD ~23% sits at the top of the allowed range; per-trade total loss is the debit (capped ~$60).
- Sample: 1h only has ~3 years; 5m only 60 days.

## Paper-trading notes
Record real quoted fly ask vs the model price at the decision time; log the quote spread (the key unknown). Track whether afternoon realized range after the filter is really below VIX1D-implied. Compare VWAP centre vs the actual dealer-gamma/max-OI strike (visible in a live chain) as the real test of pinning. Do not trade size beyond the 3% cap; expect ~40-45% hit rate with lumpy wins.
