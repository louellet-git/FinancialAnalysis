---
id: T-20261119-evaluator-etf-fund
title: Build the value model for ETFs and funds
area: engine
status: backlog
owner: louellet-git
head: fundamental
order: 6
story: E3.S5
branch:
created: 2026-11-19
acceptance: One command gives the look-through value of an ETF as the weighted sum of the model values of its holdings, with the share of the holdings that have a model, and the premium or discount of the price to the net asset value.
---
Sequence: 4 of 5. Start after the price history (`T-20261119-price-history`) and the
automatic model (`T-20261119-dcf-auto-model`), because this model uses the two.

Why a separate model: an ETF has no revenue and no cash flow of its own. Its value is
the value of its holdings.

Method (first version):

- Net asset value: holdings x market prices.
- Look-through value: holdings x the P10, P50 and P90 of the model value of each
  holding (`fa dcf auto` or a reviewed model). Report the share of the weight that has a
  model; a holding with no model uses its market price.
- For a fund that owns loans or private assets (template `fund`): value from the net
  asset value and the history of its discount; no look-through.

Data to get: the holdings. Free source to test first: the SEC form N-PORT (quarterly
holdings, XBRL or XML). The ETF issuers also publish daily files; check their terms.
The correlation between the holdings matters for the range: the sum of the P10 values
is not the P10 of the sum.

Test: an ETF with three synthetic holdings (fixed answers); SPY against the sum of the
S&P 500 models.

**Board note (15:22 UTC, 2026-11-23): moved into story E3.S5** ("Value what a fund holds: remove the limits of the URA page";
word of the person on the URA page of the Head of Economics: "On the limitations section can we fill that out/begin work on
removing limitations."). What this ticket now waits for:
1. The dated holdings of a fund (the Head of Technology builds them from the SEC form N-PORT). Without a dated list the
   look-through value has a look-ahead.
2. The share of the holdings that have a model value. For URA on 2026-11-20 the engine gives a value for one large holding
   (LEU). The miners need the model of `T-20261123-miner-value-model-design`; the 40-F filers need their facts; the
   holdings that are not SEC filers (Kazatomprom, the Sprott trust) have no model.
3. The prices of the holdings (UUUU, SMR, DNN and OKLO have no price file in store a798b9b).
The command must print the covered share next to the value, and a value with a low covered share is not a fund value.
