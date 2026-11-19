# s11_vrp_condor — 30-45 DTE iron condor, volatility-risk-premium harvest

**Verdict: NEGATIVE under the $2,000 / $60-cap / $0.65-per-leg constraints.** The VRP is real
in the data (and in the model), but the account's fixed costs and the risk cap eat it. The
strategy loses money after costs and then stops trading itself (see "Death spiral").

## Hypothesis
Index options are priced at an implied vol that, on average, exceeds subsequently realized vol
(the variance risk premium; Carr & Wu 2009, Bakshi & Kapadia 2003, Coval & Shumway 2001,
CBOE PUT/CNDR index studies). Selling premium with defined risk (iron condor) at ~30-45 DTE,
managing at 50% profit / 21 DTE (tastytrade-style), should harvest it.

## Rules
- Underlying SPY, daily bars, 2005-01-03 onward (data to 2026-10). One position at a time.
- Decide on Mondays (fills at Tuesday open): expiry 28 sessions after the fill session (~41 calendar days).
- Structure: $1-wing iron condor; short put / short call at SHORT_DELTA (|delta| via synthetic quote).
- Entry skipped if the engine would reject it (`(wing - credit)*100 + $5.20 fees > 3% of equity`).
- Exits (ExitRules): take profit at 50% of credit, stop at 1.0x credit loss, `exit_dte=15` sessions (~21 calendar days).
- Sizing: engine's (1 unit essentially always: max loss ~$55-60 vs $60 budget).

## Key structural finding: 16-delta does not fit the cap
Measured entry credit of a $1-wide condor (SPY and QQQ, 28 sessions, synthetic pricer, after the
bid/ask haircut), and resulting max loss including $5.20 round-trip... (entry+exit fees = 8 legs x 0.65):

| short delta | credit/share | max loss $ |
|---|---|---|
| 0.16 | 0.22-0.24 | ~81-83 |
| 0.20 | 0.28-0.31 | ~75-78 |
| 0.25 | 0.35-0.40 | ~66-70 |
| 0.30 | 0.42-0.47 | ~58-63 |
| 0.35 | 0.50-0.56 | ~49-56 |

Max loss must be <= 3% of equity = $60 (and equity only shrinks when the strategy loses). The spec'd
16-delta version was run as written: **1,025 of 1,025 entries rejected `too_risky_for_account`, zero trades.**
To trade at all, shorts must be >= ~0.30 delta, which is a different (much tighter, ~1.0-1.2 sigma) animal
than the "16-delta" condor: a ~35-delta short with a $1 wing is nearly a short strangle with a hedge.
Default SHORT_DELTA = 0.32.

## Results (final config, SPY, 2005-2026, `python -m scripts.trade_backtest s11_vrp_condor`)
```
s11_vrp_condor: 2005-01-03→2026-10-09  final $1,587
  ALL  ret -20.7%  CAGR -1.1%  maxDD -21.2%  Sharpe -0.92  trades 40  win 22%  PF 0.06
  OOS (from 2020-03-26)  ret +0.0%  maxDD 0.0%  Sharpe 0.00  trades 0  win 0%  PF 0.00
  rejected entries 3  PDT-blocked exits 0
  rejections: {'too_risky_for_account': 3}
```
All 40 exits were `dte_exit` (0 take-profit, 0 stop-loss): with ~32-delta shorts and ~20 days of holding,
the 50% target is essentially never reached before 21 DTE, and the stop is rarely reached before then.

### Death spiral (why only 40 trades, zero OOS trades)
Each losing trade lowers equity; the cap is 3% of equity, so once equity < ~$1,900-2,000 the $1-wing condor
no longer fits ($55-60 max loss + fees) and the engine rejects every entry. The account stalls at ~$1,590
forever (2005-2010 trades, then sporadic). OOS 0.00 is "no trades", NOT "flat edge". Treat the 40-trade
number as a cost-impaired, stranded sample.

### Cost decomposition (same rules, SPY, full sample; "unconstrained" = starts at $6,000 so sizing never stalls the run)
| run | final / ret | trades | win | PF | maxDD | notes |
|---|---|---|---|---|---|---|
| actual ($2k, spread + $0.65/leg) | $1,587 / -20.7% | 40 | 22% | 0.06 | -21% | stalls |
| commissions off only | -20.4% | 70 | 39% | 0.26 | -21% | still stalls |
| spreads off only (half-spread -> 0) | -19.0% | 353 | 56% | 0.79 | -27.5% | OOS +7.5%, Sharpe 0.34 |
| **both off**, $2k | +98.3% | 367 | 67% | 2.04 | -4.6% | OOS +31.7%, Sharpe 1.25 |
| both off, $6k | +153.9% | 367 | 67% | 2.13 | -4.9% | avg gross pnl $5.4 per $60-risk unit |

Gross (frictionless) the model-priced strategy earns ~$5.4 per 1-unit trade (~$25 at ~6 units). Real frictions
per trade: commissions 8 legs x $0.65 = $5.20 round trip PLUS up to ~4 x $0.01 x 2 x 100 = $8 of minimum half-spread
(the pricer's floor is $0.01/leg, so $1-wide, cheap options pay the floor, a large proportion of a $0.40 credit). About
$13-18 of friction versus ~$5 of gross edge. The frictionless result is "all of it VRP" and none is tradable at this size.

## Realized vs implied (the model-artifact question)
The synthetic pricer sets ATM IV = VIX (SPY) / VXN (QQQ) directly (interpolated with VIX1D only post-2023), then adds
a fixed skew/smile. Because VIX historically exceeds realized vol, selling premium priced off VIX shows a built-in
positive expectancy. Measured on the data in this repo (index at close vs the realized vol of the NEXT 28 sessions,
close-to-close annualized, 2005-01 to 2026-10):

| | n days | mean VIX/VXN | mean fwd RV | gap (vol pts) | % days IV>RV | RV/IV |
|---|---|---|---|---|---|---|
| SPY / VIX | 5,449 | 19.13 | 15.76 | **+3.37** | 83% | 0.82 |
| QQQ / VXN | 5,449 | 22.16 | 19.16 | **+3.00** | 79% | 0.86 |

First-70% / last-30% gap (SPY): +2.87 / +4.53. The gap was negative only in 2008 (-2.5) for SPY and 2008, 2018, 2022
(~0) for QQQ, with a worst day-level gap of -67 vol pts (Oct 2008 / Mar 2020 type crashes; 4% of days < -10 pts).
This matches the published literature (roughly 3-4 vol points), so the premium itself is not invented.

What IS model-assumed and biases the backtest in the strategy's favor:
1. Every priced option uses VIX as its 30-day ATM vol. Real SPX VIX is a variance-swap-style number that already
   includes the skew; true ATM IV is typically 1-2 pts below VIX. The pricer then adds a skew on top, slightly
   double-counting skew premium on the put side.
2. A flat 1.0 multiplier on VIX for all expiries / strikes (a `0.5%-of-mid` tight spread, floor $0.01). Real 40-DTE
   SPY condor fills are mid-minus-a-few-cents per leg, not mid +/- $0.01; real slippage on 4 legs is probably worse.
3. Daily expirations are assumed (non-issue at 40 DTE).

Sensitivity (frictionless, $6k start so the cap never binds, same rules; scaling the pricer's whole IV surface):

| pricer IV vs VIX | ret | PF | Sharpe | avg pnl / trade |
|---|---|---|---|---|
| 1.00x (as-built) | +153.9% | 2.13 | 1.20 | $25.2 |
| 0.93x (~VIX minus 1.3 pts) | +95.0% | 1.69 | 0.84 | $15.5 |
| 0.82x (IV = avg realized: no VRP) | +23.2% | 1.16 | 0.25 | $3.8 |

Shaving 1.3 vol pts of "IV" cuts per-trade P&L by ~40%; removing the premium entirely leaves a ~1% CAGR residual
(the remaining edge is just short-gamma carry in a drifting-up market). So the as-built edge is largely, though not
entirely, the pricer's IV = VIX assumption combined with the empirical VIX > RV gap, and it is MOST fragile in
exactly the way that matters: the profit is $5/trade against a $13-18 friction bill.

## Variants tried (<= 6; all tuned only on full-sample/IS behaviour, none rescued the strategy)
1. 16-delta shorts (spec): 0 trades, all rejected (cap).
2. 32-delta, TP 50% / stop 1x / 21 DTE (FINAL; also the spec exits): -20.7%, stalls.
3. + only trade when VIX >= 20: -18.5%, 41 trades, PF 0.13 (stalls identically; higher vol does not help at the cap).
4. QQQ instead of SPY: -20.1%, 39 trades, PF 0.08.
5. No TP/stop (TP=5x, stop=5x), exit at 3 sessions to expiry (hold to ~expiry): -17.0%, 107 trades, PF 0.79 (fewer,
   longer trades dilute fees, but expiry gamma on a 32-delta/$1-wing condor loses more than the premium collected).
6. Decomposition runs above (fees/spread/IV scaling) — diagnostics, not tuning.
FINAL = variant 2 because it follows the assignment's exit rules at the minimum feasible delta; nothing improved it.

## Risks / failure modes
- Cap makes the position sizing binary: 1 unit at ~$55-60 risk, or nothing. A drawdown below ~$1,900 disables the
  strategy permanently (as in the backtest). A real account would also be PDT-irrelevant (multi-week holds).
- Gap risk through the short strikes: the $1 wing makes max loss ~= credit-adjusted $55, so the tails are capped, but a
  ~32-delta short breaches in roughly one of three cycles; win rate is low (22% after fees) rather than the 70-80% of 16-delta condors.
- Assignment/early-exercise on short ITM legs near ex-dividend (ignored; European model).
- Skew model: put wing priced from a fixed smile; actual crash skew for 28-session 32-delta strikes can be much steeper.
- The pricer's Oct-2008 / Mar-2020 VIX/RV gap (-60 pts) shows IV is not always above RV.

## Paper-trading notes
- Do NOT paper-trade the 16-delta/$1-wing version at this size; the cap rejects it. If practiced, use QQQ/SPY with real quotes
  and record the actual bid/ask on each leg: the only question is whether real round-trip friction on a 4-leg $1-wide
  condor is below ~$6 (it will not be at $0.65/leg: that is $5.20 on its own).
- A larger account ($5-10k+) is the real fix: the same trade at 16 delta with $2-5 wings has far better credit/fee ratio
  (fees fixed, credit scales with width).
- Watch: credit received vs the model's mid, fill slippage, stop frequency, and whether net P&L per trade exceeds friction.

## Harness notes
- `scripts/trade_backtest.py` / `trading/` were not present in this worktree's base commit (branch `claude/trading-bot-spy-qqq-xl1o2o`); copied locally from that branch to run (not part of the deliverable).
- `ctx.strike_for_delta` costs ~2 ms of 40 quote calls; calling it every bar for a 20-year daily run takes ~60 s. The bot uses its own 14-step bisect and a Monday gate (~7-20 s per run).
