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

The command reads `data/raw/<T>/options/2026-11-19.json`. It uses no network.
The rate is the Treasury par curve of the date (4.10 % to 4.81 %). `--rate R` sets a flat rate.

## Checks

- Intrinsic (American options): ask below the intrinsic value. A person buys at the ask and exercises at once.
- Monotony, slope, butterfly, calendar and parity bound follow `fa options --help`. A quote that breaks the intrinsic check leaves them. The header counts these quotes.

## Data limits

- The files have no `source` field and no time of the quotes. They are Cboe delayed quotes.
- The spot is a last price. A stock price has no bid and no ask.
- Monotony and slope use all strike pairs. One bad quote makes many breaks.
- The result is one day of one source. A second day gives the share of breaks that stays.

## Results

| Item | IBM | JNJ |
|---|---|---|
| Spot (USD) | 228.53 | 272.70 |
| Quotes in file (bid 0 or missing: 269 and 318) | 1700 | 1006 |
| Quotes used | 1431 | 688 |
| Removed (ask below intrinsic value) | 43 | 28 |
| Series with no break | 6 | 6 |
| Monotony breaks / tests | 56 / 29295 | 1 / 7451 |
| Slope breaks / tests | 262 / 29295 | 45 / 7451 |
| Butterfly breaks / tests | 32 / 1312 | 5 / 596 |
| Calendar breaks / tests | 57 / 8204 | 19 / 2732 |
| Parity-bound breaks / tests | 13 / 523 | 0 / 240 |
| Breaks after the removal | 420 | 70 |
| Intrinsic breaks | 43 | 28 |
| All breaks | 463 | 98 |
| Quotes in at least one break (sum of the series) | 381 | 102 |

The first version (no intrinsic check) gave 708 breaks for IBM and 257 for JNJ.
The intrinsic check removes 245 IBM breaks and 159 JNJ breaks.

## The five largest remaining breaks (checks 1 to 5)

IBM (spot 228.53):

| Size (USD) | Break | Judgment |
|---|---|---|
| 21.95, 21.90, 21.75, 21.75 | Call 135 of 2026-12-18, bid 117.85, against the asks of the calls 150 to 170 (slope) | One quote. The call 135 bid is too high. The call 140 has bid 87.8 and ask 91.3; the intrinsic value of the call 135 is 93.53. Volume 1, open interest 1. Stale or wide quote. |
| 21.85 | Same call 135 bid, against the ask 96 of the call 135 of 2027-01-15 (calendar) | Same bad bid. |

The call 130 of 2026-12-18 (bid 113.15) has the same fault. The five breaks are one bad quote.

JNJ (spot 272.70):

| Size (USD) | Break | Judgment |
|---|---|---|
| 43.00, 42.35, 41.90 | Put 350 of 2026-12-18, bid 122.2, against the asks of the put 350 of three later expiries (calendar) | One quote. The intrinsic value of the put is 77.30. The bid is 45 above it, with 29 days to expiry. Volume 2, open interest 0. Stale or wide quote. |
| 42.20, 41.75 | Same bid against the asks of the puts 310 and 300 of 2026-12-18 (slope) | Same bad bid. |

Judgment: the five largest remaining breaks of each ticker are one bad quote.
I did not find a price break that a person can trade on.
I did not judge the other 415 IBM breaks and 65 JNJ breaks one by one.

## The five largest intrinsic breaks

IBM: five puts (strikes 360 to 440, 2027-12-17 and 2028-01-21), asks 82 to 147.5, 48 to 64 USD below intrinsic.
JNJ: five deep in-the-money calls (strikes 90 to 105), asks 86.25 to 118.5, 64 to 91 USD below intrinsic.
These are long-dated, deep in-the-money contracts with low open interest. The ask is stale or wrong.

## Implied dividend and open points

- The implied dividend q of a short expiry is noise (IBM 2026-11-20: -55 %, 1 day).
- From 120 days: IBM 1.2 % to 2.3 %, JNJ 1.7 % to 2.1 %. q is an approximation for American options.
- A bad bid is not found by the intrinsic check, which tests the ask only. A bid check against neighbours is open.
- No finding here is arbitrage. The costs, the borrow, the early exercise and the time of the quotes are not known.
