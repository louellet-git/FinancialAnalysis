# Record: technology (louellet-git)

The decisions of the person, with the date. Quote the words when the exact words are important. Add at
the end only. This repo is public: no secret, no personal contact detail, no private words.

## 2026-11-20 19:11 UTC (chat; the words were said earlier on 2026-11-20, the entry was written at this time)
Words: "Can you ensure that their chief files advise them to upgrade/update their underlying team agent files/skills when needed. Also, some work on memory is otw for you to implement for chiefs too. Also that their teams operate on sonnet unless otherwise advised"
Done: `chiefs/ORIENTATION.md`, section 3 "Your team" (pull request 54).

## 2026-11-20 19:11 UTC (chat)
Words: the person sent a manual, "How to give each chief session a memory", with no other words.
Done: `chiefs/MEMORY-HOWTO.md`, the memory rules of `chiefs/ORIENTATION.md` (section 8), an index and a record file for each of the four heads.

## 2026-11-20 19:15 UTC (chat)
Words: "note again that the memory is different between heads. They serve different purposes. Memory allows the heads to operate better and remember what's important as they're doing their job, you know, very loosely. But it's more particularly in line with what Anthropic released in regards to memory."
Done: `chiefs/MEMORY-HOWTO.md` written again from the Anthropic post: a purpose for the memory of each head, a small dashboard, few files, the dream pass. The memory rules of `chiefs/ORIENTATION.md` are shorter.

## 2026-11-20 19:28 UTC (chat)
Words: "if the head ever wants to slightly upgrade the agents a little bit, they're more than welcome to in whatever way helps towards the goal that they have. This is not a must."
Done: an optional paragraph next to the dream pass in `chiefs/MEMORY-HOWTO.md`, and the same permission in `chiefs/ORIENTATION.md` (sections 3 and 8).

## 2026-11-20 19:29 UTC (chat)
Words: "So a note for all heads, they are orchestrator agents. Make sure that's in the relevant file."
Done: `chiefs/ORIENTATION.md` (first paragraph and section 1) and `docs/TEAMS.md` say that a head is an orchestrator agent, with what that means.

## 2026-11-20 19:42 UTC (chat)
Words: "Also as a rule all programming should be done in rust"
Done: the rule in `CLAUDE.md` ("Work rules") now includes one-time analysis programs and scratch programs; the same rule is in `chiefs/ORIENTATION.md` (section 9) and `docs/QUALITY-STANDARDS.md` (section 8).

## 2026-11-20 19:45 UTC (chat)
Words: "The head of fundamental is trying to download a bunch of data that I think you probably already have. So I think we shouldn't have all that data spread across 4 sessions. Can you think of a good solution? And let it know to stop that if you can think of a better solution. Also, we don't need everything in Rust, mainly trading and stuff. Like if we need some work in SQL or whatever, like yeah, that's different."
Done: the Rust rule has its new scope (`CLAUDE.md`, `chiefs/ORIENTATION.md` section 9, `docs/QUALITY-STANDARDS.md`). A message went to the Head of Fundamental Research about the data. The one data store waits for a decision of the person on its place.

## 2026-11-20 19:52 UTC (chat)
Words: "Okay, I made the repo."
Done: the data store `louellet-git/fa-data` has the SEC company facts, the SEC earnings lists and the GSW file (8 pushes of less than 40 MB). The repo is public, so the prices are not in it. The rule and the copy command are in `chiefs/ORIENTATION.md` (section 2) and `docs/DATA-SOURCES.md`.

## 2026-11-20 19:54 UTC (chat)
Words: "Great, park leftover work once you wrap up current jobs"
Done: the open state is in the ticket `T-20261120-data-store-prices` and in the dashboard. The local worktrees of merged branches are removed.

## 2026-11-20 20:01 UTC (chat)
Words: "The fa-data won't be private, continue, and load all the data to it for now"
Done: each data set of the cache is in `louellet-git/fa-data` (15 pushes; a scan found no key and no mail address outside the SEC filing files). A new clone takes 28 seconds and `fa dcf auto IBM` gives the same P50 on it. The download of the rest of `next500.txt` runs.

## 2026-11-20 20:16 UTC (chat)
Words: "When merlan1996 gets on and sets up his agent, how will his Claude code session react to this? Should we have a check for sessions that exist/creation of new sessions? He may also want to create sessions, we should potentially distinguish between his heads and ours, + any other contributors"
Done: `chiefs/ORIENTATION.md`: the person of a head is the GitHub login of its session (`get_me`), a check at the start for a second session with the same title, a rule for new sessions, and section 7 for more than one person. `CLAUDE.md`: the data rule for each session.

## 2026-11-20 21:29 UTC (chat)
Words: "Btw, this reduction in billing usage is a serious improvement, good work." Then: "Make sure that the rules for billing that make it sustainable are abided by for all heads"
Done: seven cost rules in `chiefs/ORIENTATION.md` (section 6), a short rule in `CLAUDE.md` for each session, one line in the skill `release-checks`. GitHub Actions cost: $1.19 on 2026-11-19, $0.16 on 2026-11-20, billed $0.

## 2026-11-20 21:41 UTC (chat)
Words: "There is a repo called the -chiefs that I have. [...] just extract what we have and paste it into there in a section called like a folder. Just make it like the lean version [...] direct anyone that spawns that to go grab that and use that as its version. Generalize it for different projects, but keep it lean"
Done: `louellet-git/chiefs` has `lean/` (README, ORIENTATION, MEMORY) and a pointer at the top of its README (chiefs pull request 31).

## 2026-11-20 21:41 UTC (chat)
Words: "Dont take the rules from scan support chiefs on, remove them when you get the chance."
Done: the Scan Support rules are not followed. The delete of the local copy `/home/user/chiefs` was refused by a safety check; the person can approve it. Do not open that folder as a working folder.

## 2026-11-20 21:41 UTC (chat)
Words: "Also, every time a compaction is done, automatically or requested, that is when dreaming occurs"
Done: `chiefs/MEMORY-HOWTO.md` (section 4) and `chiefs/ORIENTATION.md` (section 8) start a short dream pass after each compaction; the same change in `lean/` of the `chiefs` repo.

## 2026-11-20 21:42 UTC (chat)
Words: "Maybe it should be beforehand?"
Done: the dream pass comes before a compaction (context use above about 75% of the threshold, or the person says "dream" first); after a compaction only a short check.

## 2026-11-20 21:54 UTC (chat)
Words: "For all heads, if they find their requests expanding the pre-existing teams/skills they currently have, they are able to improve. Not a need, but they are allowed to add more teams/skills and should consider refinement whenever they deem it a good effort. Not a need so make this a one-liner where relevant, not a stand-alone initiative but permission."
Done: one permission sentence in `chiefs/ORIENTATION.md` (section 3) and in `lean/ORIENTATION.md` of the `chiefs` repo.

## 2026-11-20 21:58 UTC (chat)
Words: "I think we want to set up a paper trade team once we feel ready, half will be for the fundamental team and half for the quant research team. Also, we need a means of backtesting."
Done: two backlog tickets, `T-20261120-portfolio-backtest` (head technology) and `T-20261120-paper-desk` (head owner, with the decisions for the owner).

## 2026-11-20 22:05 UTC (chat)
Words: "whenever a separate contributor wants to begin working, I don't want it to automatically default to the board based on tickets that I've brought up. I think it should default to tickets that the collaborator brought up individually. So when a new session is spawned, don't immediately prompt to start working on tickets that other members have created."
Done: the field `requested_by` (no field means `louellet-git`); a head offers and takes only the tickets of its own person (`chiefs/ORIENTATION.md`, sections 4 and 7; `PLANNING/board/README.md`; skill `ticket-workflow`; the lean version in `chiefs`).

## 2026-11-20 22:24 UTC (chat of the Head of Quant Research; checked in its transcript)
Words: "You don't need me to tell you to continue unless I tell you to stop, less tech head know"
Done: after a ticket, report and take the next ticket of the queue with no wait, until the person says stop. Started `T-20261120-portfolio-backtest`, `T-20261120-store-arb-data` and `T-20261120-delete-merged-branches`.

## 2026-11-20 22:32 UTC (chat)
Words: "Also, I need every team to have a team that scrutinizes the amount of tests that any given team has, not run all the time but every so often at the discretion of the head"
Done: a test-audit mode in `.claude/agents/test-engineer.md`; one paragraph in `chiefs/ORIENTATION.md` (section 3): each head starts it from time to time; not a gate. Same line in the lean version of `chiefs`.

## 2026-11-20 23:01 UTC (chat)
Words: "Think about many heads from many users working, and consider that from a lean perspective to find a solution to this potential future issue. Token cheap, GitHub cost cheap, line of code cheap and time cheap,"
Done: measured the files that today's merges touched most (CLAUDE.md 7, docs/ARCHITECTURE.md 6, README.md 5, PLANNING/token-ledger.md 4, docs/HIDDEN-ASSUMPTIONS.md 3). Removed the test counts from the documents; new traps are one file each in `docs/traps/`; the shared token ledger takes no new row; four rules in `chiefs/ORIENTATION.md` (section 2, "Many heads at the same time").

## 2026-11-20 23:09 UTC (chat)
Words: "Please take the recent state of the heads operating system, and merge to the chiefs repo. I want an exact replica of our current system"
Done: `lean/` of `louellet-git/chiefs` is a byte-for-byte copy of the heads system at commit fd830a9 (chiefs pull request 32). `CLAUDE.md` is `CLAUDE.template.md` and `.claude/` is `dot-claude/` there, so that the copy does not load in the Scan Support sessions. A later change of the system here needs a new copy there.

## 2026-11-20 23:08 UTC (chat of the Head of Fundamental Research; checked in its transcript)
Words: "I think you might have deleted an old ticket, that's not how we should handle those, speak w tech head"
Done (2026-11-20 23:10 UTC): a merged ticket moves to `PLANNING/board/done/` (`git mv`, `status: done`, a line `merged:`); never deleted (`PLANNING/board/README.md`, skill `ticket-workflow`, `docs/TEAMS.md`). The 27 tickets that were deleted before are restored from git history into `done/`.

## 2026-11-20 23:18 UTC (chat)
Words: "For the lean system, don't include the agents and skills but include the logic/rules that govern having agents and skills being created that I've already stated. We should be transferring over memory preferences, board preferences, everything, just generalized"
Done: `lean/` of `louellet-git/chiefs` is the general version (8 files, rules only; chiefs pull request 33). It replaces the exact copy of chiefs pull request 32.

## 2026-11-20 23:22 UTC (chat of the Head of Quality and Risk; relayed, not checked in its transcript)
Words: "Don't over commit to one model, since the scope of the job will change is what I meant. Tell the tech that. Keep the rule lean and not a need but permission to."
Done (2026-11-20 23:29 UTC): `chiefs/ORIENTATION.md`, section 3 "Model": the agent files give Sonnet; for one run a head can choose a different model that fits the job; a permission, not a duty.

## 2026-11-20 23:30 UTC (chat)
Words: "Also eventually we're going to begin making trades on the paper account and eventually on live account. This will mean write ups explaining the logic of the trade and the steps taken/etc. this should never exceed a doc of 2 pages. But it does need to be documented. Also, it should be a standard for everyone to be using 85% ASD-STE100, unless it already is"
Done: `trading/journal/README.md` (one write-up for each trade, at most 2 pages, with a form), a row in `docs/QUALITY-STANDARDS.md`, a note in the paper-desk ticket. The language rule of `CLAUDE.md`, the skill `ste-writing` and the document reviewer use "about 85% ASD-STE100"; a document that already meets it needs no rework.

## 2026-11-20 23:49 UTC (chat)
Words: "Great, that works. We need a way that you can get permission requests to the user once it's filtered. Lean method? I don't want a bunch of questions buried in the session."
Done: `chiefs/ORIENTATION.md`, section 5, "Requests for the person": a refused command or a decision of the person goes to the Head of Technology, which filters it; what stays becomes one board ticket with `area: ask` and `head: owner`, and one push notification for each batch. First ask: `T-20261120-ask-rates-appendix-run`.

## 2026-11-20 23:51 UTC (chat)
Words: "Also the question should be delivered in clear terms"
Done: the ask form in `chiefs/ORIENTATION.md`, section 5 (Question, Why it matters, Choices, Recommendation, How to answer; plain words, no internal names); the three open asks written again in that form; the push names each question in plain words.

## 2026-11-20 23:52 UTC (chat)
Words: "I need the question as a yes/no from the sessions to my phone"
Done: an ask is a yes/no question; the session where the person answers pushes it to the phone ("Yes or no: ...? Recommended: ..."). `chiefs/ORIENTATION.md`, section 5; the three open asks written again.

## 2026-11-21 00:08 UTC (chat)
Words: "I need the question as an actual question I click y/n for, lean"
Done: `chiefs/ORIENTATION.md`, section 5: the session where the person answers shows the question dialog (`AskUserQuestion`) with "Yes" and "No", plus a push notification to open the session.

## 2026-11-21 00:11 UTC (question dialog)
Question: "May one automatic job save the day's option prices each trading day (about 3:45 pm New York time), as a test for 20 trading days?"
Words: "Yes (Recommended)"
Done: the scheduled task "Daily option price save (20-day test)" (15:27 New York time, market days; SPX, SPY, VIX chains to the store; stops after 20 days). Ticket moved to `PLANNING/board/done/`.

## 2026-11-21 00:37 UTC (chat)
Words: "Is the lean method up to date? Can you move to a new repo called the-heads. I created it so it'll be separate from the chiefs."
Done: it was not up to date (no model permission, no ask flow, no board area `ask`, no write-up row). `louellet-git/the-heads` has the general system with these rules (commit 6264abb). `lean/` is removed from `louellet-git/chiefs`, and its README points to the new repo (chiefs pull request 35).

## 2026-11-21 02:57 UTC (chat)
Words: "Please get to work on the backtesting environment, ask the heads what they require."
Done: ticket `T-20261121-backtest-environment` (order 1); a question to the Quant, Fundamental and Quality and Risk heads.
## 2026-11-21 03:00 UTC (chat)
Words: "I think we should also get a head of economics which looks at current economic conditions/factors and is generally our economy guy. Supporting role."
Done: a new row `Head of Economics` in `chiefs/ORIENTATION.md` (sections 1 and 2; folder `economics`, files `analysis/macro/`), the agent `macro-economist`, its row in `docs/TEAMS.md`. The person starts the session.

## 2026-11-21 03:03 UTC (chat)
Words: "You can start the head, ensure that heads every now and then will do research on what the industry does I.e what should an economics head be doing/economics team at a quant/fundamental firm be doing, same w the other heads."
Done: `chiefs/ORIENTATION.md`, section 3, "Industry practice" (from time to time, a note of at most 1 page in the folder of the head; at most three proposed changes; not a gate). The same rule in `louellet-git/the-heads`. The session "FA-L - Head of Economics" started.

## 2026-11-21 03:23 UTC (question dialog)
Question: "May I subscribe to the Tiingo Power plan ($30 a month, with your Tiingo key) and keep its older daily prices (back to about 2009, for about 1,000 tickers) in a new private data store that only your sessions use? ..."
Words: "No, please find a better source then if needed"
Done: no Tiingo plan, no private store. Research of free sources whose terms permit a public copy (Quandl WIKI archive, Stooq, Alpaca, open datasets). Tiingo terms (read 2026-11-21, https://www.tiingo.com/tos section 1.6): the free plan forbids any persistent storage; paid plans are internal and personal use only.

## 2026-11-21 05:04 UTC (chat)
Words: "Remove the tests used for a decision I think that's overdoing it ?" and "The backtest harness and the hypothesis file are more for the heads that require it"
Done: the row "Test or experiment that a person uses for a decision" removed from `louellet-git/the-heads` (commit 96eabd4). In this repo the harness is opt-in: `trading/hypotheses/README.md` says who uses it; an exploratory run that reaches a registered window is no longer refused (a WARNING names the hypothesis).
## 2026-11-21 04:03 UTC (relayed by the Head of Quant Research; checked in its transcript)
Words: "Before asking me anything, please look into it. Send that to the tech head."
Done: `chiefs/ORIENTATION.md`, section 5, item 2 of "Requests for the person": research comes first; a question carries the result and a recommendation; a question that research can answer is not asked. The same rule in `louellet-git/the-heads`.

## 2026-11-21 04:16 UTC (chat, then the question dialog)
Words: "I'd rather we run the quant algo somewhere that other research firms will typically do it, but for free."
Question: "May the quant algo engine run on a free Google Cloud e2-micro machine in us-east1 ... set up in your own Google account? ..."
Words: "Yes (Recommended)"
Done: ticket `T-20261121-algo-host-gcp` (order 2). The Head of Quant Research is told.

## 2026-11-21 14:58 UTC (chat)
Words: "Can you add to the-heads and to our heads that they can parallelize work when it is reasonable to"
Done: `chiefs/ORIENTATION.md`, section 3, "Parallel work" (a permission: independent agents in their own worktrees, or two tickets that touch different files; not for work on the same files, dependent work, or more agents than the head can examine). The same rule in `louellet-git/the-heads`.

## 2026-11-21 15:02 UTC (chat)
Words: "Remove out the not a duty part, and make the rule lean so that it's as lean as other models"
Done: "Parallel work" is two lines, with no "not a duty" sentence, here and in `louellet-git/the-heads`.

## 2026-11-21 (dialog)
Words: "Yes (Recommended)" to: may I move the FRED files out of the public store into local copies that each head gets with one command?
Then: "Would it help if I just made the repo private instead?" Answer: only part of the way (the terms also prohibit storage and giving the data to a second person; private repos use paid Actions minutes; each session needs access). The plan stays.
Done: ticket `T-20261121-fred-local`.

## 2026-11-21 (dialog)
Words: "Yes (Recommended)" to: will you make fa-data private? (The terms of Nasdaq, Yahoo, Cboe, FRED and ICE do not permit a public copy; GitHub ToS D.3.)
Done: the store README says private (store e3b735e); the heads are told to attach fa-data before a clone. The owner changes the visibility on GitHub. The FRED move continues.

## 2026-11-21 (refusals reported by the heads)
- Fundamental Research: the permission system refused `git push origin claude/sp100-week-attribution` (its own branch). Not needed: the same commits are on GitHub through claude/premium-series. Cause not known. Do not try again in another form.

## 2026-11-21 ~18:00 UTC (dialog in the Quality and Risk session, read in its transcript)
Words: "Shared store, I accept" (the daily save of the Alpaca chains goes into the shared private store; the owner accepts the terms risk).
Done: the note in `docs/DATA-SOURCES.md`, section "Alpaca paper trading".

## 2026-11-21 (chat)
Words: "I think we should have a head of staff, which is above you and the other heads. ... Fable is more for me to speak w and to address the other heads. I want it for overall coordination but it doesn't need to get into the weeds too much." Then: "1. I think it should go to the head of staff 2. I and the head of staff set the pace 3. I'll speak w head of staff when convenient for me"
Done: the Head of Staff row and rules in `chiefs/ORIENTATION.md` (questions for the person go through it; the person and it set the order; no fixed report time); the session.
## 2026-11-21 (dialog)
Words: "Yes (Recommended)" to: may we keep the Philadelphia Fed SPF file as a private copy (not in the public repo), with only the results in the repo?
Done: ticket `T-20261121-spf-loader`.
Words: "Can you send a blast ensuring that models should be properly documented as per the rules?"
Done: a message to the four heads with the rule (CLAUDE.md "Model documents"; QUALITY-STANDARDS section 4): each head lists the models of its area without a document or a record and writes them; the Quality and Risk head refuses a model change without a current document.
