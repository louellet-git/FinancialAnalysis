# Chain check of IBM and JNJ, 2026-11-19

Ticket: `PLANNING/board/T-20261120-arb-1-chain-check.md`.
This file gives the result of the first no-arbitrage check of stored option chains.
A model price is not a market price. A quote break is not a profit.

## Question

Do the stored Cboe chains of IBM and JNJ break a no-arbitrage rule at executable prices?
A break at mid prices only is not a finding.

## Command

```
rust/target/release/fa options check IBM --date 2026-11-19
rust/target/release/fa options check JNJ --date 2026-11-19
```

The command reads `data/raw/<T>/options/2026-11-19.json`. Without `--rate`, the rate is the Treasury par curve of the date (4.10 % to 4.81 %) and it can be downloaded (one file). Give `--rate R` for no network.

## Checks

- The checks (intrinsic, monotony, slope, butterfly, calendar, parity bound) are in `fa options check --help`. The intrinsic check is a buy at the ask and an exercise at once. A quote that breaks the intrinsic check leaves them. The header counts these quotes.

## Data limits

- The IBM and JNJ files have no `source` field and no time of the quotes. They are Cboe delayed quotes.
- The spot (IBM 228.53, JNJ 272.70) is a last price. A stock price has no bid and no ask. The result is one day of one source.

## Results

| Item | IBM | JNJ |
|---|---|---|
| Quotes in file (bid 0 or missing: 269 and 318) | 1700 | 1006 |
| Quotes used | 1431 | 688 |
| Removed (ask below intrinsic value) | 43 | 28 |
| Monotony breaks / tests | 56 / 29295 | 1 / 7451 |
| Slope breaks / tests | 262 / 29295 | 45 / 7451 |
| Butterfly breaks / tests | 32 / 1312 | 5 / 596 |
| Calendar breaks / tests | 57 / 8204 | 19 / 2732 |
| Parity-bound breaks / tests | 13 / 523 | 0 / 240 |
| Breaks after the removal | 420 | 70 |
| All breaks | 463 | 98 |
| Quotes in at least one break (sum of the series) | 381 | 102 |

Without the intrinsic check the counts were 708 (IBM) and 257 (JNJ).

## The five largest remaining breaks (checks 1 to 5)

| Size (USD) | Break | Judgment |
|---|---|---|
| 21.95, 21.90, 21.75, 21.75 | IBM call 135 of 2026-12-18, bid 117.85, against the asks of the calls 150 to 170 (slope) | One quote. The call 135 quote is too high: its ask 121.4 is 28 above the intrinsic value. The call 140 has bid 87.8 and ask 91.3; the intrinsic value of the call 135 is 93.53. Volume 1, open interest 1. Stale or wide quote. |

| Size (USD) | Break | Judgment |
|---|---|---|
| 43.00, 42.35, 41.90 | JNJ put 350 of 2026-12-18, bid 122.2, against the asks of the put 350 of three later expiries (calendar) | One quote. The intrinsic value of the put is 77.30. The bid is 45 above it, with 29 days to expiry. Volume 2, open interest 0. Stale or wide quote. |

Judgment: the five largest remaining breaks of each ticker are one bad quote.
The other breaks are not judged. No break is shown to be tradable.

## The five largest intrinsic breaks

IBM: five deep in-the-money puts. JNJ: five deep in-the-money calls. Each ask is 48 to 91 USD below intrinsic, in long-dated contracts with low open interest. The ask is stale or wrong.

## Implied dividend

- q of a 1-day expiry is noise (IBM -55 %). From 120 days: IBM 0.9 % to 2.3 %, JNJ 1.7 % to 2.1 %. q is an approximation for American options.
- The parity check is the upper bound only (the lower bound needs the dividends). Monotony and slope use all strike pairs.
- No finding here is arbitrage. The costs, the borrow, the early exercise and the time of the quotes are not known.

## After arb-1b

Ticket: `PLANNING/board/T-20261120-arb-1b-chain-check-depth.md`. Run: `fa options check T --date 2026-11-19 --rate 0.04 --top 5` (flat rate, no network). The command has new options and a new rule for bid 0.

What changed:
- A quote with bid 0 and an ask above 0 is a buy leg (IBM 219, JNJ 254). It is never a sell leg. Quotes with bid 0 and ask 0 are dropped (IBM 50, JNJ 64).
- New check "bound": bid of a call above S, or bid of a put above K. IBM and JNJ have 0 of them.
- European indexes are a list (SPX, XSP, NDX, RUT, VIX). `--min-size USD` removes small breaks. The output gives four size bands.
- A quote is marked when its Black-76 implied volatility is more than 10 points from the median of its 2 neighbours on each side.
  The default is 10 points. 10 points is a convention: the clean count depends on it (IBM 12 / 61 / 152 at 5 / 10 / 20 points). I chose it before I saw the counts.
  `--iv-mark POINTS` changes it. A marked quote stays in the checks. The output counts the breaks with and without a marked leg.
- `--compare D2` gives the breaks of day D that stay on day D2 (same check, same legs). The store has one day only. The command was not run on two days.

Breaks of the checks 1 to 5 after the removal of the intrinsic breaks (43 for IBM, 28 for JNJ):

| Item | IBM | JNJ |
|---|---|---|
| Breaks, all | 427 | 70 |
| With a marked leg | 366 | 55 |
| With no marked leg | 61 | 15 |
| Band below 0.10 USD (all / no marked leg) | 5 / 3 | 1 / 0 |
| Band 0.10 to 1 USD | 30 / 14 | 3 / 0 |
| Band 1 to 10 USD | 221 / 44 | 13 / 3 |
| Band 10 USD and above | 171 / 0 | 53 / 12 |
| Marked quotes in the file | 69 | 44 |

Sensitivity of the threshold (breaks with no marked leg): 5 points: IBM 12, JNJ 14. 10 points: 61 and 15. 20 points: 152 and 18.
With `--min-size 1`: IBM 392 and JNJ 66 breaks of the checks 1 to 5, of which 44 and 15 with no marked leg.

The five largest breaks with no marked leg, and my judgment:
- IBM, all in the series 2028-06-16: calls 315 to 340 (sizes 5.25 to 6.80 USD; one put slope of 5.25 in 2028-12-15).
  The quotes are scattered: call 330 bid 25.1, call 320 ask 18.3, call 335 bid 13.0, call 340 bid 22.0, call 350 ask 13.4. Volume is 1 to 3.
  The marker did not mark them, because the neighbours are scattered too. Judgment: stale or wide quotes. No break is shown to be tradable.
- JNJ, all with the put 330 of 2026-12-18, bid 93.75 (sizes 31.85 to 33.70 USD). The put 330 intrinsic value is 57.30.
  The bid is 36 above it. The neighbours put 320 (85.7 / 89.7) and put 350 (122.2 / 126.05) are also far above their intrinsic values.
  Volume is empty and open interest is 0. Judgment: a stale quote in a jumbled deep in-the-money series. The marker did not mark it.
- The marker finds a single bad quote (IBM call 135, JNJ put 350) but not a series where many quotes are bad. Open: a bid check against neighbours.
- The largest breaks with a marked leg are the same two quotes as before. One new IBM break: parity 23.00 USD of call 135 (bid 117.85) against the buy-only put 135 (ask 0.89).

Limits:
- No second day is stored, so `--compare` is tested on fixtures only. I did not judge the other breaks.
- The implied volatility of a deep in-the-money American quote is distorted by early exercise. The marker compares quotes of one type only. It also marks liquid penny quotes (IBM261120C250, bid 0.02, ask 0.04, volume 431).

## SPX 2026-11-20

Command: `fa options check SPX --date 2026-11-20 --rate 0.04 --top 5` (default) and the same with `--all-strikes`.
The file is the Alpaca indicative feed (SPX and SPXW, 57 series, spot 6929.78 from the forward of the nearest series).
The indicative feed gives modified quotes, not executable prices (trap #280). SPX options are European: no calendar, parity, intrinsic or bound test.
No SPX break can be a finding until a feed with real quotes exists. The filter removes 98.6 % of the breaks (10172 to 143) and 94 % of the distinct quotes in a break (4439 to 271). The 143 that stay are below 6 USD.
For a European index the strike filter is the default: a call with K >= 0.99 F and a put with K <= 1.01 F (F is the parity forward of the series).

| Item | `--all-strikes` | Default (strike filter) |
|---|---|---|
| Quotes in file / used (1071 are buy legs; 3262 dropped) | 32046 / 28784 | 28784, of which 12996 removed by the filter |
| Breaks (monotony, slope, butterfly) | 10172 | 143 (10029 breaks of the unfiltered run have a removed leg) |
| With a marked leg / none (marked quotes) | 3686 / 6486 (956) | 1 / 142 (32) |
| Band below 0.10 USD (all / no marked leg) | 93 / 76 | 12 / 12 |
| Band 0.10 to 1 USD | 826 / 633 | 72 / 71 |
| Band 1 to 10 USD | 4733 / 3370 | 59 / 59 |
| Band 10 USD and above | 4520 / 2407 | 0 / 0 |

The five largest breaks with no marked leg:
- `--all-strikes`: 115.42 USD (butterfly, calls 1800 to 2200 of 2027-02-19), 113.55 (slope, calls 800 and 1000 of 2027-04-16), 110.68, 105.37 and 98.36.
  All are deep in-the-money quotes. Judgment: modified quotes of the indicative feed. Not a finding.
- Default: 5.86 USD (butterfly, calls 7100, 7125, 7150 of 2027-09-17), 5.45 (put monotony, SPXW 2027-09-30, puts 6830 and 6840), 5.00, 4.51 and 4.34 (butterflies).
  Example: the bid of the call 7125 is 564.76; the asks of the calls 7100 and 7150 are 569.96 and 547.85. The spreads are 8 to 19 USD wide.
  The put 6830 of 2027-09-30 has bid 442.55, and its last trade is 209.62. The put monotony 5.45 is more than the 4.96 spread of its sold leg (bid 442.55, ask 447.51). The breaks are 0.8 % to 1.2 % of the price. Judgment: wide and modified quotes.
  No break is shown to be tradable. Limits: one day, one feed, no time of the quotes; the other 138 breaks are not judged.

