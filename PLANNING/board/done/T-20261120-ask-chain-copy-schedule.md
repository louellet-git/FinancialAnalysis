---
id: T-20261120-ask-chain-copy-schedule
title: Yes or no: test a daily automatic save of option prices for 20 trading days?
area: ask
status: done
owner: louellet-git
requested_by: louellet-git
head: owner
order: 2
branch: claude/chain-save-yes
created: 2026-11-20
acceptance: The person answers yes or no in the session that the ticket names.
merged: 2026-11-21 (answer: yes)
---
**Question.** Yes or no: may one automatic job save the day's option prices each trading day (about
3:45 pm New York time), as a test for 20 trading days?

**Why it matters.** The free sources do not keep past option prices with their bid and ask, so a day
that nobody saves is lost for good; the option model tests need many days. Your rule is "nothing runs
between two commands", so this needs your yes. Cost: one short cloud run each trading day and about
10 MB a day. After 20 days the Head of Technology reports the cost and the use.

**Recommendation.** Yes.

**Answer in.** The session "FA-L - Head of Technology".

**Answer.** Yes (2026-11-21, the question dialog). The Head of Technology made the scheduled task "Daily option price save (20-day test)": each market day at 15:27 New York time a new cloud session saves the SPX, SPY and VIX chains (Alpaca) and pushes them to the data store. It skips 2026-11-26, 2026-11-27 and 2026-12-25 and stops itself after 20 days (about 2026-12-22). Then the Head of Technology reports the cost and the use.
