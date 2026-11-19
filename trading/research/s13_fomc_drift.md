# s13_fomc_drift — pre-FOMC announcement drift via call debit spreads

**Verdict: NEGATIVE.** The underlying equity effect is real in the data (and decays after 2015, as published), but a $1-wide call debit spread on a $2,000 account does not monetise it after commissions, bid/ask and the payoff cap.

## Hypothesis
Lucca & Moench (2015, "The Pre-FOMC Announcement Drift", JF) find that US equities earn a large share of their annual excess return in the ~24h before scheduled FOMC statements (2:00pm ET). Later work (e.g. Kurov et al., Hu et al. 2022) finds the effect weakened/vanished after ~2015-16, so a decay test is part of the study.

## Evidence in this repo's data (SPY/QQQ daily, open of D-1 → close of D, D = scheduled statement day, 2005–2026, 173 meetings)
| | n | mean | hit | t |
|---|---|---|---|---|
| SPY all | 173 | +30.1bp | 60% | 2.69 |
| SPY pre-2015 | 80 | +59.5bp | 68% | 3.30 |
| SPY 2015+ | 93 | +4.8bp | 54% | 0.36 |
| QQQ all | 173 | +41.0bp | 63% | 3.22 |
| QQQ pre-2015 | 80 | +67.3bp | 66% | 3.81 |
| QQQ 2015+ | 93 | +18.4bp | 60% | 1.03 |

Baseline (any 2-day open→close) is +5.4bp SPY / +8.1bp QQQ, hit 55%. So: effect strong pre-2015, SPY effect statistically gone since (consistent with the literature's decay finding); QQQ retains a weak, insignificant residual.

## Rules
- Dates: hardcoded SCHEDULED statement days 2005–2026 (`FOMC_DATES`), published a year ahead, so no lookahead. Excluded: unscheduled 2008-01-22, 2008-10-08, 2020-03-03, 2020-03-15 and the cancelled 2020-03-18 meeting. All dates verified present in the SPY/QQQ daily index.
- Entry: decide at close of D-2 (D = `ctx.expiry(2)`), fills at D-1 open. Buy SPY call debit spread, long strike at ~0.50 delta, $1 wide, expiry = D+1 session (so no 0DTE at exit).
- Exit: `ctx.close` on the D close bar (held overnight → not a day trade; PDT blocks = 0). No stop/target (the hold is only ~1.3 sessions).
- Sizing: engine's (max loss ≤ 3% of equity; 1 unit normally).

## Parameters
`DELTA=0.50, WIDTH=1.0, EXTRA_DTE=1, MIN_YEAR=2005, SKIP_VIX_ABOVE=1.0 (off)`; SPY; interval 1d; start 2005-01-01.

## Results (final config)
```
s13_fomc_drift: 2005-01-03→2026-10-09  final $1,739
  ALL  ret -13.1%  CAGR -0.6%  maxDD -19.9%  Sharpe -0.16  trades 116  win 47%  PF 0.84
  OOS (from 2020-03-26)  ret -9.4%  maxDD -9.4%  Sharpe -0.51  trades 23  win 39%  PF 0.54
  rejected entries 57  PDT-blocked exits 0
  rejections: {'too_risky_for_account': 57}
```
Sample: 116 trades (57 of 173 meetings rejected as too risky — the debit plus commission exceeded 3% of a shrinking equity, or high-vol debits exceeded $60); OOS only 23 trades. Treat all statistics as low-power.

## Variants tried (7 runs; all ≈ negative, so the selection is not meaningful)
| Variant | ALL ret | ALL Sharpe | OOS Sharpe |
|---|---|---|---|
| **Base: Δ0.50, exp D+1 (final)** | -13.1% | -0.16 | -0.51 |
| Δ0.40 | -33.2% | -0.41 | -0.39 |
| Δ0.30 | -50.2% | -0.60 | -0.26 |
| Δ0.40, exp D (0-1DTE at exit) | -26.0% | -0.16 | -0.07 |
| Δ0.40, only 2015+ meetings | -27.1% | -0.53 | -0.56 |
| Δ0.40, skip VIX>30 | -37.0% | -0.52 | -0.39 |
| QQQ, Δ0.50, exp D+1 | -19.2% | -0.23 | -0.16 |

Final = base because it had the best ALL/in-sample result and highest delta (cheapest relative to its theta); not chosen on OOS.

## Why it fails
1. Payoff cap: a $1 spread on SPY (≈$120 in 2005, ≈$780 now) caps profit at ≈0.1–0.8% of spot; the drift mean is ~30bp pre-2015 and ~5bp after, so the spread captures a fraction of an already small edge.
2. Costs: 2 legs × 2 sides × $0.65 = $2.60 per round trip on ~$50 of risk (~5%) plus model bid/ask on both legs; the per-trade expected gross profit post-2015 is ≈ zero, pre-2015 only a few dollars.
3. Win rate ~47% with symmetrical-ish theta/gamma loss on flat days; the effect is a small shift in mean, not a high-probability directional move.
4. Post-2015 decay makes the recent (and OOS) years essentially a coin flip with negative carry.

## Risks / model-artifact concerns
- Option prices are model-priced; this strategy is NOT dependent on a vol mispricing, but the IV for a 1–2 day option comes from VIX (30d) without an event premium. Real FOMC-day IV is bid up (the event is priced in), so real fills would likely be WORSE than modelled — negative result is, if anything, optimistic.
- Pre-2016 daily expirations didn't exist; 2005–2015 results use synthetic daily expiries (approximate; real weeklies/monthlies give different strikes/liquidity).
- Open-to-close daily bars approximate the 24h window with extra overnight/post-announcement noise (post-announcement drift is mixed/negative).
- 2020 has only 7 scheduled dates kept (March meetings were unscheduled/cancelled).

## Paper-trading notes
Not recommended for live paper deployment as an options strategy. If one wanted to monitor the effect: log SPY return from the open of D-1 to the 2:00pm print vs. the baseline; a revival of the drift (> +20bp mean over ≥ 16 meetings) would be the trigger to retest, ideally with a long ATM call instead of a spread only if capital permits (not feasible at $2k: ATM 2-day SPY call ≈ $400+ per contract).
Upcoming scheduled dates in the code: 2026-10-28, 2026-12-09.
