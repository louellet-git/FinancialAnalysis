# Data sources: one table of all data sets (2026-11-19)

Purpose. This table lists each data set of the repo, the sources in their order, and the command that gets it.
The command `fa sync` (ticket `T-20261119-data-sync`) brings these data sets up to date. The column "`fa sync`" gives the name of the data set (the names of `--only` and `--skip`) and its maximum age.

Owner rule. A person syncs the data when the person wants to. No program runs all the time.
So each data set needs a source that gives the PAST on request. A source that gives only the present is a weak source.
The program must then copy the data each day, and a day that nobody copies is lost.

## Words in the table

- **Key.** A free or paid key that the person puts in the environment or in `.env` (names in `.env.example`). "none" means no key.
- **API.** The source publishes the address and the rules in its documentation. "site" means a file or an endpoint of a web site
  that has no published rules. A site can change or refuse the program at any time.
- **History on request.** How far back the source answers when the program asks for it.
- **Live test.** The program ran against the real source. "fixtures only" means that the tests use text that follows the documentation,
  and that no test ran against the real source (the key was not available).

## The table

| Data set | Sources in their order | Key | API or site | History on request | Live test | Command | `fa sync` (name, maximum age) |
|---|---|---|---|---|---|---|---|
| SEC company data (facts of the 10-K and 10-Q) | SEC EDGAR `data.sec.gov` JSON (company facts, submissions) | none (contact text in `SEC_USER_AGENT`) | API | All filed years of each filer | See the mapping tests of `rust/mapcheck` | `fa import TICKER` | `sec`, 1 market day (one look at the SEC filing list each day; the facts are asked again when the list shows a newer filing) |
| Filings (text of the 10-K) | SEC EDGAR archive files | none (same contact text) | site files | All filings | Same as above | `mapcheck filing TICKER` | `filings`, with `sec` (tickers that you name and tickers that have the files) |
| Daily prices | 1. Tiingo. 2. Alpaca. 3. Yahoo chart. 4. Nasdaq site API | Tiingo: `TIINGO_API_KEY`. Alpaca: `APCA_API_KEY_ID` and `APCA_API_SECRET_KEY`. Yahoo and Nasdaq: none | Tiingo and Alpaca: API. Yahoo and Nasdaq: site | Tiingo: more than 30 years. Alpaca: since 2016 (IEX feed). Yahoo: from the first trade. Nasdaq: 10 years | Nasdaq: yes, 2026-11-19 (503 tickers, #212). Yahoo: HTTP 429 all day, 2026-11-19 (#141). Tiingo and Alpaca: fixtures only | `fa prices get TICKER...` | `prices`, 1 market day |
| Intraday bars (1 minute, 5 minutes) | 1. Alpaca. 2. Yahoo chart (only 60 days of 5 minutes, 730 days of 1 hour) | Alpaca: two keys. Yahoo: none | Alpaca: API. Yahoo: site | Alpaca: since 2016. Yahoo: 60 or 730 days | Alpaca: fixtures only. Yahoo: HTTP 429 from the container (#90) | `fa prices intraday SYMBOL --from D` (Alpaca). `fa trade fetch` (Yahoo, the bars of the option bots) | `intraday`, 1 market day (5-minute bars of SPY, QQQ and `--intraday-symbols`; Alpaca with keys, else Yahoo) |
| Option chains now (bid, ask, volume, open interest) | 1. Cboe delayed quotes (file). 2. Alpaca "indicative" feed | Cboe: none. Alpaca: two keys | Cboe: site file. Alpaca: API | Present only. A past chain exists only if a person saved it | Cboe: yes, 2026-11-19. Alpaca: fixtures only | `fa options snapshot SYMBOL --source cboe` or `--source alpaca`. `fa analyze` uses Cboe | `options`, 1 market day (symbols of `--option-symbols`, default SPY) |
| Option surface history (past dates) | 1. Alpaca historical option bars (daily closes of each contract). 2. Massive (paid; plan only, not built) | Alpaca: two keys. Massive: one paid key | Alpaca: API. Massive: API | Alpaca: since February 2024. Massive: 2 years (Starter), 4 years (Developer), more (Advanced); see `docs/ARCHITECTURE.md` | Fixtures only | `fa options history SYMBOL --date D` or `--from D --to D` | none: a person asks for the dates with `fa options history` |
| VIX index and VIX futures | VIX daily close: Yahoo `^VIX` (bar cache of the option bots). VIX futures: Cboe settlement files. The Cboe VIX history file (CSV, from 1990) is a published file that no code reads yet | none | Yahoo: site. Cboe futures files: site files | VIX: from 1990 at Cboe. Futures: the history file of each contract | Futures: yes, 2026-11-19 (#70 to #78). Yahoo: HTTP 429 | `fa trade fetch` (VIX). `fa futures quote VX...` | `vix` (the Cboe history file), 1 market day; the VIX futures are in `futures`; the Yahoo `^VIX` bars are in `intraday` (SPY and QQQ only) and in `fa trade fetch` |
| Futures (CME and CBOT) | 1. Yahoo chart. 2. CFTC positions file. 3. Atlanta Fed policy path | none | Yahoo: site. CFTC and Atlanta Fed: published files | Yahoo: from the first trade of a live contract; an expired contract answers HTTP 404 | Yes, 2026-11-19 (#70 to #78) | `fa futures ...` | `futures`, 1 market day (VX curve, CFTC positions, policy path, Yahoo series of each root) |
| Estimates and earnings dates | Nasdaq site API (current estimates and dates). FMP with `FMP_API_KEY` (older loader). The detail is in the documents of the estimates ticket (another branch) | Nasdaq: none. FMP: `FMP_API_KEY` | Nasdaq: site. FMP: API | Present only. A past estimate exists only if a person saved it | See the estimates ticket | `fa estimates` and `fa earnings` (another branch) | `estimates`, 7 days; `earnings`, 1 market day (calendar scan of 60 days, past dates, SEC filing list) |
| Treasury curves (par, real) | home.treasury.gov CSV, one file for each year | none | published files | Nominal from 1990. Real from 2003 | Yes, 2026-11-19 (#130) | `fa rates curve`, `yield`, `history` | `rates`, 1 market day |
| Federal Reserve curve (GSW) | federalreserve.gov file `feds200628.csv` | none | published file | The whole file | Not recorded in the documents | `fa forwards` | `gsw`, 1 market day |
| New York Fed rates (SOFR, EFFR, OBFR, TGCR, BGCR) | markets.newyorkfed.org JSON | none | published API (the documentation page refused the fetch tool on 2026-11-19: the terms are not read) | The history of each rate | Yes, 2026-11-19 (#130) | `fa rates nyfed` | none: the code has no cache for these rates (`fa rates nyfed` asks the site each time) |
| FRED series (spreads, yields, indexes) | 1. FRED API with a key. 2. FRED CSV with no key | `FRED_API_KEY` for the API | API (1). Site (2) | Each series from its first date. Limit: 120 requests each minute | API: fixtures only. CSV: the site closed the connection from the container, 2026-11-19 (#128) | `fa rates fred`, `fa bonds ...` | `fred`, 1 market day (the series of the bond functions in `data/raw/bonds/`) |
| Inflation (breakevens, CPI) | Treasury real and nominal curves. BLS API (CPI). FRED T10YIE and T5YIFR for the cross-check | BLS: none (limit for each address, #129) | BLS: API. Others as above | CPI: the BLS series. Breakevens: from 2003 | Yes, 2026-11-19, but not FRED (#130) | `fa rates inflation` | `rates` (the BLS CPI file); the breakevens come from the Treasury files |
| FedInvest bond prices | TreasuryDirect FedInvest price file | none | site (a session is needed) | One file for each date. A past date works only if the file is still on the site | The Rust download failed with HTTP 400; `curl` worked (#67, #69) | `fa recon bonds --date D` (reads `data/raw/prices/fedinvest_<date>.csv`) | `bonds`, 1 market day (the file of the last market day before today) |

## Which data sets still depend on a site or a paid plan

Data sets with a published API and history on request, after this change (all need a key except SEC, Treasury, the BLS and the New York Fed):

- SEC company data. Treasury curves. New York Fed rates. BLS CPI. (no key)
- Daily prices (Tiingo, Alpaca). Intraday bars (Alpaca). FRED series (FRED API). Option surface history since February 2024 (Alpaca). (free key)

Data sets that still depend on a site endpoint:

- Option chains now (Cboe file), VIX futures (Cboe files), CME futures (Yahoo), estimates and earnings dates (Nasdaq site API), FedInvest prices.
- These sources can refuse the program. Two of them refused the cloud container on 2026-11-19.

Data sets with no source for the past, with no payment:

- The option surface of SPX and VIX options. Alpaca does not confirm index options. Massive is the candidate (paid). See `docs/ARCHITECTURE.md`.
- Past analyst estimates. No source with a published API was found.
- A bid and ask of a past date. Alpaca gives closes only (no historical quotes page was found).

## Rules for `fa sync`

1. Try the sources in the order of the code (it is the order of the table, except the prices: the source of the cache goes first and no other source replaces it). Stop at the first source that answers.
2. A key that is missing is not an error. The sync goes to the next source and prints, in the summary, which key would add which source.
3. A source that answers HTTP 429 or HTTP 403, or gives a block page, stops for the run. The cache stays as it was. The other hosts go on.
4. A request of a keyed source never prints the key (`rust/src/market/keys.rs`).
5. A data set with a "site" source needs a daily copy to keep its past. The sync does not copy it by itself: nothing runs between two commands (owner decision of 2026-11-19). Run `fa sync` on each day that you need the data.
6. The SEC data sets (`sec`, `filings`, and the SEC part of `earnings`) need `SEC_USER_AGENT`. Without it the sync skips them and says so.
7. One queue for each host (Nasdaq, Alpaca, Tiingo, Yahoo, Cboe, Treasury, New York Fed, FRED, Federal Reserve, BLS, TreasuryDirect, Fiscal Data, SEC, CFTC, Atlanta Fed). The hosts run at the same time. Two requests to one host never run at the same time.
8. The age of a data set with a maximum age of 1 day is counted in market days: a weekend or a market holiday does not make the data old. `sync.toml` (optional) changes the ages.

## Facts not confirmed

The facts of each Alpaca, FRED and Massive plan, with their URLs, are in `docs/ARCHITECTURE.md` (section "Keyed data sources")
and in `docs/HIDDEN-ASSUMPTIONS.md` (#230 to #244). Prices of paid plans come from pages that change: read them again before you pay.
