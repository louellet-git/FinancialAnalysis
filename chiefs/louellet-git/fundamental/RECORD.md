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

## 2026-11-20 23:21 UTC (chat)
Words: "I need to know why the market crashed today and over past few days, generally what I'm looking for is an understanding of where the market valuation WENT. This is a large task that requires its own models being built, and probably will start with you doing one stock at a time until you've built a general enough model to explain. I want to know what NVDA is worth."
Done: tickets `T-20261120-nvda-valuation` (order 0.1) and `T-20261120-valuation-move-attribution` (order 0.2); data request to the Head of Technology.

## 2026-11-21 01:04 UTC (chat)
Words: "Ensure that work we do is repeatable and contributes to the dcf engine"
Done: (1) each ticker or market analysis has a command file with the store commit, so that a person can run it again; (2) each judgment of a ticker model that can be a general rule is listed and goes to the engine ticket `T-20261121-dcf-engine-from-tickers`; (3) the analyst and quant agent files have this as a check.
