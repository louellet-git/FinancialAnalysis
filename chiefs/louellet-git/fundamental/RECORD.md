# Record: fundamental (louellet-git)

The exact words of the person that decide or direct the work, with the clock time. Add at the end only.
Form and rules: `chiefs/MEMORY-HOWTO.md`, section 5. This repo is public: remove each secret and each
personal contact detail before you write an entry.

## 2026-11-20 19:27 UTC (chat)
Words: "Can you enhance our current DCF engine? There's questions that are being brought to me about what assumption to use and when. In reality, I'd rather we stick to what is most appropriate, which obviously is quite loose, but I'm going to be relying on you for a lot of decision-making here. Ultimately, I think we want a model that, yes, does mark to market, but we use mark to market to see what the market is implying across industries, sectors, stocks, and that's where the engine comes in. Deterministic way that we can do that. So when it comes down to what assumption we should use, that is a loaded question which should be studied and understood and documented through the assumption documentation process in a way that is easy, repeatable, understandable."
Done: the head decides the conventions (ticket `T-20261120-dcf-conventions-decision`, in progress). New tickets: `T-20261120-dcf-assumption-register`, `T-20261120-dcf-market-implied`, `T-20261120-sic-code` (head: technology).

## 2026-11-20 19:44 UTC (chat)
Words: "Immediate goal is basically optimizing / improving the DCF engine. It's a stochastic engine for calculating various simulations of what a stock price or asset price should be. We're going to stick to underlyings and not expand into options and other stuff for now."
Done: new ticket `T-20261120-dcf-engine-accuracy` (order 1.3). The engine work comes before the other tickets of the queue. No option work in this area.
