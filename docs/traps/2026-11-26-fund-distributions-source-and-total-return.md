# The Nasdaq dividend endpoint is empty for NYSE Arca funds, and a distribution is not in the Nasdaq close

*Trap:* `api.nasdaq.com/api/quote/URA/dividends?assetclass=etf` answers HTTP 200 with the message "Dividend History for
Non-Nasdaq symbols is not available" and empty rows. The class `stocks` answers "Symbol not exists". An empty answer is not
"no distribution". The Nasdaq close (`prices.json`) is not adjusted for distributions: a December distribution of URA is a
drop of the price on the ex-date, and it makes the price return lower than the NAV total return of N-PORT by the size of the
distribution (-9.18, -3.28 and -6.42 pp before the fix, the quarters with December).

*What to do:* Use `fa fund distributions TICKER` (Yahoo chart, `events=div`). It gives ex-date and amount, not the pay date. The
amount has the share count of the day of the request: after a split, check it. The price total return of a quarter is
(close_end + distributions with an ex-date in (start, end]) / close_start - 1. The distribution is not reinvested inside the
quarter (a small second-order effect). Check the yearly sum against the financial highlights of the SEC N-CSR (the fiscal year
of URA ends 31 October, so a December ex-date is in the next fiscal year).

*Evidence:* `rust/src/market/fund_distributions.rs` (`total_return`, tests); request of 2026-11-26 in
`docs/DATA-SOURCES.md` (row "Fund distributions"); the N-CSR of 2026-01-09 (accession 0000930413-26-000079): FY2021 to FY2025
agree with the Yahoo sums to the cent.
