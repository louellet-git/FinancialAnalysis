# s21 — Bollinger Band fade (orchestrator-built, 2026-10-11, user request)

**Question:** when SPY/QQQ closes above its upper Bollinger Band (20-day, 2 sigma), does
a bearish trade (put debit spread or bear call spread) win consistently while spending
little time in the market? What does the win rate look like?

## What the underlying does (SPY/QQQ daily, 2005–Oct 2026, first close outside a band)

| After first close… | Horizon | SPY lower after | SPY mean move | All days lower |
|---|---|---|---|---|
| above upper band (n=143) | 5 days | 44% | −0.07% | 41% |
| above upper band | 10 days | 42% | +0.03% | 38% |
| below lower band (n=158) | 10 days | 36% (64% higher) | +0.81% | 38% |

- **No real reversal after the upper band.** SPY is about a coin flip and QQQ keeps
  rising (momentum: lower only 36% of the time over 5 days). Buying puts has nothing
  to work with.
- **But SPY rarely runs much further.** The median 10-day maximum rally after the cross
  is +0.9%, and only 18% of signals rallied more than 2%. That suits *selling* a call
  spread above the market. For QQQ the figure is 43%, so the edge is weaker there.
- The lower band is the stronger side: SPY is higher 61–64% of the time 5–10 days later.

## Rules (final)

- Signal: the first close outside BB(20, 2) on SPY. Entry at the next open, at most one
  position per side.
- Upper band: **bear call spread**, short call at ~0.25 delta, 10 sessions out.
- Lower band: **bull put spread**, short put at ~0.25 delta, 10 sessions out.
- Width: the widest of $5/$4/$3/$2/$1 whose worst case fits the risk budget, 1 contract.
  Spreads paying under 15% of their width are skipped.
- Exit: take profit at 50% of the credit, otherwise hold to expiry.

## Results (model-priced options, $2,000 account)

| Variant | Risk/trade | Trades | Win rate | Profit factor | Return | Max drop |
|---|---|---|---|---|---|---|
| **Final (SPY, both sides)** | $250 | 206 | **91%** | 1.60 | +84% (2.8%/yr) | −15% |
| Final, last 30% only (2020-03 →) | $250 | 66 | 89% | 1.29 | +11% | −9% |
| Any credit spread | $60 | 0 | — | — | cannot fit the cap | |
| Put debit spread, SPY upper band | $250 | 88 | 33% | 0.62 | −82% | −84% |
| Put debit spread, SPY upper band | $60 | 80 | 39% | 0.69 | −23% | −30% |
| Bear call only, SPY | $250 | 99 | 89% | 1.52 | +40% | −17% |
| Bull put only, SPY | $250 | 116 | 83% | 1.18 | +15% | −20% |
| Bear call only, QQQ | $250 | 67 | 79% | 1.16 | +11% | −15% |
| Bull put only, QQQ | $250 | 72 | 75% | 0.70 | −23% | −30% |

**The win-rate trade-off.** About 9 of 10 trades win, but the average win is about $24
and the average loss about $155. One loss wipes out roughly six wins. Upper-band side:
92% wins, average +$27 / −$169, worst −$236. Lower-band side: 91% wins, average
+$21 / −$144, worst −$176. 164 of 206 trades closed at the 50% profit target.

**Time in market:** about 24% of trading days, roughly 9 trades a year, median hold
7 days.

**Losing years:** 2008 (−$211), 2015, 2017 and 2026 to date (−$250). The worst stretches
are sharp rallies (2017, 2026) and crashes that keep going (2008), the usual short-option
failure modes.

## Tuning (in-sample only; out of sample shown for the record)

Six variants of the combined SPY bot, chosen on the first 70% of bars only.

| Variant | In-sample PF / win | Out-of-sample PF / win |
|---|---|---|
| Base: 0.25 delta, 10 sessions, TP 50%, 2x-credit stop | 1.63 / 88% | 1.02 / 81% |
| Short delta 0.20 | 2.44 / 90%, only 29 trades | 0.91 / 81% |
| Short delta 0.30 | 1.64 / 87% | 1.36 / 83% |
| **No stop (chosen)** | **1.84 / 92%** | **1.29 / 89%** |
| 5 sessions | 1.37 / 86% | 2.50 / 85% (not chosen: looked only at in-sample) |
| 15 sessions | 1.47 / 86% | 1.18 / 85% |

## Caveats

- Option prices are model estimates. The edge in selling 0.25-delta spreads leans on
  the volatility risk premium baked into IV = VIX. Real fills on 10-session SPY spreads
  are usually tight, but check them in paper trading.
- It needs the $250 risk budget. At $60 no credit spread fits, and debit spreads lose.
- High-win-rate, negative-skew strategy: a run of 2–3 losses in a row costs months of
  gains. Size accordingly.
- Early assignment on a short call before ex-dividend is not modeled. SPY pays
  quarterly dividends, so watch short calls near ex-dates.

Reproduce: `python -m scripts.trade_backtest s21_bollinger_fade --max-risk 250 -v`
