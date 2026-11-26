---
id: T-20261125-fund-attribution
title: URA and GLD layers 1 and 2: N-PORT history with new fields, and `fa fund attribution`
area: importer
status: done
owner: louellet-git
requested_by: louellet-git
head: technology
story: E3.S5
order: 1
branch: claude/fund-attribution
created: 2026-11-25
acceptance: `fa fund holdings URA --from 2023-09-30` reads each quarterly NPORT-P filing of 3 years with the currency, the exchange rate, the net assets and the 3 monthly total returns; `fa fund attribution URA` prints layer 1 (net assets against the sum of the holdings, the price return against the NAV return, so the change of the premium) and layer 2 (each quarter: weight x return of each holding and each group; the parts add to the fund return; a named remainder); the six limits are printed; fixed-answer tests; one real run.
merged: pull request 260 (branch claude/fund-attribution)
---
Owner word (2026-11-25, through the Head of Staff): "What else can we elaborate on so that the ETF URA and GLD are
not black boxes but fully reconcilable/decomposed", then "All three layers this week". Size given: about 4% of
the week for (a) the gold and uranium series, (b) the N-PORT history, (c) the price link, (d) the command. This
ticket is (b), (c) and (d) for URA; GLD and the series (a) follow (T-20261123-commodity-series).
The six limits: the foreign part of URA (about 27%) is quarterly only; trades inside a quarter go to the
remainder (with cash, fee, securities lending); the premium comes from price TR - NAV TR (no shares count);
GLD months are approximate (World Bank monthly average); the Sprott trust look-through needs Sprott's consent;
the last quarter can be missing (N-PORT public 60 days after the quarter, third month only).

Result (2026-11-25): `fa fund holdings URA --from 2023-07-31` read 13 reports (18 requests; 3 amendments not used);
`fa fund attribution URA` gives 12 quarters (report months Jan, Apr, Jul, Oct). Remainder: -0.25 to +0.35 pp in 10
quarters; +1.89 pp at 2024-10-31 (Oklo entered in the quarter, about +1.71 pp) and +1.59 pp at 2025-10-31 (an entering
holding and trades in a +40% quarter; estimate only). Every linked holding links by ISIN. Splits are inferred from the
balance and flagged (Paladin 1:10, Mitsubishi Heavy 10:1). rtn1 is the newest month (checked against URA closes).
Price against NAV: the Nasdaq URA file has no distributions, so the December quarters show the distribution. The
16 US tickers of the map come from knowledge, checked against the N-PORT price (largest difference 4.8%). GLD:
`T-20261125-gld-attribution`.
