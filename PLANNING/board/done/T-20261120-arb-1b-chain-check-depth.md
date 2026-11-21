---
id: T-20261120-arb-1b-chain-check-depth
title: Make the chain check separate real breaks from bad quotes
area: trading
status: done
owner: louellet-git
requested_by: louellet-git
head: quant
order: 1.3
branch: claude/upbeat-cori-a4ykf5
merged: 100
created: 2026-11-20
acceptance: `fa options check` gives the breaks in size bands, marks each quote that is far from its neighbours, uses the ask of a quote with bid 0, has the bounds call <= S and put <= K, and `--compare` has fixture tests (the real two-day run is in `T-20261120-arb-3-index-parity-box`, because the store has one day).
---
Follow-up of `T-20261120-arb-1-chain-check` (review of 2026-11-20, PASS-WITH-NOTES, notes 4, 6, 7, 9;
findings note `trading/research/arb-01-chain-check-2026-11-19.md`). On IBM and JNJ of 2026-11-19, each of the
largest breaks comes from one stale quote. The output must show a person the few breaks that are not that.

1. Size bands (below 0.10, 0.10 to 1, 1 to 10, above 10 USD) and a size floor option.
2. A quote far from its neighbours (its implied volatility against the smile of its series) is marked. Report
   breaks with and without the marked quotes.
3. A quote with bid 0 and an ask above 0: its ask is a valid buy leg. Add the bounds call <= S and put <= K.
4. An explicit list of the European indexes (SPX, XSP, NDX, RUT, VIX) before such a chain is checked.
5. Tests: a missing ask, a series with one strike, 0 days, a calendar pair across two roots, a duplicate strike.
6. Two days: when the store has a second day of a chain, give which breaks stay. A break that stays one day is
   more likely a real price than a stale quote.

Agent: `volatility-derivatives-quant`. Gate: `model-validator`, `code-reviewer`.
