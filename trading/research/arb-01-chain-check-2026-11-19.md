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

- Intrinsic (American options): ask below the intrinsic value. A person buys at the ask and exercises at once.
- Monotony, slope, butterfly, calendar and parity bound are in `fa options check --help`. A quote that breaks the intrinsic check leaves them. The header counts these quotes.

## Data limits

- The files have no `source` field and no time of the quotes. They are Cboe delayed quotes.
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

IBM (spot 228.53):

| Size (USD) | Break | Judgment |
|---|---|---|
| 21.95, 21.90, 21.75, 21.75 | Call 135 of 2026-12-18, bid 117.85, against the asks of the calls 150 to 170 (slope) | One quote. The call 135 quote is too high: its ask 121.4 is 28 above the intrinsic value. The call 140 has bid 87.8 and ask 91.3; the intrinsic value of the call 135 is 93.53. Volume 1, open interest 1. Stale or wide quote. |
| 21.85 | Same call 135 bid, against the ask 96 of the call 135 of 2027-01-15 (calendar) | Same quote. |

The call 130 of 2026-12-18 (bid 113.15) has the same fault.

JNJ (spot 272.70):

| Size (USD) | Break | Judgment |
|---|---|---|
| 43.00, 42.35, 41.90 | Put 350 of 2026-12-18, bid 122.2, against the asks of the put 350 of three later expiries (calendar) | One quote. The intrinsic value of the put is 77.30. The bid is 45 above it, with 29 days to expiry. Volume 2, open interest 0. Stale or wide quote. |
| 42.20, 41.75 | Same bid against the asks of the puts 310 and 300 of 2026-12-18 (slope) | Same quote. |

Judgment: the five largest remaining breaks of each ticker are one bad quote.
The other breaks are not judged. No break is shown to be tradable.

## The five largest intrinsic breaks

IBM: five puts (strikes 360 to 440, 2027-12-17 and 2028-01-21), asks 82 to 147.5, 48 to 64 USD below intrinsic.
JNJ: five deep in-the-money calls (strikes 90 to 105), asks 86.25 to 118.5, 64 to 91 USD below intrinsic.
These are long-dated, deep in-the-money contracts with low open interest. The ask is stale or wrong.

## Implied dividend

- The implied dividend q of a short expiry is noise (IBM 2026-11-20: -55 %, 1 day).
- From 120 days: IBM 0.9 % to 2.3 %, JNJ 1.7 % to 2.1 %. q is an approximation for American options.
- The parity check is the upper bound only (the lower bound needs the dividends). Monotony and slope use all strike pairs.
- A quote with bid 0 and an ask above 0 is dropped (IBM 219, JNJ 254), although its ask is a valid buy leg.
- No finding here is arbitrage. The costs, the borrow, the early exercise and the time of the quotes are not known.

## Open
- A bad bid passes the intrinsic check, which tests the ask only. Add a bid check against the neighbours.
- Add a size floor or size bands for the breaks. A second day gives the share that stays.
- Give an explicit list of European indexes before NDX, RUT and XSP are checked.
- Use the ask of a bid-0 quote, with the bounds call <= S and put <= K.
- Tests to add: missing ask, one strike, days = 0, calendar across two roots, duplicate strike.
