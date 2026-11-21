# Continuous option data: Alpaca limits, the e2-micro, and three designs (2026-11-21)

Question: can we copy the SPX, SPY and VIX option chains during all market hours, and check the no-arbitrage
relations on each new snapshot, on a free Google Cloud e2-micro in us-east1? A research agent read each source
on 2026-11-21. It did not call an API. Sizes come from our stored file. "Not documented" means that the pages
read do not give the value. `chiefs/louellet-git/quant/algo-host-2026-11-21.md` was not in either checkout: not used.

## 1. Alpaca REST limits and the requests for one chain
| Item | Basic (free) | Algo Trader Plus ($99/month) | Source |
|---|---|---|---|
| Historical API calls | 200 / min | 10,000 / min | [A1] |
| Option feed | indicative | OPRA | [A1] |
| Historical data limit | latest 15 minutes | no restriction | [A1] |
| Chain snapshot `limit` | default 100, maximum 1000 | same | [A2] |

- Endpoint: `GET /v1beta1/options/snapshots/{underlying}` with `feed`, `limit`, `page_token`, `type`,
  `strike_price_gte/lte`, `expiration_date`, `expiration_date_gte/lte`, `root_symbol`, `updated_since` [A2].
  Each `latestQuote` has its own time `t` [A2]. Our parser (`alpaca::parse_chain`) does not keep `t` now.
  Our call: `limit=1000`, one call for each root, 0.4 s between two requests (150 / min).
- Stored file `data/raw/SPX/options/2026-11-20.json`: source `alpaca`, feed `indicative`, roots SPX and SPXW,
  32,046 quotes, 53 expiries, 28,984 quotes with a bid above zero. At 1000 a page this is at least 33 requests.
  `docs/DATA-SOURCES.md` records 33 requests for SPX, 14 for SPY (12,500 quotes), 3 for VIX (1,510 quotes).
- One full set (SPX + SPY + VIX) = 50 requests = 25% of 200 / min. At 0.4 s a request, one set takes at least
  20 s. Thus a "snapshot" is not one moment: the quotes of the first and the last page are 20 s or more apart.
- Near expiries: SPX has 16,440 contracts to 45 days (17 pages) and 2,124 on the day of expiry (3 pages).

## 2. Alpaca option stream, feeds, index options
- URL `wss://stream.data.alpaca.markets/v1beta1/{feed}`, feed `indicative` or `opra` [A3].
- Encoding: MessagePack only; JSON gives error 412 [A3]. A `*` subscription for option quotes is refused (413) [A3].
- Symbols: Basic 200 option quotes, Algo Trader Plus 1000 option quotes [A1].
- Connections: "in many subscriptions (or without one) this limit is 1" (error 406 above it) [A4]. Basic: not documented.
- Indicative feed: "the quotes are not actual OPRA quotes, they're just indicative derivatives"; "the trades are
  also derivatives and they're delayed by 15 minutes" [A5]. A quote delay: not documented in the docs. An Alpaca
  employee wrote on the forum (2025-06-25): "The indicative option quotes are not delayed, but adjusted" [A6].
- Index options: the roots SPX, SPXW, SPXO, XSP, VIX, VIXW, DJX, DJXW are listed [A7]. The page names only the
  OPRA feed for their data [A7]. Our stored SPX file shows that the indicative REST chain also has SPX and SPXW.
  Index options on the indicative stream: not documented and not tested.

## 3. The underlying at the same moment
- Alpaca: "Market data for the underlying index level (spot SPX, VIX, and similar) is not in the initial
  offering" [A7]. SPY is a stock: Basic has real-time IEX data only [A1].
- SPX: our archive already takes the forward of each expiry from put-call parity of the mids (57 of 57 series
  on 2026-11-20). This forward is at the same moment as the chain and needs no other source. Then parity is not a
  test of the SPX chain (it defines the forward); the tests are strike monotonicity, convexity, calendar order.
- VIX: the parity forward of a VIX expiry is the forward of the VIX future, not the VIX index. A free intraday VIX
  index source that permits automatic requests: none found. The Cboe quote pages say: "It is strictly prohibited
  to download delayed quote table data from this web site by using auto-extraction programs/queries and/or
  software" and "Cboe will block IP addresses" [C1]. The delay of the Cboe index value: not documented on [C1].
- Finding for the owner: `rust/src/trading/chain.rs` (the paper trader) reads the Cboe delayed-quote file
  automatically. The text of [C1] is about the quote table of the site; whether it also covers that JSON file is
  not documented. Do not use it for a continuous capture.

## 4. The e2-micro (free tier)
- Free: 1 non-preemptible e2-micro a month in us-west1, us-central1 or us-east1, for all hours of the month;
  30 GB-months standard persistent disk; 1 GB outbound data a month from North America (not China, Australia) [G1].
- Machine: 2 vCPUs, each 12.5% of CPU time (25% in total); bursts of about 30 s at 100%; 1 GB memory [G2].
- A process that runs all market day (or all day) is in the free hours, because the hours equal the hours of the
  month [G1]. Usage above a limit is billed at standard rates; Google can change the offer with 30 days notice [G1].
- Inbound data is free ([G3], from a search summary, not a full read). The boot disk uses part of the 30 GB.
  The price of an external IPv4 address: not checked here.

## 5. Size estimate (SPX only, measured on the 2026-11-20 file)
| Form | One snapshot | gzip | 1 min (390/day) | 2 min (195) | 5 min (78) |
|---|---|---|---|---|---|
| Stored JSON (indented) | 10,378,582 B | 749,628 B (13.8x) | 292 MB/day, 6.14 GB/month | 146 MB, 3.07 GB | 58 MB, 1.23 GB |
| CSV symbol,bid,ask | 1,032,407 B | 284,364 B (3.6x) | 111 MB/day, 2.33 GB/month | 55 MB, 1.16 GB | 22 MB, 0.47 GB |

- A month = 21 market days; a day = 390 minutes (09:30 to 16:00 ET). `gzip -9` gives 704,033 B for the JSON.
- Without gzip the JSON at 1 minute is 4.05 GB a day: it does not fit on the disk. The raw Alpaca reply size:
  not measured (no call). SPY and VIX add to these numbers; not measured.
- The 1 GB free egress is the hard limit if we send the data out of the VM (for example to the data store).

## 6. Options for us
| Design | Requests | Disk / month (SPX, CSV gzip) | Risk |
|---|---|---|---|
| A. Full REST chains (SPX, SPY, VIX) every 5 min | 50 per cycle = 10 / min average | 0.47 GB + SPY, VIX | 5-min gaps; a set spans 20 s or more |
| B. Mixed REST: SPX to 45 days every 2 min (17 pages) + full chains every 30 min (50) | about 10 / min average; 17 or 67 at one time | about 0.60 GB (estimate: bytes scale with contracts) + full sets 0.08 GB | indicative quotes are adjusted; check the `t` of each quote |
| C. Stream of 200 fixed SPXW symbols + full REST chain every 15 min | 1 connection; REST 50 per 15 min | stream size not measured | 200 symbols only; one connection; MessagePack; index options on the indicative stream not documented |

- All designs stay far below 200 / min. All designs use the same Alpaca key as `fa options archive`; whether the
  limit is per key or per account is not documented, so one queue for the Alpaca host must stay.
  `updated_since` [A2] can reduce B further (only changed contracts); the reduction is not measured.

## 7. Recommendation
Design B. Start with a one-day smoke test on the VM: SPX to 45 days every 2 minutes, the full SPX, SPY and VIX
chains every 30 minutes, CSV gzip with the quote time `t` of each contract, and the strike and calendar checks
on each SPX snapshot. Keep the files on the VM disk. Measure the real bytes and the request time, then choose the
interval (1 minute is possible for the requests; the disk and the 1 GB egress decide). Use parity forwards for
the SPX level. Do not poll Cboe pages. A same-moment VIX index level stays open.

## Sources (all read 2026-11-21)
- Alpaca: [A1] https://docs.alpaca.markets/us/docs/about-market-data-api; [A2] https://docs.alpaca.markets/reference/optionchain;
  [A3] https://docs.alpaca.markets/docs/real-time-option-data; [A4] https://docs.alpaca.markets/docs/streaming-market-data;
  [A5] https://docs.alpaca.markets/us/docs/historical-option-data; [A7] https://docs.alpaca.markets/us/docs/index-options;
  [A6] https://forum.alpaca.markets/t/indicative-feed-returning-live-data/17187 (forum, not documentation).
- Cboe: [C1] https://www.cboe.com/delayed_quotes/spx/quote_table/
- Google: [G1] https://docs.cloud.google.com/free/docs/free-cloud-features;
  [G2] https://docs.cloud.google.com/compute/docs/general-purpose-machines; [G3] https://cloud.google.com/vpc/network-pricing
