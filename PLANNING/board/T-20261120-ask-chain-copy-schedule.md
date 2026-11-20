---
id: T-20261120-ask-chain-copy-schedule
title: Allow one automatic job each trading day to save option prices?
area: ask
status: backlog
owner: louellet-git
requested_by: louellet-git
head: owner
order: 2
branch:
created: 2026-11-20
acceptance: The person answers in the session that the ticket names.
---
**Question.** May one automatic job run each trading day, at about 3:45 pm New York time, to save the
day's option prices?

**Why it matters.** The free sources do not keep past option prices with their bid and ask. A day that
nobody saves is lost for good. The tests of the option models and the arbitrage checks need many days.
Your rule now is "nothing runs between two commands", so this needs your yes.

**Cost.** One short cloud run each trading day (a small use of tokens), and about 10 MB of data a day.

**Choices.**
- A. Yes, as a test for 20 trading days; then the Head of Technology reports the cost and the use.
- B. Yes, with no end date.
- C. No: a head saves the prices by hand on the days that someone works (days will be lost).

**Recommendation.** A.

**How to answer.** In the session "FA-L - Head of Technology", say "A", "B" or "C".
