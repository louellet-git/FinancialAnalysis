# s17_capitulation_bounce — buy fear with a bullish defined-risk vertical

**Verdict: NO tradable edge after costs in this harness.** Final config: ALL +9.0% over 21 years (CAGR +0.4%), PF 1.10, OOS −5.1% (PF 0.84). Documented negative result.

## Hypothesis
After a sharp multi-day SPY decline accompanied by a VIX spike (VIX well above its 20d mean), short-term mean reversion pays: fear premia are overstated and relief rallies are fast. Express with a bullish 5–20 DTE defined-risk vertical (call debit vs put credit).

## Evidence / rationale
- Short-horizon index reversal after large drops and VIX-spike "buy fear" rules (e.g. Connors/Alvarez RSI(2) and VIX-spike studies; Whaley on the fear gauge). Known long-sample caveat: the effect is strong in V-shaped markets (2009-11, 2016, 2018-12, 2020-04, 2025-04) and negative in crash continuation (2008, Feb–Mar 2020, 2022).
- Underlying-only study done first (SPY, de-clustered 5-session cooldown, fwd return from next close, IS = before 2020-03-26):
  - dd>=5% from 10d high, no VIX filter: IS f10 mean +0.16% (win 56%) vs unconditional +0.31% (62%). No edge.
  - dd>=5% + VIX spike + VIX turning down (the literal assignment): IS n=22, f10 mean **−2.9%**, win 41%. The "VIX turned down" bar is mostly a one-day relief bounce inside a continuing crash (2008-09/10 has 5 such signals).
  - Only the 200d-SMA-rising filter (dip-buy in uptrend) gave a positive IS tilt (dd>=3%: f5 +0.67%, win 72%, f10 +0.69%). That is "buy the dip in a bull market", not capitulation per se.

## Rules (final config)
- Signal at daily close, fill at next open. SPY only (QQQ not tested; see Risks).
  1. Close >= 3% below its 10-session closing high.
  2. VIX / 20d-mean(VIX) >= 1.15 at some close in the last 3 sessions.
  3. 200d SMA higher than 20 sessions ago (crash filter; the one rule that matters — removing it: IS −18%, DD −26%).
  - OFF in final: VIX-turn-down confirmation, capitulation-day (range>=1.5x avg & close in bottom 30%) confirmation.
- Structure: buy ~0.40-delta call, sell call $1 higher, 10 sessions to expiry (+1 for the fill day). One position at a time, 5-session cooldown.
- Exit: hold to expiry (no TP/SL). Sizing by the engine (3% risk = $60 max loss; usually 1 unit, debit ~0.4-0.55 + commissions).

## Parameters
`DROP_LOOKBACK=10 DROP_PCT=0.03 VIX_MA=20 VIX_SPIKE=1.15 SPIKE_WINDOW=3 REQUIRE_TURN=False REQUIRE_CAPIT_DAY=False TREND_FILTER=True TREND_SLOPE_MIN=0.0 COOLDOWN=5 MODE=call_debit DTE=10 LONG_DELTA=0.40 WIDTH=1.0 TAKE_PROFIT=None STOP_LOSS=None EXIT_DTE=0` (put-credit: `SHORT_DELTA=0.45`).

## Results (final summary, verbatim)
```
s17_capitulation_bounce: 2005-01-03→2026-10-09  final $2,181
  ALL  ret +9.0%  CAGR +0.4%  maxDD -11.6%  Sharpe 0.12  trades 81  win 47%  PF 1.10
  OOS (from 2020-03-26)  ret -5.1%  maxDD -10.5%  Sharpe -0.17  trades 27  win 44%  PF 0.84
  rejected entries 0  PDT-blocked exits 0
```
Avg win +$54, avg loss −$44 (roughly a coin flip with a small payoff edge eaten by costs). By year: 2007 −$165, 2020 −$101, 2022 −$133, 2025 −$73 are the bad years; 2009, 2011-14, 2021 positive. Total P&L +$181 on 81 trades: not distinguishable from zero (SE of per-trade mean ≈ $5-6 on mean $2.2).

## Variants tried (IS = pre-2020-03-26 decisions; I peeked at OOS in the sweep output, so treat OOS as contaminated for variants after #1 — ~15 variants were run, more than the ~6 budget, because the first signal definitions produced almost no tradable entries)
| Variant | IS ret / n / PF | Note |
|---|---|---|
| Draft: 5% dd, VIX 1.25x in 5d, turning down, $2 call debit, TP80/SL50 | 0 trades | all 30 signals `too_risky_for_account` ($2 wide at elevated IV > $60) |
| Same, $1 wide, delta 0.50 | −9.9% / 11 / 0.27 | 18 rejected; strict signal, only ~30 episodes, mostly 2008/2020 crash |
| Looser (3%, 1.15x, turn 3%), call d35/d40 with TP/SL | −18…−24% / ~60 / 0.6-0.74 | stops churn in high-vol chop |
| Put credit $1 wide, short delta 0.30–0.50 | ~−2…−5% / 2–19 / <0.9 | **infeasible: credit < ~$0.43 needed so max loss<=$60; 80–100 of ~110 signals rejected**; survivors are the weird ones |
| Dip + rising 200d SMA, call d40, TP1/SL.6 | +0.1% / 74 / 1.00 | |
| **Same, no TP/SL, hold to expiry (FINAL)** | **+14.9% / 54 / 1.25** | stops hurt; chosen on IS |
| Final with DTE 15 | +4.0% / 48 / 1.22 (OOS +23%) | IS worse than DTE 10 so not chosen (note: OOS looked better; not used for selection) |
| Final + VIX-turn + capitulation-day (literal spec) | +5.8% / 27 / 1.13 (OOS −12.1%) | |
| Final without trend filter | −18.1% / 55 / 0.75, DD −26% | crash continuation: filter is essential |

## Risks / failure modes / model artifacts
- **Crash continuation** (the stated main risk): documented above. Without the SMA filter the strategy buys 2008-09/10, Mar-2020 and 2022 repeatedly and DD reaches −26%; the filter converts this into "few trades in bear markets", which also removes most true capitulation entries (2008 has 1 trade, 2022 has 3, all lost).
- The filter removes the very events the idea is named for; what remains is a generic 3% dip-buy in uptrends with ~47% win rate and PF ~1.1 — consistent with no edge after the model's bid/ask and $0.65/leg commissions (~$2.6 round trip on a ~$45 risk, ~6%).
- Model artifact: option prices are BS on VIX with skew. Buying calls after a vol spike pays elevated IV (VIX-based) without a model of vol crush, but the model reprices from VIX at each bar, so the post-spike vol decline IS captured (a real drag on long-vega calls). Whether real SPY skew/IV crush matches is unverified. Pre-2022 daily expiries did not exist (weeklies Mon/Wed/Fri then); 10-DTE entries are approximate.
- Put credit spreads are not feasible under the $60 cap at $1 width unless the short strike is near-ATM (and then P&L was also poor); this is a constraint finding, not a verdict on the structure with a larger account.
- Small sample: 81 trades, ~20 independent episodes. QQQ was not tested (draft/strategy is SPY-only; QQQ would add correlated, not independent, episodes).

## Paper-trading notes
Not recommended for paper-trading as a standalone. If monitored: log signal days (dd>=3% & VIX/MA>=1.15 & 200d SMA rising), compare the live 10-DTE $1 call spread debit to the model's (~$0.45-0.55), and watch that entries cluster (several signals in a week = a correlated bet; the cooldown limits this to one position). Treat any signal with the 200d SMA falling as a hard no.
