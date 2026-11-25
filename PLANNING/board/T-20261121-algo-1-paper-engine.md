---
id: T-20261121-algo-1-paper-engine
title: Build an algo engine that trades live quotes in our own paper simulator
area: trading
status: review
owner: louellet-git
requested_by: louellet-git
head: quant
story: E4.S1
order: 0.9
branch: claude/quant-algo-paper
created: 2026-11-21
acceptance: On the laptop of the person, `fa algo run --strategy demo` reads live Alpaca quotes during market hours, decides in milliseconds, fills each order in our own simulator at the bid or ask, stops at its loss and position limits, and writes a journal; `fa algo replay --strategy demo --date D` on stored data gives the same decisions as the live run of day D for the same quotes.
---
Owner request, 2026-11-21: "You need a means to set up algos to perform fast trades off fast computations tested
in a paper environment." Owner answers (question dialog): venue "Wherever best"; it runs on the owner's laptop.
Decision of the Head of Quant Research: our own simulator first. No order leaves the repo, so the rule "the repo
does not send orders to a broker" stays. An Alpaca paper account is a later ticket, with the `risk-compliance-officer`
and the owner's confirmation, because it is order code.

## Design (lean, Rust, one implementation for replay and live)
1. A `Strategy` trait: it gets each new quote or bar and the time, and returns order intents (side, size, limit).
   The same strategy code runs in replay and live.
2. A market-data source: live = the existing Alpaca client (`rust/src/market/alpaca.rs`, REST, polling every few
   seconds within the plan limit of 200 requests a minute); replay = stored chain copies and bars. A websocket
   stream is a later step (a new dependency, the Head of Technology's area).
3. A simulator broker: a fill at the bid or ask (a parameter for the share of the spread), a fixed latency, fees per
   contract or share, partial fills not modelled (stated). Positions, cash, P&L marked at the mid.
4. Risk: a maximum position, a maximum loss for the day, a kill file (`touch` stops the engine), and a stop at 15:55
   New York time. Each check has a test.
5. A journal: one line for each decision and fill, with the quote time, the decision time and the latency; a daily
   summary in the form of `trading/journal/`.
6. A demo strategy that proves the pipe (not a claim of profit), for example a SPY mean reversion on 1-minute bars.
7. Windows: it builds and runs on the laptop (PowerShell commands in the README line); the keys are read from the
   environment or `.env` as today.

Agent: `strategy-researcher` (with `volatility-derivatives-quant` for an option strategy later).
Gate: `model-validator`, `code-reviewer`; add `risk-compliance-officer` because it touches the "no orders" rule.

Scope note (review 2026-11-21): `fa algo` is outside the trading sleeves. It is a pipe test, and no result is used
for a decision until limits in percent of a stated paper capital exist (the limits are in shares now; 100 shares of
SPY is about $51,000, and a short is allowed).
