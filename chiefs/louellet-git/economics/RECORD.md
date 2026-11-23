# Record: economics (louellet-git)

The exact words of the person that decide or direct the work, with the clock time. Add at the end only.
Form and rules: `chiefs/MEMORY-HOWTO.md`, section 5. This repo is public: remove each secret and each
personal contact detail before you write an entry.

## 2026-11-21 03:04 UTC (start message from the Head of Technology)
Words (of the person, given in the start message): "You can start the head" and "ensure that heads every
now and then will do research on what the industry does I.e what should an economics head be
doing/economics team at a quant/fundamental firm be doing".
Done: The title is `FA-L - Head of Economics`. Ticket `T-20261121-economics-first-brief` on the branch
`econ/first-brief`: the industry-practice note and the first brief.

## 2026-11-21 03:27 UTC (chat)
Words: "I think I also want you to have like a historian team that can help to shed light on some
particularly unique scenarios. So if your historian team ever has like something in particular to say,
I don't know when you should use it or how to go about it. It's not a rule or anything. I just think
it's a good lens to have."
Done: New agent `market-historian` (ticket `T-20261121-economics-historian-agent`, branch `econ/historian`).

## 2026-11-21 14:37 UTC (question dialog in the session of the Head of Technology)
Question: "May we use the 17-country stock market history (Jordà-Schularick-Taylor database) as a private
local copy, and put only the test results (no data) in the repo?" Answer of the person: Yes (relayed by the
Head of Technology). Conditions from the terms: non-commercial use only (a research check, not a trade);
no JST file in any repo; a source note with the citation that JST asks for; a live trade of real money
that rests on a JST result needs the permission of the JST team first.
Done: ticket `T-20261121-economics-jst-design`; local copy only.

## 2026-11-23: Luc's research request on the Anthropic IPO and AI limits (relayed by the Head of Staff)
Luc's exact words: "Can you have some research done? I think it's fundamental research / economist. Why did Anthropic IPO? Like, I thought that they were making revenue. I feel like they already had enough donors. I don't understand why they had to IPO. Like, what do they need that debt for? And it sounds like they got a lot of debt. Like, I want to understand the full scope. So if you can make like a 1 to 2 page report on like why they needed the debt in the first place, or like why they wanted it. And is there anything ahead that might indicate like, hey, AI might slow down? Like, is there any reason like to believe that AI might slow down? Not just from like an investing perspective, but more from like a technological, physics, etc."
Lead: Head of Economics; the Head of Fundamental Research gives the filing numbers. Ticket `T-20261123-economics-ai-buildout-report` (story E2.S5). Conflict of interest: the agents of this repo run on Anthropic models; the report carries the disclosure; no value and no trade view.

## 2026-11-23: Luc's one-page question on URA (relayed by the Head of Staff)
Luc's exact words: "one page - why is URA at its current level? One pager."
Ticket `T-20261123-economics-ura-one-pager` (story E2.S6). No trade view.
Added the same day (relayed by the Head of Staff), Luc's exact words: "I want an analysis including past 3 years". The page explains the level with the path of the last 3 years (about 2023-11 to 2026-11): the main turns, and for each turn the reason and its test.
Luc read the draft (b11b3a0), relayed by the Head of Staff on 2026-11-23. His exact words: "Good first draft for the URA. On the limitations section can we fill that out/begin work on removing limitations." Done: the limits become a table (limit, effect, removal, owner, when), story E3.S5 (Technology: data; Fundamental Research: model).
