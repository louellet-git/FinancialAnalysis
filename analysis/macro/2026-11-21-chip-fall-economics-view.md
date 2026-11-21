# The chip fall of 2026-11-20: the economics view (2026-11-21)

Author: Head of Economics. Question (Luc): why did the chip and AI-hardware companies fall on Friday 11-20, what does the street say, is it tested, and what does the market imply now?

## Main answer

The street gives three reasons. Our data is **consistent with** two of them, and it does **not support** the third as the main cause:

1. **Doubt about the AI build-out (the main reason in the press).** Our data agrees: the suppliers fell 2 to 6 times more than the large buyers of chips, and our engine shows that the price now implies less growth for the suppliers.
2. **An NVDA-specific shock:** a Reuters report at 2:16 PM that the SEC examines the accounting of NVDA. This adds to the fall of NVDA. It cannot explain the whole fall, because names with no SEC story (MU, ANET, DELL, AMAT) fell by the same amount.
3. **China regulation** (a short report on AMD on 11-19). Our data does not support it as the main cause: the names with little China revenue (ANET, DELL) fell as much as AMD.

Heavy selling made the day worse: the volume of NVDA was 2.1 times its 20-day median, and AVGO 2.3 times. Rates were not the cause on the day: yields fell, although Governor Cook spoke in favor of rate increases. That is a flight to safety.

## What the sources say (each page read on 2026-11-21)

| Reason | Source, date | Quote |
|---|---|---|
| AI build-out doubt | CNBC live blog, "S&P 500 and Dow close in correction; Nasdaq erases 2026 gain as AI rout picks up steam", 11-20 6:46 PM EST | "Jensen Huang's ouster at Nvidia all raised questions about the future of the AI buildout"; "Signs of moderation in the capital expenditures splurge" |
| Sector already weak | Yahoo Finance, "Nvidia drops 12%, Broadcom and AMD extend declines as the AI trade melts down", 11-20 2:21 PM CST | "The sector was already on shaky ground following the exit of Nvidia CEO Jensen Huang and concerns over AI financing" |
| SEC and NVDA accounting | Reuters (on investing.com), "Nvidia's accounting is a focus of US SEC investigation, sources say", 11-20 2:16 PM | "SEC enforcement staff are scrutinizing potential accounting issues at the company" |
| China regulation | Motley Fool, "Why AMD Stock Was Plunging on Thursday", 11-19 12:23 PM EST | "Chinese server maker IEIT received updated AMD firmware even after it was blacklisted by the U.S." |
| Rates | CNBC live blog, entry of 11-20 12:38 PM EST (Governor Cook) | "waiting longer to implement rate increases risks getting behind the curve" |

Not used, because the page check did not find them: a quote of an analyst on China, and a market probability of a December rate increase.

## The tests (store data, closes of 11-19 and 11-20)

| If the reason is ... | then this must also be true | Result |
|---|---|---|
| AI build-out doubt | suppliers fall much more than buyers | Yes. Suppliers -10.7% to -16.1%; buyers MSFT -2.8%, GOOGL -2.7%, AMZN -4.1%, META -5.9% |
| AI build-out doubt | the price implies less supplier growth | Yes. Implied 10-year growth, 11-19 to 11-20: NVDA 31.8% to 29.8%, MU 27.8% to 25.4%, AMAT 22.5% to 21.0%; MSFT 17.1% to 16.6%, META 24.2% to 23.4% |
| SEC probe of NVDA | NVDA falls much more than its peers | Partly. NVDA -14.0%; peers -10.7% to -16.1%. An intraday check (a step at 2:16 PM) needs intraday prices; the store has none |
| China regulation | high China revenue falls most | No. ANET -11.6% and DELL -11.0% fell about as much as AMD -13.2% |
| Forced selling | very high volume; a rebound later | Volume: yes (NVDA 2.1x, AVGO 2.3x the 20-day median). Rebound: not known before Monday 11-23 |
| A rate shock | yields up on the day | No. 10-year -12 bp, real 10-year -11 bp |

Source of the implied growth: `fa dcf implied` (the automatic model; a screen, not a reviewed valuation), with `--price-date` 2026-11-13, 11-19 and 11-20, store fa-data 1e2739c. The level of the implied growth is high for all chip names (the model needs 21% to 34% a year for 10 years; AMD is at the bound of the search). Use the change, not the level.

## What the market implies now

The price of the chip names still implies very high growth: about 25% to 30% a year for 10 years for NVDA and MU, against 17% for MSFT. The week took about 3 to 4 points off that growth for NVDA and MU, and 1 to 2 points for the buyers. So the market now expects less from the AI build-out, but it still expects a lot. A further fall needs a cut in the real orders (capex guidance of the buyers, supplier estimates). The next dates: the capex guidance of the buyers in their next reports, and the Fed decision on 12-09.

The view engine of Economics (`fa macro view`) does not apply here: it gives the market premium and the long-run growth of the economy, and no SPF release falls in this week.

## Monday to Thursday: "the stocks followed the yield rise of the week before"

The test (store, weekly Friday closes 2016-11-18 to 2026-11-13, 520 weeks; SPY prices and the GSW 10-year zero yield): the next-week SPY return on this week's change of the 10-year yield. Slope: -2.1% for each point of yield rise, t = -2.3 (no correction for overlap is needed; the weeks do not overlap). In the 14 weeks with a rise of 25 bp or more, the next week had a mean SPY return of -0.54% (all weeks: +0.24%), and 64% of them were negative.

Reading: the lag effect is real in direction, but it is small. The rise of 29 bp in the week to 11-13 predicts about -0.6% for the next week. The fall was about -8.5%. So the yield rise explains less than one tenth of the fall. The rest is the AI and chip news above.

## Limits

- One day and one week; this is "consistent with", not proof.
- No intraday prices, no option data, no VIX close for 11-20 in the store.
- The implied growth comes from the automatic model, not a reviewed valuation.
- Disclosure: the agents of this repo run on Anthropic models. The press of 11-19 and 11-20 names the Anthropic IPO. This note makes no valuation of Anthropic.
