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
