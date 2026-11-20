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
