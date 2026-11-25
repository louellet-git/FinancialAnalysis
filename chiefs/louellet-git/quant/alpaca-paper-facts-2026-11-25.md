# Alpaca paper account: documented facts (read 2026-11-25)

Scope: public Alpaca pages only. No API call. Each fact has a source key; the key table gives the URL.
All sources were read on 2026-11-25. "Not documented" means that the pages in the table do not say it.
Forum posts are not a source here.

| Key | URL (read 2026-11-25) |
|---|---|
| PT | https://docs.alpaca.markets/docs/paper-trading |
| OT | https://docs.alpaca.markets/docs/options-trading |
| L3 | https://docs.alpaca.markets/docs/options-level-3-trading |
| PO | https://docs.alpaca.markets/reference/postorder |
| OA | https://docs.alpaca.markets/docs/orders-at-alpaca |
| MD | https://docs.alpaca.markets/docs/about-market-data-api |
| HO | https://docs.alpaca.markets/docs/historical-option-data |
| SM | https://docs.alpaca.markets/docs/streaming-market-data |
| AU | https://docs.alpaca.markets/us/docs/authentication |
| RF | https://docs.alpaca.markets/docs/regulatory-fees |
| UP | https://docs.alpaca.markets/docs/user-protection |
| IX | https://docs.alpaca.markets/us/docs/index-options (Broker API page) |
| FS | https://files.alpaca.markets/disclosures/library/BrokFeeSched.pdf ("Revised on November 17, 2026") |
| TC | https://files.alpaca.markets/disclosures/alpaca_terms_and_conditions.pdf |
| RL | https://alpaca.markets/support/usage-limit-api-calls (dated December 2022) |
| LO | https://alpaca.markets/learn/how-to-trade-options-with-alpaca (updated 2026-07-15) |
| NU | https://alpaca.markets/learn/live-trading-account-non-us (updated 2026-03-11) |
| BP | https://alpaca.markets/blog/alpaca-introduces-index-options-paper-trading/ (2026-07-23) |
| BL | https://alpaca.markets/blog/alpaca-launches-index-options-via-trading-api/ (2026-09-02) |
| BD | https://alpaca.markets/blog/finra-retires-the-pdt-rule-introducing-alpacas-new-intraday-margin-framework/ |

## 1. Fills on paper
- Paper matches orders "against the best available current market price (NBBO)" [PT].
- An order fills only when it is marketable: a buy limit at or above the best ask, a sell limit at or below the best bid [PT]. Thus a buy fills at the ask and a sell at the bid. No fill at mid. Price improvement is not modelled [PT].
- Feed of the fill quote: not documented. A "Paper Only Account holder" gets IEX data only [PT].
- Partial fills: "partial fills for a random size 10% of the time"; the rest is evaluated again while marketable [PT]. This is the only random part that the page gives.
- "Your order quantity is not checked against the NBBO quantities" [PT]. Thus there is no size limit and no depth.
- Paper does not model: market impact, information leakage, slippage due to latency, queue position of non-marketable limits, price improvement, regulatory fees, dividends [PT]. Borrow fees: "Coming Soon!" [PT].
- Options on paper: fill rules for single-leg and mleg orders are not documented. BP says only that fills, liquidity, latency and order handling "can differ from live trading".
- Assignment and early exercise on paper: not documented. Non-trade activities (NTAs: exercise, assignment, expiry) on paper sync "at the start of the following day" [OT]. Halts: not documented.

## 2. Fees, PDT and margin on paper
- Live retail fees [FS]: no commission on stocks and equity/ETF options. Index options: $0.50 per contract plus exchange fees (SPXW $0.50 / $0.59; XSP $0 below 10 contracts, $0.07 at 10 or more).
- Live pass-through fees on options [FS]: SEC $0.0000206 x trade value (sells); FINRA TAF $0 per contract (sells); CAT $0.000003 per equivalent share (100 per contract); ORF $0.015 per contract; OCC $0.025 per contract (buys and sells). Each fee type is added up per day per account and rounded up to $0.01 [FS, RF].
- Paper does not account for regulatory fees [PT]. Paper commissions: not documented (live retail is zero).
- PDT: Alpaca removed the PDT designation, the $25,000 minimum and the day-trade count; an intraday margin pre-trade check replaces them (FINRA effective date 2026-06-04) [BD]. Whether paper uses the same check: not documented.
- Margin trading and short selling are available on paper [PT]. Wash-trade protection applies to paper [UP].

## 3. Market data
- Basic plan (free) is "the default option for both Paper and Live trading accounts" [MD]. Stocks: IEX real time; historical limit "latest 15 minutes" [MD]. Options: "Indicative Pricing Feed" [MD].
- Indicative feed: "not actual OPRA quotes, they're just indicative derivatives"; its trades are 15 minutes delayed [HO].
- Algo Trader Plus ($99 a month): all US stock exchanges (SIP) and the OPRA feed [MD]. Whether a paper-only account can buy it: not documented.
- Websocket limits on Basic: 30 stock symbols; 200 option quotes [MD]. Most users have one active connection for each endpoint; error 406 above the limit [SM]. Historical calls: 200 a minute on Basic [MD].
- Paper uses the same data endpoint as live (data.alpaca.markets) [AU].

## 4. Options on paper
- Options are enabled by default on paper [OT]. "All paper trading accounts will automatically have access to Level 3 strategies" [LO].
- Index options on paper: SPX, SPXW, VIX, VIXW, DJX, XSP [BP]. Live for the Trading API since 2026-09-02 [BL]. Spot index levels are not in the data offer [BL]. Not supported (Broker API page): American-style index options, calendar and diagonal spreads, naked index shorts, extended hours, early exercise [IX].
- mleg: order_class "mleg", at most 4 legs, types market or limit only [PO]. The GCD of the leg ratios must be 1; no stock leg; each short leg must be covered inside the same order [L3]. Limit price: positive is a debit, negative is a credit [PO].
- Expiration day: options orders before 3:15 p.m. ET, and before 3:30 p.m. ET for broad ETFs such as SPY and QQQ [LO]. ITM by $0.01 or more is exercised; without buying power Alpaca sells the position in the last hour [OT].

## 5. Accounts and keys
- Paper keys are different from live keys; live keys do not work on paper [PT, AU]. Paper host: paper-api.alpaca.markets [AU].
- Paper accounts are now created and deleted in the dashboard, not reset; a new account needs new keys; default balance $100k [PT].
- Anyone can open a Paper Only Account with an e-mail address [PT]. A live account has conditions of residence; check them before any live plan [NU].
- Terms: use "solely for your own personal and non-commercial purposes" [TC]. An explicit rule "one account for each person, no sharing": not documented in TC. The indemnity covers "any user of your account" [TC].
- Rate limit of the trading API: 200 requests a minute per account; HTTP 429 above it [RL].

## 6. Order safety
- client_order_id: unique, at most 128 characters, made by Alpaca if not sent [PO]. Result of a duplicate: not documented.
- Options types: market, limit, stop, stop_limit (stop types for single leg only) [PO, OT]. Bracket, OCO, OTO: equities only [PO].
- Options time in force: day or gtc only [PO, OT]. extended_hours must be false for options [OT].

## What this means for us
1. Paper gives a real-time NBBO touch fill: buy at the ask, sell at the bid, no mid, no price improvement. It does not give a fill model. Our cost model for SPY options stays: paper shows the full half-spread, a live limit can do better.
2. The free plan gives only the indicative option feed (not OPRA). Thus the paper quotes and our recorded data are not real OPRA. A true spread record needs Algo Trader Plus ($99 a month); its use on a paper-only account is not documented.
3. Paper has no size check, no depth, no queue and no impact. For 1 to 10 SPY contracts this is small; we must still model it for size.
4. Paper does not charge regulatory fees. We must add ORF, OCC, CAT and SEC from FS (about $0.04 per contract plus SEC on sells).
5. Assignment, early exercise and dividends are not modelled or not documented on paper. Short SPY legs near ex-dividend dates need our own model.
6. mleg (up to 4 legs, market or limit) works on paper at Level 3. The fill rule for mleg is not documented: we must measure it from our own paper orders.
7. Each person needs a paper account and keys (200 requests a minute each); paper is open to all. A live account has conditions of residence: check them before any live plan [NU].
8. Use client_order_id (at most 128 characters) for each order. Respect day/gtc only and the 3:30 p.m. ET SPY cutoff on expiration day.
