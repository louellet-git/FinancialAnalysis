# Model: fund attribution (layers 1 and 2 of an ETF)

Two parts: the N-PORT part (URA, a fund of shares; below) and the gold trust part (GLD; the last section).

Updated: 2026-11-26. Commit: pull request 262.

## Purpose
Explain an ETF (first URA, the uranium fund) so that it is not a black box. Layer 1: do the net assets agree with the
holdings, and how far is the price return from the NAV return (the change of the premium)? Layer 2: how much of the fund
return in each quarter comes from each holding and each group, and what is left over (a named remainder)?
The user is the Head of Economics. The model is a measurement. It makes no forecast and no trade signal.

## Inputs
| Input | Source (variable, file or command) | Note |
|---|---|---|
| Holdings, net assets, monthly NAV returns, flows | SEC N-PORT, `data/raw/URA/nport/*.xml` (`fa fund holdings URA --from D`) | One report for each fiscal quarter (31 Jan, 30 Apr, 31 Jul, 31 Oct for URA); NPORT-P, an amendment only if the NPORT-P is missing |
| Price of a holding in USD | valUSD / balance of the line (units NS, asset category EC, long) | Both quarter ends; no other price source; includes the exchange rate of the filer |
| Fund price | `data/raw/URA/prices.json` (Nasdaq, adjusted for splits only) | NO distributions: a price return, not a total return |
| US ticker of a holding | `rust/src/market/fund_ticker_map.csv` (ISIN to ticker, 16 lines, same ISIN on both markets) | Only for the monthly view and the price check; `fa prices get T --source nasdaq` fills the files |

## Method (short)
1. Layer 1a: for each report date, holdings + other assets - liabilities against net assets. N-PORT has no number for the
   other assets, so they are total assets - holdings and the difference is 0 by construction (the check is that the report adds
   up). The independent check is the sum of pctVal against holdings / net assets.
2. Layer 1b: price return (Nasdaq close at the two report dates) minus NAV return (product of the three monthly returns,
   rtn1 is the newest month). The difference is the change of the premium or discount plus the distributions.
3. Layer 2: link each holding of the start report to the end report by ISIN, else CUSIP, else name. Part = start weight
   (valUSD / net assets) x (end price / start price - 1). Sum by group (US-listed if the ISIN is in the map file, else foreign).
4. Remainder = NAV return - sum of the parts. Named line: "trading in the quarter, cash, fee, securities lending, other".
5. A holding that enters or leaves has no start or end price: it is out of the parts, and its return is in the remainder.
   Estimates (not facts) are printed beside the remainder: entering US-listed holdings (Nasdaq closes, held from the start) and
   the trades in the linked holdings (balance change less the pro-rata change, x half the return).
6. A price factor outside 1/3 to 3, a currency change, or a balance and price change that a split explains gets a FLAG. A split
   or consolidation that the list of factors (2, 3, 4, 5, 10, 20, 50, 100 and their inverses) explains is adjusted: return =
   price factor x share factor - 1. The factor is inferred from the balance, not read from a record.
7. Monthly view: for the US-listed holdings, start weight x (month-end close / start close - 1) from the Nasdaq files, summed,
   against the cumulative NAV return of the fund. The foreign part is one line, quarterly only.

## Outputs
Plain text: layer 1a table, and for each quarter layer 1b, layer 2 (parts, remainder, estimates, largest parts, flags, price
check of the map against Nasdaq) and the monthly view, then a summary table and the six limits. No file is written.

## Assumptions
No assumption record: the model has no tuned value. The constants are in the code (`fund_attribution.rs`): the flag limits
(1/3 and 3), the split list, the balance tolerance (ln 0.45), the quarter gap (80 to 100 days), the remainder warning (1 point), the map price tolerance (8%: above it the
output prints MAP SUSPECT; with no Nasdaq close it prints price check NOT RUN).

## Limits
1. The foreign part (about a quarter of the net assets) is quarterly only: no permitted monthly price.
2. Trades inside a quarter go to the remainder, with cash, fee and securities lending.
3. The premium is a difference of returns. N-PORT has no share count, so there is no premium level.
4. GLD months are approximate (World Bank monthly average): this model is not run for GLD yet.
5. The look-through of the Sprott trust needs the consent of Sprott: the trust is one foreign line.
6. The last quarter can be missing: N-PORT is public 60 days after the quarter end, for the third month only.
Also: the repo and money market lines (cash collateral of securities lending) are in the holdings but can be outside total assets;
the NAV is struck with foreign prices at their own close, so the premium difference includes this timing; the NAV return of N-PORT
is net of the fee. The output also prints these cautions: each quarter uses a report filed about 60 days later (a look back);
the 2026 ticker map is used for all quarters; a foreign return includes the currency change; the split rule can misfire; an
amendment is not compared with its original.

## Code and command
| What | Path or command |
|---|---|
| Code | `rust/src/market/fund_attribution.rs`, `rust/src/market/fund_holdings.rs` |
| Model or parameter file | `rust/src/market/fund_ticker_map.csv` |
| Command | `fa fund holdings URA --from 2023-07-31` (network), then `fa fund attribution URA` (cache only) |
| Tests | `cargo test --manifest-path rust/Cargo.toml --locked fund_` (fixed-answer fixtures: two quarters, premium, enter, leave, missing price, split) |

## Review status
Gate of Quality and Risk (model validator), 2026-11-25: PASS-WITH-NOTES; the validator reproduced the 12 remainders with its own
parse. Its two conditions are done: the map price check has a tolerance and prints MAP SUSPECT or NOT RUN; a remainder above 1 point
prints "probable cause (not proved)" and the unexplained part. The entering estimate is a bound (the fund bought during the quarter). Run of 2026-11-25 on URA: 13 reports, 12 quarters; the remainder is between -0.25 and +0.35 points in 10 quarters
and +1.89 and +1.59 points in two (2024-10-31: Oklo entered the fund, bound +1.71, unexplained +0.06; 2025-10-31: trades and an entering holding, unexplained +0.32 by
this estimate and +0.64 by the validator's estimate).
Record: `docs/HIDDEN-ASSUMPTIONS.md`, #288.

## The GLD part (a gold trust, quarter ends; 2026-11-26)
Command: `fa fund attribution GLD [--fetch] [--refresh] [--from D] [--to D] [--cache DIR]`. The program chooses this part from the ticker.
GLD holds gold bars only, so there is no N-PORT list of holdings. The trust is an SEC filer (CIK 1222333), and its own 10-Q and 10-K
state the gold, the NAV and the fee. The model uses the 4 quarter ends of the fiscal year (31 Dec, 31 Mar, 30 Jun, 30 Sep).
The monthly view: not yet built (it waits for an owner answer).

### Inputs (GLD)
| Input | Source | Note |
|---|---|---|
| Ounces of gold, market value of the gold, date of the LBMA Gold Price PM | Text of the 10-Q or 10-K: the sentence "the amount of gold owned by the Trust ... ounces ... market value ... based on the LBMA Gold Price PM on DATE" | Custom item: not in the XBRL companyfacts file |
| Gold price used by the trust | Market value / ounces (implied) | The filing names the LBMA Gold Price PM but gives no price number |
| Gold receivable, gold payable, accounts payable to the sponsor (the accrued fee), net assets, shares, NAV per share | Text of the statement of financial condition (USD thousands) | A receivable or payable row exists only in quarters that have one; no row is 0 |
| Sponsor fee of the quarter | XBRL companyfacts, tag `SponsorFees`, the 3-month fact | Cross-check lines only; not in the identity |
| Cross-check | XBRL `AssetsNet`, `SharesOutstanding`, `NetAssetValuePerShare` at the period end | A difference is printed as `XBRL check` |
| GLD close | `data/raw/GLD/prices.json` (Nasdaq, split-adjusted; GLD pays no distribution) | The last close up to 5 days before the date (a weekend period end) |
Cache: `data/raw/GLD/trust/` (`submissions.json`, `company_facts.json`, `ACCESSION.htm`, `record.json`). Without `--fetch` no request is sent.
A first run with `--fetch` needs 2 requests and one for each filing. The SEC client names the program and reads the contact from `sec_contact.toml` or `SEC_USER_AGENT`.

### Method (GLD)
1. Layer 1a, for each quarter end: ounces per share x gold price - accrued fee per share against the reported NAV per share. The ounces per
   share include the gold in settlement: (ounces + (gold receivable - gold payable) / price) / shares. The line prints the difference, and
   the distance of the implied price from a whole cent (the LBMA price is in cents). Flags: NAV difference above half a cent, price off the
   cent grid, net assets / shares against the reported NAV, gold value + receivable - payable - fee payable against net assets (above 1000 USD), an XBRL difference.
2. Layer 1b: NAV per share against the GLD close: the premium LEVEL, close / NAV - 1 (the share count is known).
3. Layer 2, for each quarter: NAV return = gold price return g + fee drag d + rest. The fee drag d is the change of the ounces per share.
   The rest has an exact split: the cross term g x d, and the effect of the accrued fee payable ((f0 (1+g)(1+d) - f1) / NAV0, f is the fee per share); what is left is rounding
   (thousands of USD in the statement). The fee check prints d against 0.40% x days / 365 and against the fee of the quarter / mean net assets.
4. Layer 2 premium: GLD price return - NAV return = the change of the premium (difference form); the ratio form (1+r)/(1+n) - 1 equals the change of the premium level (printed with the difference of the two, about 1e-16).

### Run of 2026-11-26 (13 filings, 2023-09-30 to 2026-09-30, 12 quarters; 15 requests)
- Layer 1a: the identity NAV agrees with the reported NAV in each of the 13 filings (largest difference 0.0040 USD, below the rounding of the reported 2 decimals); the implied price is on the cent grid (largest distance 0.003 cent); the XBRL values agree with the text.
- Gold in settlement is up to 242 million USD (7 of 13 quarter ends). Without it the identity fails by 0.40 to 0.73 USD in those 7.
- Layer 2: the fee drag is -0.093 to -0.103 pp in each quarter (the rate gives -0.099 to -0.101 pp); the rest is -0.015 to +0.015 pp; rounding is below 0.0001 pp.
- Layer 1b: the premium level is -1.19% to +1.64% (2023-09-29 and 2026-03-31 are the extremes); the change of the premium is -1.66 to +1.70 pp in a quarter.

### Limits (GLD)
1. The gold price is the trust's own number (market value / ounces), not a second source. Layer 1a tests the arithmetic and the cent grid.
2. The monthly view is not yet built. A later monthly view with the World Bank monthly average compares an average of a month with a month-end NAV: they do not match.
3. The NAV uses the LBMA Gold Price PM (3:00 PM London); the GLD close is at 4:00 PM New York. The premium level has the move of gold between the two times.
4. The fee drag is the change of the ounces per share: it contains every other change of the ounces per share (none is known). The rate of 0.40% is a text of the filing.
5. A quarter is 85 to 100 days between two filings. The last quarter end is missing until its filing is public. A 10-K/A is not used.
6. The text layout of the statement is assumed (tested on the 13 filings of the run and on a trimmed text). A new layout gives "NOT READ" with the reason, not a wrong number.

### Tests (GLD)
`cargo test --manifest-path rust/Cargo.toml --locked fund_gold`. The fixture text is trimmed from the real 10-Q of 2026-06-30 (the sentence on the gold
is the real one; the statement keeps the real numbers and rows, with the layout cut down). The decomposition and settlement tests use constructed round numbers (100 ounces,
10 shares, prices 2000 and 2200) with a known answer. The filing list and companyfacts tests use small constructed JSON. No test uses the network.
Record: `docs/HIDDEN-ASSUMPTIONS.md`, #289.
