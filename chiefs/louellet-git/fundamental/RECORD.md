# Record: fundamental (louellet-git)

The exact words of the person that decide or direct the work, with the clock time. Add at the end only.
Form and rules: `chiefs/MEMORY-HOWTO.md`, section 5. This repo is public: remove each secret and each
personal contact detail before you write an entry.

## 2026-11-20 19:27 UTC (chat)
Words: "Can you enhance our current DCF engine? There's questions that are being brought to me about what assumption to use and when. In reality, I'd rather we stick to what is most appropriate, which obviously is quite loose, but I'm going to be relying on you for a lot of decision-making here. Ultimately, I think we want a model that, yes, does mark to market, but we use mark to market to see what the market is implying across industries, sectors, stocks, and that's where the engine comes in. Deterministic way that we can do that. So when it comes down to what assumption we should use, that is a loaded question which should be studied and understood and documented through the assumption documentation process in a way that is easy, repeatable, understandable."
Done: the head decides the conventions (ticket `T-20261120-dcf-conventions-decision`, in progress). New tickets: `T-20261120-dcf-assumption-register`, `T-20261120-dcf-market-implied`, `T-20261120-sic-code` (head: technology).

## 2026-11-20 19:37 UTC (chat)
Words: "Immediate goal is basically optimizing / improving the DCF engine. It's a stochastic engine for calculating various simulations of what a stock price or asset price should be. We're going to stick to underlyings and not expand into options and other stuff for now."
Done: new ticket `T-20261120-dcf-engine-accuracy` (order 1.3). The engine work comes before the other tickets of the queue. No option work in this area.

## 2026-11-20 19:52 UTC (chat)
Words: "Park leftover work once you wrap up current jobs"
Done: no new work starts. The two jobs in progress are completed and committed (the conventions decision, the market-implied command); the other tickets stay in the backlog with their state on the board.

## 2026-11-20 21:19 UTC (chat)
Words: "Proceed with your work"
Done: the parked work starts again. The data store `louellet-git/fa-data` is the cache (`data/raw`). Next: the full runs of the conventions decision, the gate request, then the engine accuracy ticket.

## 2026-11-20 22:27 UTC (chat)
Words: "Merging rules are those site by head of tech"
Done: the merge rule is `chiefs/ORIENTATION.md`, section 4: the head merges its own pull request after a gate pass and a green check, with no new question. Applied first to pull request 77.

## 2026-11-20 22:27 UTC (correction)
The rule of the entry above is in `chiefs/ORIENTATION.md`, section 6 ("Reviews and merges", rule 3), not section 4.

## 2026-11-20 23:09 UTC (chat)
Words: "I think you might have deleted an old ticket, that's not how we should handle those, speak w tech head"
Done: question to the Head of Technology; the rule is now in `PLANNING/board/README.md` (a merged ticket moves to `PLANNING/board/done/` with `status: done` and `merged:`; never deleted). The two deletions on my branches are replaced by the move.

## 2026-11-21 03:17 UTC (chat)
Words: "I need you to begin working on testing your DCF engine on the back tester. That's the real value. You and the chief of economics will have to look for as long of data as you have, chief of economics could/should even consider periods of time that are outside the length of the stock market when that makes sense."
Done: ticket `T-20261121-dcf-engine-backtest` (order 0.05); first runs of `fa portfolio backtest --strategy value-rank`; questions to the Head of Economics (market-level history) and the Head of Technology (older prices).

## 2026-11-21 03:16 UTC (relayed by the Head of Quality and Risk)
Words: "I need the report to be a trade recommendation, based on fundamentals. Trade recommendations should be sub 3 pages."
Done: `analysis/NVDA/TRADE-RECOMMENDATION.md` (no trade), branch `claude/nvda-valuation` caa0082, in the gate.

## 2026-11-21 03:47 UTC (chat)
Words: "And every week you will need to propose 3 trades to begin on the live trader, beginning next Friday. A valuation engine will be required. Potentially a hedging engine when and if that's needed."
Done: ticket `T-20261121-weekly-trade-proposals` (first Friday 2026-11-27); questions to the Head of Quality and Risk (the gate and the sleeve limits) and the Head of Technology (the live trader).

## 2026-11-21 03:50 UTC (answers to two questions of this head)
Size of each weekly trade: "Not sure, size of trades should be subject to the trade and that responsibility is of the head."
Sides: "Anything".
Done: record `docs/assumptions/A-009-weekly-trade-rule.md` (written before its test): buy, sector-neutral, 3 a week, 1/39 of the sleeve each, 13-week hold; a short waits for its test.

## 2026-11-21 about 03:55 UTC (relayed by the Head of Quality and Risk)
Words: weekly proposals "Paper 4 weeks, then live"; sleeve limits "Not sure".
Done: the weekly ticket says paper for 2026-11-27 to 2026-12-18, live from about 2026-12-25 after Luc approves the limits.

## 2026-11-21 04:30 UTC (chat)
Words: "Ensure that before you have trades and we begin the weekly trades, that you have effectively back tested your future state blind projections using the DCF engine and using prior data. A recommendation should work at any time and point and when it doesn't, the model should have already priced it in such that it does work. You can speak w the economist about this."
Done: ticket `T-20261121-blind-projection-test` (a condition before the first weekly trade, paper included); the test design is written before any run and goes to the Head of Economics for review.

## 2026-11-21 04:30 UTC (chat)
Words: "Backtesting is integral to our work here"
Done: a standing rule of this area: each model change and each strategy has a point-in-time backtest, written before its run (check 6 of `valuation-quant` and `equity-analyst`); proposed to the Head of Quality and Risk as a standard for all areas.

## 2026-11-21 about 04:50 UTC (answer to a question of this head)
Question: if T1 does not pass by 2026-11-26, what happens on 2026-11-27? Answer: "No trades until T1 passes (Recommended)".
Done: the ticket `T-20261121-blind-projection-test` has the decision and the written test of the T1 fix.

## 2026-11-21 14:25 UTC (chat)
Words: "We need a backtesting engine, then we will need to use our DCF engine across a variety of stocks and different points in time in combination with our backtesting engine until we find a positive expected return, any questions? We need rust for this in particular so that we can optimize our run times"
Answers to two questions: the hold-out: "Any data that is ahead of the point in time, since we are dealing with stochastic processes they must be F_t adapted. If we're simulating a DCF in 2004, we don't use info from 2005"; the measure: "We effectively just need to know if the stock is a buy or a short, and for how long or until what price point, then we sell or buy back accordingly and that's our profit over that time period. Accounting for reality after trade would have supposedly been plaved".
Done: ticket `T-20261121-trade-level-backtest` (signal panel, trade simulator, walk-forward rule choice).

## 2026-11-21 about 14:30 UTC (chat and an answer)
Words: "You likely need further back data, as far back as possible". Answer to the source question: "Have head of tech look into it, do what you can I mean time. This is a model calibration/expandion exercise".
Done: data request to the Head of Technology (prices before 2016, delisted companies, index members, financials before 2009, total return; terms, cost, a private store); question to the Head of Economics on the market engine in 16 more countries (Jorda-Schularick-Taylor); the T1 revenue fix started (fundamentals from 2009, no price needed).
