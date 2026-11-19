# SPY/QQQ options bots — leaderboard (2026-10-11)

20 research bots (one strategy family each) plus one orchestrator ablation (s14b), all
run on the same harness after the pricing fixes below. **Every option price is
MODEL-PRICED** (Black-Scholes on VIX/VXN/VIX1D + skew + spread); no free historical
option quotes exist. OOS = the last 30% of each bot's bars, never used for tuning by the
bot itself (several bots admit to peeking or to running more than the ~6 allowed variants;
see their notes). Reproduce: `python -m scripts.trade_backtest [--max-risk 250]`.

## Account A — $2,000, max loss 3% of equity per trade (~$60)

| Bot | Bars | All return | Max DD | Sharpe | Trades | OOS return | OOS Sharpe | OOS trades |
|---|---|---|---|---|---|---|---|---|
| **s14b_spy_trend_calls** | 1d 2005– | +358% | -28% | 0.63 | 516 | +65% | 0.66 | 160 |
| **s14_spy_qqq_rotation** | 1d 2005– | +399% | -29% | 0.60 | 540 | +63% | 0.60 | 162 |
| **s05_ic_0dte** | 1h 2023– | +84% | -11% | 1.19 | 99 | +40% | 1.69 | 56 |
| s18_pin_butterfly | 1h 2023– | +72% | -26% | 0.78 | 259 | -9% | -0.23 | 93 |
| s17_capitulation_bounce | 1d 2005– | +10% | -11% | 0.12 | 81 | -5% | -0.16 | 27 |
| all other 15 | | -2% to -69% | | < 0 | | ≤ +6% on ≤ 17 trades, or no trades | | |

## Account B — $2,000, max loss up to $250 per trade (12.5%, scales down with equity)

Only **s05_ic_0dte** improves: +357%, max DD -30%, OOS +60% (Sharpe 1.82, 56 trades).
Everything else gets worse — most bots were built around $1-wide spreads, so the bigger
budget just buys 3–4x the units and turns modest losers (and s14/s14b, which hold three
overlapping positions) into -80% to -95% wipeouts. **More risk per trade is not better
for these bots**; the $250 budget is only useful if a bot is redesigned to use it for
structures the $60 cap forbade (16-delta condors, wider credit spreads) at 1–2 units.

## Added after the first round: s21 Bollinger Band fade (user request)

Fades the first SPY close outside the 20-day, 2-sigma bands with 0.25-delta credit
spreads: a bear call spread above the upper band, a bull put spread below the lower
band, 10 sessions out, 50% profit target. **It needs the $250 budget** (at $60 no
credit spread fits). At $250: 206 trades, **91% winners**, profit factor 1.60, +84% over
2005–2026 (2.8%/yr), max drop −15%, in the market about 24% of days. Last 30%: 89%
winners, +11%. Average win ≈ $24 vs average loss ≈ $155, so one loss erases about six
wins. Buying put debit spreads at the upper band loses (33–39% winners): SPY doesn't
actually reverse after the band, it just rarely runs much further. Details:
`s21_bollinger_fade.md`.

## What held up, and the caveats

1. **Trend-filtered weekly SPY call debit spreads, held to expiry (s14b / s14).** The
   only long-history result (21 years, 500+ trades, consistent OOS). It is mostly equity
   beta with a 200-day-SMA filter. The SPY/QQQ rotation does not reliably add value: from
   2005 always-SPY ≈ rotation; from 2012 rotation is ahead; started mid-2001 the rotation
   is ruined (-93%) by the dot-com bust while always-SPY makes +461%. Prefer s14b.
   Holding to expiry matters — s08 (same idea, rolling early with profit targets) loses
   because round-trip friction is ~18% of a ~$50 debit.
2. **0DTE calm-day iron butterfly (s05).** Strongest Sharpe, but only ~3 years of hourly
   data, the top 10 trades are most of the P&L, ~14 variants were tried, and the edge
   leans on the volatility risk premium baked into IV = VIX1D. The pricing fix cut its
   backtest from +168% to +84%. Paper-trade it against the live chain before trusting it.
3. s18's in-sample edge vanished out of sample; the rest are documented negative results.
   Several real underlying effects (pre-FOMC drift pre-2015, RSI(2) pullbacks, overnight
   drift, turn-of-month) exist in SPY/QQQ but are too small to survive $1-wide option
   spreads after $0.65/leg commission and the bid/ask.

## Harness changes made after the bots' reports

- Variance clock in `pricing.year_fraction`: overnight decay (overnight ≈ 20% of daily
  variance) and a U-shaped intraday profile; a close-to-close day is exactly 1/252.
  Before, options did not decay overnight and post-morning 0DTE vol was overstated —
  both flattered some bots (s05 and s18 lost about half their backtest return).
- `RiskConfig.with_max_risk(dollars)` / `--max-risk` on both CLIs.
- `ctx.day_trades_left` (PDT budget visible to bots); faster `strike_for_delta`.
- Known limits still open: exits fill only at bar close; no price-level exit rule;
  $1 strike grid; early assignment, dividends and real expiry calendars before 2022
  are not modeled.
