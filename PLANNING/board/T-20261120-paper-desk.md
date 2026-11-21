---
id: T-20261120-paper-desk
title: Set up a paper-trading desk with one half for each research head
area: trading
status: backlog
owner: louellet-git
head: owner
order: 3
branch:
created: 2026-11-20
acceptance: A paper book with two sleeves of equal capital (Fundamental Research, Quant Research) records each paper trade with its date, price and source, gives a daily profit and loss for each sleeve, and runs only strategies that passed the portfolio backtest and a review.
---
Owner request, 2026-11-20: "set up a paper trade team once we feel ready, half will be for the fundamental team
and half for the quant research team". The owner starts this ticket when the owner is ready (head: owner).

Decisions for the owner before the start:
1. Where the paper trades go. (a) A paper book in this repo (no order leaves the repo; the rule of `CLAUDE.md`
   "It does not send orders to a broker" stays). (b) The Alpaca paper account (orders go to a paper broker, no
   real money; this changes the rule of `CLAUDE.md`). Recommendation of the Head of Technology: (a) first, (b)
   when the fills of (a) look too easy.
2. The capital of each sleeve and the limits (largest position, largest loss in a day, stop of a sleeve).
   Recommendation: the Head of Quality and Risk proposes the limits.
3. Which head runs the desk each day. Recommendation: each research head runs its own sleeve; the Head of
   Technology keeps the book and the data; the Head of Quality and Risk checks the limits each week.

Entry condition for a strategy: it passed `T-20261120-portfolio-backtest` and a review. The option bots also
need `T-20261119-options-paper-test` (live quotes against model prices) first.

Owner word, 2026-11-20: each paper trade, and later each live trade, has a write-up of at most 2 pages
that explains the logic of the trade and the steps taken (`trading/journal/README.md`).
