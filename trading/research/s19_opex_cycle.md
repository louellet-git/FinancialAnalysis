# s19_opex_cycle - monthly options-expiration cycle (NEGATIVE RESULT)

## Hypothesis
Monthly OPEX (third Friday; Mar/Jun/Sep/Dec = quad witching) has a calendar footprint: the OPEX week is
bullish (dealer long-gamma/pinning, vanna/charm tailwind into expiry), the week after OPEX is weaker (those
supports roll off; Ni-Pearson-Poteshman pinning, Stivers-Sun / practitioner "OPEX week" lore).

## Evidence (own calendar study, SPY/QQQ daily closes 2005-2026, ~87 monthly cycles each)
Third Fridays computed from the calendar. SPY: all-week mean +0.20%/wk, win 57%.
- OPEX week (prior Fri close -> OPEX Fri close): mean -0.02%, win 54% (SPY); +0.15% (QQQ). No edge; below the average week.
  Quad months SPY -0.25% (OOS-era 2019+: -0.70%), QQQ +0.35%. Sign differs by symbol -> noise.
- Week AFTER OPEX: +0.25% (SPY) / +0.39% (QQQ) = roughly the unconditional average, NOT weak. Only after
  quad-witching (Mar/Jun/Sep/Dec) is it ~0.0% (win 41%), vs +0.36% (win 62%) after non-quad months. Within quad
  months this is Jun/Sep-driven (IS 2005-18: mean -1.0%, n=28) and mostly vanished in 2019+ (-0.1%, n=16);
  March and December show nothing. Statistically weak (n~20/month bucket), unstable by period.
So the folklore ("OPEX week bullish") is not in the data; the only visible pattern (post-quad weakness) is thin and decays.

## Rules (as shipped)
Decision on the last session of each week, fill at Monday open, SPY only, $1-wide 0.45-delta debit vertical, expiry = that Friday
(Thursday if holiday).
- OPEX week (quad and non-quad): bullish call debit spread.
- Week after quad-witching: bearish put debit spread. Week after non-quad OPEX: skip. All other weeks: flat.
- Exits: take profit +80% of debit, stop -50%, exit_dte=1. One position at a time; engine sizing under the 3% rule.

## Parameters
SIDE_OPEX_QUAD=1, SIDE_OPEX_NON=1, SIDE_AFTER_QUAD=-1, SIDE_AFTER_NON=0, DELTA=0.45, WIDTH=1.0, TAKE_PROFIT=0.8, STOP_LOSS=0.5, EXIT_DTE=1. start=2005-01-01.

## Results (final variant, V1)
```
s19_opex_cycle: 2005-01-03->2026-10-09  final $1,263
  ALL  ret -36.8%  CAGR -2.1%  maxDD -38.8%  Sharpe -0.40  trades 194  win 40%  PF 0.75
  OOS (from 2020-03-26)  ret -16.4%  maxDD -16.6%  Sharpe -0.82  trades 24  win 21%  PF 0.44
  rejected entries 148  PDT-blocked exits 0
  rejections: {'too_risky_for_account': 148}
```
Rejections are mostly after the account has bled below ~$1.5k (3% budget < cost of one spread + $2.60 round-trip
commission); i.e. rejections are a symptom of the losing equity curve, not an independent filter.
Sample caveat: ~1 trade/month, ~24 OOS trades - anything here is statistically meaningless either way.

## Variants tried (7, all-sample / OOS; chosen on the hypothesis, not on OOS)
| Variant | ALL ret | PF | OOS ret | OOS n |
|---|---|---|---|---|
| V1 as shipped (delta .45) | -36.8% | 0.75 | -16.4% | 24 |
| V2 delta .30 | -64.2% | 0.56 | -11.6% | 19 |
| V3 OPEX-week bull only | -52.5% | 0.56 | +4.5% | 22 |
| V4 post-quad bear only | -51.2% | 0.37 | -25.4% | 20 |
| V5 post-non-quad bull only | -60.1% | 0.26 | -7.9% | 12 |
| V6 post-OPEX week all bull, hold to expiry | -63.5% | 0.36 | -5.8% | 14 |
| V7 delta .40, OPEX+post-nonquad bull, no TP/SL | -50.6% | 0.71 | +4.0% | 7 |
V1 kept because it is the pre-stated hypothesis and the least bad on the full sample. The two small positive OOS numbers
(V3/V7, n=22/7) sit on deeply negative full-sample results - not evidence of an edge; not adopted.
Conclusion: the OPEX-cycle calendar does not produce a tradeable edge after spread + commission on $1-wide spreads.

## Risks / model-artifact concerns
- Pre-2022 weekly-expiry assumption is approximate; pre-2015 SPY options were less liquid than modeled.
- Costs dominate: modeled bid/ask ~$0.04-0.05 round trip plus $2.60 commission on a ~$30-50 debit (~8-10% friction per trade).
- Directional debit spreads are low-win-rate (OOS 21%); the underlying calendar signal is far too small (+/-0.2%/wk) to overcome it.
- The model-priced results are not driven by implied-vs-realized artifacts (pure directional debit), so the negative result is plausibly genuine.

## Paper-trading notes
Not recommended for paper-trading as a standalone strategy. If monitored: compare realized SPY return in OPEX weeks and post-quad weeks
against the unconditional weekly mean; skip unless post-quad weakness (Jun/Sep) re-appears with >=30 more observations.
