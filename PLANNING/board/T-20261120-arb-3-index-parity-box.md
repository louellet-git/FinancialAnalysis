---
id: T-20261120-arb-3-index-parity-box
title: Compare the box-spread rate and the SPX and SPY forwards with the rate curve
area: trading
status: backlog
owner: louellet-git
requested_by: louellet-git
head: quant
order: 3.1
branch:
created: 2026-11-20
acceptance: For one copied day, a table gives for each SPX expiry the box-spread rate at executable prices against the Treasury bill curve and SOFR, and the SPX parity forward against 10 times the SPY parity forward, with the implied dividend difference. Also `fa options check SPX --date D --compare D2` on two copied days (moved from arb-1b).
---
Owner request, 2026-11-20: start to find arbitrage in our data.

Depends on: `T-20261120-arb-1-chain-check` (the parity code) and `T-20261120-vol-7-chain-copy` (SPX, SPXW
and SPY chains with bid and ask at the same time).

1. Box spread: buy the call spread and the put spread of two strikes (SPX, European). Its price at the ask
   gives a borrow rate, at the bid a lend rate. Compare with the bill curve and SOFR of the same day.
2. SPX against SPY: the parity forward of SPX and 10 times the parity forward of SPY for the same expiry date.
   The difference gives the dividend difference and the ratio drift (#280). Use the AM or the PM root that
   has the same settlement time as SPY, or say the error.
3. One day of SPX from Cboe and from Alpaca: compare the bid, the ask and the forward (`options_cli.rs`
   has both sources; nothing compares them).

Agent: `volatility-derivatives-quant`, with `rates-credit-quant` for the rates. Gate: `model-validator`,
`code-reviewer`.

Method (industry note `chiefs/louellet-git/quant/industry-2026-11-21.md`, 2026-11-21): measure the box rate as the
NY Fed does (van Binsbergen, Diamond, Van Tassel, Liberty Street Economics, 2023-10-02): for each SPX expiry,
regress the put minus call mid on the strike; the slope gives the discount factor; keep bid > 0, 1 month to 5
years, and an expiry only when R² ≥ 0.99999. Their 1996-2023 box rate was about 35 bp above Treasuries. Run the
arbitrage parts on NBBO quotes with the underlying at the same minute (Cboe), not on the Alpaca indicative feed.
