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
## 2026-11-20 23:21 UTC (chat)
Words: "I need to know why the market crashed today and over past few days, generally what I'm looking for is an understanding of where the market valuation WENT. This is a large task that requires its own models being built, and probably will start with you doing one stock at a time until you've built a general enough model to explain. I want to know what NVDA is worth."
Done: tickets `T-20261120-nvda-valuation` (order 0.1) and `T-20261120-valuation-move-attribution` (order 0.2); data request to the Head of Technology.

## 2026-11-21 01:04 UTC (chat)
Words: "Ensure that work we do is repeatable and contributes to the dcf engine"
Done: (1) each ticker or market analysis has a command file with the store commit, so that a person can run it again; (2) each judgment of a ticker model that can be a general rule is listed and goes to the engine ticket `T-20261121-dcf-engine-from-tickers`; (3) the analyst and quant agent files have this as a check.


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

## 2026-11-21 about 14:40 UTC (relayed by the Head of Economics)
Words (the person's answer on the JST data, as relayed): Yes, a local copy only; only results go in the repo; a live trade on a JST result needs the permission of the JST team first.
Done: the reader waits for the frozen design (frozen 31ed025, branch `econ/jst-design`); the condition goes into the test file.

## 2026-11-21 about 14:45 UTC (chat)
Words: "Also did you ever get me that 3 page doc on why the market has tanked this week? I want a valuation, so each piece of the overall drop should be attributable to a piece of valuation. If it's easier to do company by company then so be it, I'd like it on the S&P100. You can hold off on other work for now and focus on this."
Done: no, the findings file of `T-20261120-valuation-move-attribution` was not written. New ticket `T-20261121-sp100-week-attribution` (first item). The trade-level engine and the T1 revenue fix are paused, with their work committed and a status note on each ticket.

## 2026-11-21 about 15:10 UTC (chat)
Words: "Start w S&P10 / Updating data should be relatively easy, speak w tech / Make sure that whenever you stop work, just make sure that it all goes on the board for later and just report on where it is, to help out future users."
Done: the document starts with the 10 largest companies of the S&P 100 (one weight for each company). A standing rule of this head: each pause puts the state of the work on the board (the ticket says what is done, what is not, the next step), and the report says where it is.

## 2026-11-21 about 15:40 UTC (chat)
Words: "Okay, now I recommend you start with the S&P 100, but we should then be able to attribute / break down the model valuation of each component of the S&P 10 and be able to explain that such that the sum of the parts comes out to the S&P 10."
Done: the document starts with the S&P 100 bridge; a second bridge has the 10 largest companies only, company by company, and its lines add to the change of the S&P 10 (value-weighted).

## 2026-11-21 about 15:50 UTC (chat)
Words: "If there's a problem with the data, that needs to be fixed."
Done: two data defects found and sent to the Head of Technology, the owner of the files (the point-in-time mapping does not use the filing supplement, so V, BRK-B, COP and NEE fail at a past date; a duplicate row in the Treasury par file). The model limits (AMZN and INTC below 0, TSLA, AMD and PLTR outside the search range, banks with no template) are not data defects; the document states them.

## 2026-11-21 about 16:50 UTC (chat)
Words: "Can we break down higher implied risk premium? Is there any research work that can be done on this assumption for understanding it/quantifying it better / Hold off on writing the doc until I give my go ahead on the results / While you research that, please do the same break down analysis over the past 2 months for the S&P10"
Done: the document is on hold (draft kept, no review started) until the owner approves the results. Research on the implied premium starts (a plan with literature, data and first measurements). The S&P 10 breakdown runs week by week over 2026-09-18 to 2026-11-20, plus one bridge over the whole period.

## 2026-11-21 about 17:15 UTC (chat)
Words: "Can the head of tech check if there are other problems like those"
Done: request to the Head of Technology for a full data audit (one board ticket, an owner for each defect, the S&P 100 first), with a first list: 23 S&P 500 tickers fail the point-in-time mapping at 2026-11-13 but pass the normal mapping (WARN); BF-B has no price file; IBM and JNJ stop at 2026-11-18; and seven more checks to run.

## 2026-11-21 about 19:00 UTC (chat)
Words: "When will the report be ready for S&P10?" Answer to the go-ahead question: "Did you break down implied premium and growth? Also what is no model points". Then: "Where are we getting the 0.78 from?" and "For the difference i.e attributable to the indiviudal company, that is somethign to be investigated/understood."
Done: no go-ahead yet (the document stays on hold). The 0.78 is the median change of the implied premium over the 65 S&P 100 tickers with a value at both dates (store 652dc28 gives the same). New ticket `T-20261121-premium-company-part` with a design written before any analysis (sensitivity, market exposure, company news such as an earnings 8-K in the week).

## 2026-11-21 about 19:10 UTC (chat)
Words: "I think we also need to make sure that we're properly evaluating the impact of rates on a company by getting an appropriate value of how it's assets and liabilities are affected by the change in curves."
Done: ticket `T-20261121-rates-balance-sheet` with a design written before any measurement (debt at fair value and its ladder, debt securities held, pensions, floating debt and cash, the full curve); first a measurement for the S&P 100, then a model change with its own backtest only if the effect is material.

## 2026-11-21 about 19:30 UTC (chat)
Words: "Is all DCF work fitting to the DCF engine?" Answer to the one-engine question: "It depends, we need some way for this all to run efficiently/smoothly. What I meant was more so that."
Done: ticket `T-20261121-dcf-engine-integration` (one curve rule, a per-ticker weekly bridge, the common and company parts in Rust, the balance-sheet effect in the bridge if material, a cache of the point-in-time mapping for speed, the model documents). No change to the number of engines.

## 2026-11-21 about 20:10 UTC (chat)
Words: "Can you draw me up a visual html to describe via mermaid chart? of the overall process, less words more diagrams and visuals" Done: artifact https://claude.ai/artifact/6edSJhjvtqoedfNZ1VA3uC (private).
Words: "Hm, I think that the implied premium/implied growth should also be part of the attribution engine." Done: design change on `T-20261121-premium-company-part`: `fa dcf attribute` splits the premium and the growth plug into a common part and a company part (engine steps that add exactly), with an 8-K group line.

## 2026-11-21 about 20:30 UTC (chat)
Words: "For attribution, there are numbers in the valuation that are a direct result of the math, but there are some that are implied. The implied components require the view of the economists." and "The economists require their own engine in that regard."
Done: design change 2 on `T-20261121-premium-company-part`: the common change of the premium and the growth comes from an Economics engine (owner: the Head of Economics) through a file interface; the math parts stay in `fa dcf attribute`. Request and interface sent to the Head of Economics.

## 2026-11-21 about 17:20 UTC (in the session of the Head of Quality and Risk, relayed)
Words (on Deere and a captive finance arm, after the explanation of the sum of parts): "Enhance the DCF engine."
Done: ticket `T-20261121-evaluator-captive-finance` (detection, then an industrial DCF plus a finance-arm equity value; after the bank evaluator; gate model-validator + code-reviewer with a backtest).
## 2026-11-21 about 17:10 UTC (in the session of the Head of Quality and Risk, relayed)
Words: "yes approve the merge but this doesnt mean I approve that the valuation is correct"
Done: pull request 106 (NVDA) merges. It is NOT an approval of the value: the report keeps "a range for a judgment, not a price target", and no later document or trade may say that the owner approved the NVDA value.

## 2026-11-21 about 18:00 UTC (chat)
Words: "Changing you to fable for this project, proceed" (the model of the session changed; no change of the work).
Words: "The point of the attribution valuation is to understand any recent changes in prices of any given set of underlying, explain what contributed to the change, and whether those impacts make sense with reality/possibility. The point is to understand if we have a buy opportunity or a correction for this particular instance, but will be used more generally in the future for understanding/analysis from a fundamentals perspective. Do you have questions?"
Done: the purpose is recorded: (1) what changed, (2) what contributed, (3) does each implied piece make sense against reality (history, fundamentals, the economics view), (4) a reading: buy opportunity or correction. Two questions asked (the form of the conclusion; the horizon).

## 2026-11-21 about 18:50 UTC (chat)
Words: "Great, make sure that whatever is repeatable/should be expected of future work like this is built into the correct engine."
Done: the list of the repeatable steps of this study, each with its engine and owner, is on `T-20261121-dcf-engine-integration` (written before the code); the build starts on branch `claude/attribution-reality` (from `claude/premium-series`, which is at the gate).

## 2026-11-21 about 19:05 UTC (chat)
Words: "Send the same request of work to quant research for its engine/processes."
Done: the request is sent to the Head of Quant Research with the owner words, the four steps as applied in this area, the example table (`T-20261121-dcf-engine-integration`) and the borders between the two areas.

## 2026-11-21 about 19:20 UTC (chat)
Words: "Give me the prelim results before the report, that's the general rule."
Done: a standing rule of this head: the preliminary results go to the owner first (in the chat, with a results file on the branch); a report is written only after the owner has seen them and gives the go-ahead. First use: `analysis/priors/sp100_week_prelim_results_2026-11-21.md`.

## 2026-11-21 about 19:35 UTC (chat)
Words: "But why the sudden rise in the return investors are asking for? I've heard that people are expecting a rate rise too"
Done: three more runs (11-06 to 11-13, 11-06 to 11-20, 10-30 to 11-20) and the yield data: the premium rise of the last week mostly reverses the fall of the week before, when the yields rose (10-year 4.86% to 5.15%) and the stocks did not follow; section 6 of the preliminary results. News (a high CPI on 11-10, a December rate increase priced) is cited as reported, not checked.

## 2026-11-21 about 19:50 UTC (chat)
Words: "You should generally be asking the head of economics these questions"
Done: a standing rule: each macro "why" question (rates, the central bank, inflation, the required return of the market) goes to the Head of Economics; this head gives the owner that answer with its source. The question on the rate rise is sent with the data of section 6.

## 2026-11-21 about 20:40 UTC (from the heads)
The Head of Economics answered the rate question (a248803, audited): a rate rise of about two quarter-point steps in a year is priced in the bill curve; the December odds of the news are not ours; the yield rise of the week to 11-13 was a real-rate rise; "a late reaction" is one fair reading, not proven. Section 7 of the preliminary results. Correction by this head: the "high October CPI" of the news is not visible in the data of the store.
New rule (owner decision, PR 161): a Head of Staff is above the heads; send it only: a large task done or merged, a block of more than half a day, a decision that only the owner can make, a conflict with a head. A permission refusal still goes to the Head of Technology.

## 2026-11-21 19:35 UTC (from the heads; dream pass)
The Head of Staff: the order until 2026-11-25 11:00 Toronto time (agreed with the person, `PLANNING/board/EPICS.md`): the week report to its end, then the T1 correction, then new studies after the reset of the weekly token limit (71% used). The report still waits for the go-ahead of the person on the results (a message of the Head of Staff is not his approval).
Dream pass done before the compaction: the dashboard is rewritten (no session ids in it), with the page `memory/attribution-state.md`.

## 2026-11-21 19:24 UTC (words of the person, given in the session of the Head of Staff and relayed)
Words: "I guess what I would like from the fundamental research is if we've explained the S&P 10 and we're pleased with our explanation, is there any stock where that explanation doesn't hold up? Was anything hit less? Was anything hit more? If S&P10 finished expand to S&P100. Have a one pager for the S&P10 for my review before moving on to the additional work."
Done: one page for the S&P 10 (`analysis/priors/sp10_week_one_pager_2026-11-21.md`): the explanation, then for each of the 10 stocks whether it holds (hit more, hit less, or as expected), with the reason or "not explained". It goes to the Head of Staff first, then to the person. The S&P 100 part and the other work wait for his review.

## 2026-11-21 19:29 UTC (review word of the person on the S&P 10 page, relayed by the Head of Staff)
Words: "The report should include the entire week including 11-20. It's a huge drop."
Done: the page now has the week by day (SPY, the equal weight of the S&P 500, each of the 10) and a block on 2026-11-20: the fall of that day was in the chip and AI-hardware companies, the yields fell, and the cause is not in our data; the missing data are named and asked (the Head of Economics: the daily view; the Head of Technology: the 8-K filings of 11-19 and 11-20).

## 2026-11-21 19:30 UTC (second review word of the person on the S&P 10 page, relayed by the Head of Staff)
Words: "And AAPL leaving something not yet proven isn't acceptable, same w the others"
Done: a standing rule: no "possible reason, not proven" in a result; each reason is a claim with its test, and the result is proven, refuted, or "tested, no cause found" with the list of the tests. Applied: the evidence file `analysis/priors/sp10_week_tests_2026-11-21.md` (T1 to T5) and the page. Results: NVDA's fall is the fall of the chip sector (proven); the AAPL reason of this head is refuted; BRK-B and AMZN have no effect against a market that does not contain them; the cause of the chip fall of 2026-11-20 is tested with no cause found.
