# s05_ic_0dte - regime-gated 0DTE iron butterfly (SPY, 1h bars)

## Hypothesis
Selling same-day premium (variance risk premium, VRP) is profitable on average, but the
losses cluster on trend / event days. If entry is restricted to calm intraday regimes -
low VIX1D relative to VIX, small realized morning move, non-trend prior day - the short
ATM iron fly (a bet on "SPY ends the day near where it is at 10:30") should have a better
payoff than ungated selling.

## Evidence / rationale
- VRP: implied vol exceeds subsequent realized vol on average (Carr-Wu 2009; Bondarenko
  2014). Practitioner 0DTE studies (CBOE 2023-24 0DTE research; Bandi et al.) find the
  0DTE premium is much thinner than the 30-day VRP and that 0DTE sellers earn ~0 after costs
  at the index level. Do not assume the textbook VRP survives for 0DTE.
- Regime gating: VIX1D/VIX << 1 means the market expects a quiet day vs the month; a
  small morning move vs implied suggests no trend day; large prior-day ranges
  cluster (vol clustering).
- $60 risk cap forces narrow wings. A $2-wide iron fly with ~1.5 credit has max loss ~0.5/share
  (~$50 + $5.20 commissions per unit), i.e. 1 unit fits; the payoff is "lose ~$50, win up to ~$150".
  A $1-wide fly has a near-ATM credit ~0.83 / max loss ~0.17 and was ~breakeven (PF 1.00).

## Rules
- Universe/data: SPY, interval 1h (data begins 2023-11, so a ~2.9 year sample, 154 trades).
- Entry decision at the close of the first hourly bar (10:30 ET); filled at the next bar's open.
  One position at a time, 0DTE expiry, only if all gates pass:
  1. VIX1D / VIX <= 0.80 and VIX1D <= 20% (calm 1-day regime).
  2. |spot - day open| <= 0.6 x (expected move implied by VIX1D for elapsed time) (no gap/trend day).
  3. Prior-day high-low range <= 1.5 x (2 x 1-day sigma implied by VIX1D) (not a trend/expansion day).
  4. Net credit after spread >= $1.40 (else the structure cannot fit in the $60 budget).
- Structure: iron butterfly, short straddle at ATM strike (rounded to $1), long wings +/- $2.
- Sizing: engine sizes to max loss <= 3% of *current* equity (1 unit most of the time; up
  to 3-4 as equity grew - returns are partly compounding).
- Exits: take_profit 50% of credit, stop_loss 1.0x credit (never binds: max loss is ~0.3x
  credit, the wings are the stop), otherwise hold to expiry. No time exit on purpose:
  expiry settlement does not consume a PDT day trade, early exits do (3 per 5 sessions).

## Parameters
ENTRY_BAR=1, WING=2.0, BODY_OFFSET=0 (fly), MIN_CREDIT=1.40, MAX_RATIO=0.80, MAX_SHORT_VOL=0.20,
MAX_MORNING_MOVE=0.60, MAX_PRIOR_RANGE=1.5, TAKE_PROFIT=0.50, STOP_LOSS=1.00.

## Results (final, `python -m scripts.trade_backtest s05_ic_0dte -v`)
```
s05_ic_0dte: 2023-11-10→2026-10-09  final $5,368
  ALL  ret +168.4%  CAGR +40.3%  maxDD -15.5%  Sharpe 1.50  trades 154  win 42%  PF 1.64
  OOS (from 2025-11-24)  ret +87.1%  maxDD -15.5%  Sharpe 2.39  trades 68  win 44%  PF 1.85
  rejected entries 9  PDT-blocked exits 0
  rejections: {'too_risky_for_account': 9}
```
Per year P&L: 2024 +$79 (36 trades), 2025 +$1,134 (56), 2026 +$2,155 (62). Avg win +$133,
avg loss -$59, median trade -$23. The ten best trades make up 86% of total P&L: it is a
right-tail lottery ticket, not a smooth carry. Almost all exits are at expiry.
Sample caveat: 154 trades, ~3 years, one volatility regime; the 2024 result is near zero.

## Variants tried (in-sample used for choice; OOS shown for transparency, not for selection)
| variant | n | IS ret / PF | OOS ret / PF |
|---|---|---|---|
| $1 fly (credit 0.83) | 249 | -4.8% / 0.98 | +5.5% / 1.04 |
| $1 wings, short strikes +/-$1 (condor) | 233 | +1.2% / 1.01 | +64% / 1.72 |
| **$2 fly (final)** | 154 | +43% / 1.37 | +87% / 1.85 |
| $2 fly, TP 35% | 152 | +43% / 1.40 | +87% / 1.84 |
| $2 fly, entry 11:30 | 136 | +42% / 1.42 | +24% / 1.30 |
| $2 fly, stop 0.25x / 0.15x credit | 154 | +42% / 1.36 ; +35% / 1.32 | +83% ; +31% |
| $2 fly, looser gates (ratio<=1.0, vol<=25%) | 175 | +43% / 1.31 | +76% / 1.65 |
| $2 fly, NO gates | 180 | -48% (ruin) / 0.81 | none (account blown) |
| $2 fly, VIX1D/VIX gate only | 421 | +33% / 1.10 | +14% / 1.06 |
Chosen on in-sample $: the $2 fly (best IS PF and drawdown). Roughly 14 runs in total, more than
the suggested ~6 (several were ablations), so the headline numbers carry some selection bias.
Tight stops lose money (they convert pin-lottery losers into realized losers and burn PDT
slots); the take-profit is nearly irrelevant.

## How much of the edge is the baked-in VRP? (model-artifact analysis)
The pricer uses VIX1D as the ATM IV for the remaining session, then the backtest settles on
real SPY prints. So every sold option is rich by (implied - realized) by construction.
Measured on this data (721 days, hourly returns after 10:30): RMS realized vol / VIX1D = 0.83
(median ratio 0.66), i.e. variance ratio ~0.69. That ratio is a *much bigger* VRP than real
0DTE markets show, for two modelling reasons: (a) VIX1D covers overnight+full day, but the
pricer applies it to the remaining 5.5 hours at uniform variance, while real intraday variance
is front-loaded at the open (U-shape), so the post-10:30 session is cheaper than the model
thinks; (b) generic over-estimation of implied vol vs realized.
I re-ran the strategy with a pricer that scales all vols by k (fair-value pricing, settlement still real):

| k (vol scaling) | $2 fly: n / ret / PF (IS, OOS) | $1 condor: ret / PF (IS, OOS) |
|---|---|---|
| 1.00 (as modelled) | 154 / +168% / 1.64 (1.37, 1.85) | +66% / 1.27 (1.01, 1.72) |
| 0.92 | 128 / +115% / 1.53 (1.56, 1.52) | +11% / 1.06 (0.85, 1.47) |
| 0.83 (zero VRP, fair) | 95 / +80% / 1.56 (1.56, 1.55) | +0.3% / 1.00 (0.86, 1.21) |

Reading: the $1 condor edge is essentially all VRP (gone at k=0.83). The $2 fly keeps a positive edge
at fair prices, but (i) the sample shrinks to 95 trades because fewer credits clear the threshold,
and (ii) a short ATM fly is a short-pin/capped-loss structure whose payoff exploits that real
returns are peaked (median |move| well below RMS) while the normal model prices off RMS vol; plus the
gates select calm days using real information. I would only count on a fraction of the headline
(a reasonable prior: half or less of +40% CAGR, and treat it as unproven until forward tested).
The gating, not the VRP, is clearly doing real work: ungated is -48% with the same pricer.

## Risks / failure modes
- Model: no real intraday smile/skew for 0DTE, no real spreads (wide near the close and on fast days;
  model uses <= 5c half-spread), fills at bar open mid-quoted +/- spread; real fly fills at $2 wide will slip.
- Pin/gamma risk in the last hour: the fly can swing from +$100 to -$50 in minutes; held to expiry
  (no early exit, because of PDT). American/early assignment is ignored (short ITM leg of a fly near
  the close can be assigned; wings protect risk but not the margin/BP surprise).
- Tail days (FOMC, CPI, gap) are only partly filtered by gates; no event calendar in the strategy.
- Heavy right-tail dependence (top 10 trades = 86% of P&L) and median trade is a loss: long losing
  streaks (~6+) are normal; psychology/size must hold.
- Sizing scales with equity (3% of equity) so later returns compound (units grew from 1 to up to 4); a flat-size
  run was not measured.
- Only one regime: 2023-11 to 2026-10 (mostly low-vol, rising market).

## Paper-trading notes
- Check the actual 10:30 fly quote: is credit >= ~$1.40 on a $2-wide SPY fly? If real credits are
  ~$1.20, the strategy loses its edge (every 10c of credit is 20% of the max loss).
- Log VIX1D / VIX and the morning-move gate each day, and whether a skipped day would have won.
- Track real fill slippage vs the model mid (expect 3-8 cents per leg combo), and real after-hours
  pin outcome. Watch for PDT counts (no early exits means none used).
- QQQ not tested; SPY only. 5m not tested (60 days is too little).
