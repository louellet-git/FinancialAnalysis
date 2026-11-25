# Model: fund attribution (layers 1 and 2 of an ETF)

Updated: 2026-11-25. Commit: not yet made.

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
