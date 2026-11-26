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
| Fund distributions | `data/raw/URA/distributions.json` (`fa fund distributions URA`: Yahoo chart `events=div`, one request; record with source, date and address) | Ex-date and cash amount per share; no pay date. Without the file, layer 1b is the price return only, as before |
| US ticker of a holding | `rust/src/market/fund_ticker_map.csv` (ISIN to ticker, 16 lines, same ISIN on both markets) | Only for the monthly view and the price check; `fa prices get T --source nasdaq` fills the files |

## Method (short)
1. Layer 1a: for each report date, holdings + other assets - liabilities against net assets. N-PORT has no number for the
   other assets, so they are total assets - holdings and the difference is 0 by construction (the check is that the report adds
   up). The independent check is the sum of pctVal against holdings / net assets.
2. Layer 1b: price return (Nasdaq close at the two report dates) minus NAV return (product of the three monthly returns,
   rtn1 is the newest month). The difference is the change of the premium or discount plus the distributions.
   With the distribution file: price total return = (close_end + distributions with an ex-date in (start, end]) / close_start - 1.
   The output prints the price-only return, the total return and the difference of the total return to the NAV return (the
   change of the premium or discount, plus timing). The distribution is not reinvested inside the quarter: this is a small
   second-order effect (the cash would earn the return of the fund from the ex-date to the quarter end).
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
1. The foreign part (about 35% to 58% of the net assets of URA, 2023 to 2026) is quarterly only: no permitted monthly price.
2. Trades inside a quarter go to the remainder, with cash, fee and securities lending.
3. The premium is a difference of returns (price total return minus NAV return when the distribution file exists). N-PORT has no share count, so there is no premium level.
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
The monthly view (World Bank monthly average, below) is part of the same command.

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
1. Layer 1a, for each quarter end: ounces per share x gold price - accrued fee per share against the reported NAV per share. This is a check of the sum of the filing and its rounding, not of the gold price or the ounces: the price is market value / ounces, so the identity is (value + receivable - payable - fee) / shares by construction. The ounces per
   share include the gold in settlement: (ounces + (gold receivable - gold payable) / price) / shares. The line prints the difference, and
   the distance of the implied price from a whole cent (the LBMA price is in cents). Flags: NAV difference above half a cent, price off the
   cent grid, net assets / shares against the reported NAV, gold value + receivable - payable - fee payable against net assets (above 1000 USD), an XBRL difference.
2. Layer 1b: GLD close / NAV per share - 1 (the share count is known). It is NOT a premium: the NAV is the London PM price (about 15:00 London) and the close is 16:00 New York, so most of the level is the move of gold between the two times (the true premium is about 0.1 pp). The output labels it "close against the NAV at the London price: mostly timing, not a premium" and prints a noise size for each row: the high-low range of the GLD close day as % of the close (from `prices.json`, which has high and low; an upper bound).
3. Layer 2, for each quarter: NAV return = gold price return g + the change of the ounces per share d (the fee paid in gold; output label "oz/sh chg") + rest.
   The rest has an exact split: the cross term g x d, and the effect of the accrued fee payable ((f0 (1+g)(1+d) - f1) / NAV0, f is the fee per share); what is left is rounding
   (thousands of USD in the statement). The fee check prints d against 0.40% x days / 365 and against the fee of the quarter / mean net assets.
4. Layer 2 close against NAV: GLD price return - NAV return = the change of close / NAV (difference form; mostly timing, not a premium; the two day ranges are printed); the ratio form (1+r)/(1+n) - 1 equals the change of the level (printed with the difference of the two, about 1e-16).
5. The price day is the last business day (Monday to Friday) on or before the period end (the filing names the period end as the LBMA date); it is used for the GLD close and the day count (91 days, not 92, for 2023-12-31 to 2024-03-31). A holiday takes the last close before it. The period end and the filing date are printed too.
6. A filing with a NAV, cent-grid, net assets or settlement-gap flag is out of layer 2: the output prints "no attribution" with the reason. A 10-K/A or 10-Q/A is not read: the output prints "amendment not read: FORM filed DATE". A value in parentheses in the statement is negative; a negative ounces, value, shares, net assets or NAV, or a negative payable, is refused.

### Run of 2026-11-26 (13 filings, 2023-09-30 to 2026-09-30, 12 quarters; 15 requests)
- Layer 1a: the identity NAV agrees with the reported NAV in each of the 13 filings (largest difference 0.0040 USD, below the rounding of the reported 2 decimals); the implied price is on the cent grid (largest distance 0.003 cent); the XBRL values agree with the text.
- Gold in settlement is up to 242 million USD (7 of 13 quarter ends). Without it the identity fails by 0.40 to 0.73 USD in those 7.
- Layer 2: the change of the ounces per share (the fee paid in gold) is -0.093 to -0.103 pp in each quarter (the rate gives -0.099 to -0.101 pp); the rest is -0.015 to +0.015 pp; rounding is below 0.0001 pp.
- Layer 1b: close / NAV - 1 is -1.19% to +1.64% (2023-09-29 and 2026-03-31 are the extremes); its change is -1.66 to +1.70 pp in a quarter. The day ranges of the GLD close days are 0.46% to 2.55%: the level is of the size of the timing noise, so it is not read as a premium.
- 2024-03-31 is a Sunday and 2024-03-29 a holiday: the price day is 2024-03-29 (Good Friday, no GLD close) and the close is of 2024-03-28.

### Limits (GLD)
1. The gold price is the trust's own number (market value / ounces), not a second source. Layer 1a tests the arithmetic and the cent grid.
2. The monthly view compares a monthly AVERAGE gold price with a month-END fund price: approximate (see the monthly part).
3. The close against the NAV is mostly timing, not a premium: the NAV uses the LBMA Gold Price PM (about 15:00 London), the GLD close is 16:00 New York. The move of gold between the two times is in the level and in its change. The size is printed for each row (day range of the close day).
4. The change of the ounces per share (the fee paid in gold) is the measure of the fee: it contains every other change of the ounces per share (none is known). The rate of 0.40% is a text of the filing.
5. A quarter is 85 to 100 days between two filings. The last quarter end is missing until its filing is public. A 10-K/A is not used.
6. The text layout of the statement is assumed (tested on the 13 filings of the run and on a trimmed text). A new layout gives "NOT READ" with the reason, not a wrong number.

### Tests (GLD)
`cargo test --manifest-path rust/Cargo.toml --locked fund_gold`. The fixture text is trimmed from the real 10-Q of 2026-06-30 (the sentence on the gold
is the real one; the statement keeps the real numbers and rows, with the layout cut down). The decomposition and settlement tests use constructed round numbers (100 ounces,
10 shares, prices 2000 and 2200) with a known answer. The filing list and companyfacts tests use small constructed JSON. No test uses the network.
Record: `docs/HIDDEN-ASSUMPTIONS.md`, #289.

### The GLD monthly view (2026-11-26)
Input: the World Bank Pink Sheet monthly gold file in `data/raw/commodity/worldbank/` (read through `commodity.rs`; `fa commodity get gold` fills it; the monthly view makes no request; USD per troy ounce, nominal; the file has whole dollars) and the GLD Nasdaq month-end closes.
For each month of the 3 years: GLD return = month-end close / the close of the month before - 1 (the last close on or before the last business day); World Bank change = the monthly average / the average of the month before - 1; difference = GLD return - World Bank change. The output is labelled "monthly AVERAGE gold price against a month-END fund price: approximate". The quarter ends of layers 1 and 2 use the trust's own LBMA price.
Quarterly check: the World Bank change over the 3 months (average of the last month / average of the month of the earlier quarter end), and the sum of the three monthly changes, against the layer-2 gold return of the same quarter. The difference is the averaging error. A rising price makes the average lag the month end, so the error has the sign of the end of the move, and is not an error of the trust or of the World Bank.
Series break: before 2025-06 the World Bank value is the London afternoon fixing average; from 2025-06 it is a spot average of daily rates (`commodity::GOLD_BREAK_MONTH`; the Description line of the file is compared at each run, a changed text prints a warning). The row of 2025-06 and the quarter that contains it are marked.
Run of 2026-11-26 (36 months, 12 quarters, cache only): the monthly difference is -7.78 to +7.37 pp (the +7.37 is 2023-10, where the file has the same whole-dollar value, 1916, in 2023-09 and 2023-10); the quarterly averaging error is -6.97 to +6.87 pp, mean absolute 3.91 pp. The monthly view is therefore a picture of direction and size, not a measure: it cannot replace the quarter ends of layers 1 and 2.
Tests: `the_monthly_arithmetic_and_the_averaging_error` (constructed closes 100, 110, 99, 99 and averages 100, 105, 105, 105: the monthly returns, the break mark, the quarter error +6 pp, a missing month gives no error).

### URA, layer 1b header (2026-11-26)
The program prints, in the line of the distributions, how many ex-dates are "not cross-checked": an ex-date after the end of the last fiscal year that an N-CSR covers (URA: 2025-10-31, set by hand in `fund_distributions::ncsr_covered_to`; the N-CSR is not read). Run: 1 of 20 (2025-12-30, 2.0840). A ticker with no entry prints "not cross-checked (no N-CSR year known for this ticker)".
