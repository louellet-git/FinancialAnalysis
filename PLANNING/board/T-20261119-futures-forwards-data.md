---
id: T-20261119-futures-forwards-data
title: Get futures prices (the forward rates are done)
area: engine
status: review
owner: louellet-git
branch: claude/futures-prices
created: 2026-11-19
acceptance: One `fa` command prints the futures prices for a date, each with its source and its date. The code is Rust and it calls no Python code.
---
Owner request, 2026-11-19: a branch that gets futures and forward data. Merge to `main` when it is done.

Scope of the first step: interest-rate data, because the next work is the bond comparison (`T-20261119-bond-recon`). Forward rates come from official sources or from the Treasury curve that the repo has. The ticket records the futures sources, their terms as read, and their limits.

## State, 2026-11-19

Done (Rust, `rust/src/forwards.rs`): `fa forwards [--date D]` and `fa forwards check [--days N]`.
Source: Federal Reserve Board GSW file `feds200628.csv` (daily, from 1961-06-14).

Done (Rust, `rust/src/futures/`): `fa futures roots | quote | history | curve | positions | policy-path | check`.
The commands are in `README.md`; the traps are hidden assumptions 70 to 78.

Owner decision, 2026-11-19: "Use whatever resource you can to extract quality info for free,
for futures." The rule "only sources whose terms clearly permit automated use" is replaced by
this decision. The rules of conduct stay: a plain User-Agent text (`FinancialAnalysis fa
(personal research)`), no browser imitation, no way around a block, a login, a captcha, a
paywall or a rate limit, a low request rate (0.5 s between two requests, and a cache), no
personal data in a request.

### Sources in use

| Source | Data | Price type | Terms as read |
|---|---|---|---|
| Cboe settlement file (`cboe.com/us/futures/market_statistics/settlement/csv?dt=D`) and history file (`cdn.cboe.com/.../historical_data/VX/VX_<expiry>.csv`) | VIX futures, from 2013 | settlement | The Cboe page "Use of Content" says: "you must receive approval in advance from Cboe" for Cboe data and other content, and "You are not approved to use Cboe Content until a license agreement has been signed by both you and Cboe." The page does not mention automated access or personal use. OPEN RISK: the owners must decide. |
| Yahoo Finance chart endpoint (`query1.finance.yahoo.com/v8/finance/chart/...`) | ES, NQ, ZT, ZF, ZN, TN, ZB, UB, SR3, SR1, ZQ: daily close from 2000 for `=F` series, from the contract start for one contract; last trade | daily close as Yahoo gives it; last trade | Yahoo Terms, section 2(d): do not "access or collect data ... from our Services using any automated means, including but not limited to robots, spiders, scrapers, data mining tools, or data gathering or extraction tools, for any purpose without our express, prior permission." The owner decision covers its use. OPEN RISK: Yahoo can block the client. |
| CFTC public reporting dataset `gpe5-46if` (Traders in Financial Futures, futures only) | Positions by class of trader, weekly | positions of Tuesday | The CFTC page says: "The COT Report is generally published each Friday at 3:30 pm Eastern Time (US)". The page states no reuse terms. |
| Atlanta Fed Market Probability Tracker (`mpt_histdata.xlsx`) | Implied path of the policy rate from CME options on 3-month SOFR futures, from 2023-03-29 | model estimate | The file says: "Use of this data is permitted for personal and educational purposes only." and "CME GROUP MARKET DATA IS USED UNDER PERMISSION FROM CME AS A SOURCE OF INFORMATION FOR THIS REPORT." |
| NY Fed EFFR API (`markets.newyorkfed.org`) | EFFR, for the cross-check only | rate | Public reference rate; used by the Python code of the repo too. |

### Sources tested and not used

- CME Group website (`cmegroup.com/CmeWS/...`): HTTP 403 with the text "This IP address is blocked
  due to suspected web scraping activity ... Use of scripts, software, spiders, robots, avatars,
  agents, tools or other scraping mechanisms is strictly prohibited". The program does not retry.
- `ftp.cmegroup.com`: connection closed.
- Stooq (`stooq.com` and `stooq.pl`): `stooq.com` answers with a page that needs JavaScript
  ("This site requires JavaScript to verify your browser"); `stooq.pl` closes the connection.
  The program does not run the script.
- Nasdaq Data Link CHRIS: HTTP 403. Investing.com: HTTP 403. Barchart: empty answer.
- Databento (licensed CME data): paid after a one-time credit; not tested.

### Limits

- The CME prices (ES, NQ, Treasury, SOFR, fed funds) have one source only (Yahoo). No second
  source exists to compare the daily close with the settlement. See hidden assumption 76.
- Yahoo has almost no daily history for SOFR futures; only the last trade is usable
  (hidden assumption 72). Yahoo gives no expired contract.
- The VIX settlement file of Cboe can be older than the date asked; the program guards it
  (hidden assumption 73).
- The contract facts are typed from the exchange specifications and are not tested.

Acceptance: met for the `fa futures` commands (each value has its source, URL, date and price
type; the code is Rust and calls no Python). The terms risks of Cboe and Yahoo need an owner
decision before the commands run in a scheduled job.
