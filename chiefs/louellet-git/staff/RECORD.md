# Record: staff (louellet-git)

The exact words of the person that decide or direct the work, with the clock time. Add at the end only.
Form and rules: `chiefs/MEMORY-HOWTO.md`, section 5. This repo is public: remove each secret and each
personal contact detail before you write an entry.

## 2026-11-21 (chat in the session of the Head of Technology; copied from its record)
Words: "I think we should have a head of staff, which is above you and the other heads. ... Fable is more
for me to speak w and to address the other heads. I want it for overall coordination but it doesn't need
to get into the weeds too much." Then: "1. I think it should go to the head of staff 2. I and the head of
staff set the pace 3. I'll speak w head of staff when convenient for me"
Done: the Head of Staff row and rules in `chiefs/ORIENTATION.md` (#161). This session started at 18:29 UTC
with the title `FA-L - Head of Staff`. The start check found no other session with this title. This
folder was made.

## 2026-11-21 about 18:33 UTC (chat)
Words: "You don't need to wait for my word. You pretty much can work autonomously, but I do want you to have
like a backseat view. What you're kind of working on is overall like project management ... ensuring that
heads are abiding by the head rules within reason. You know, I found that it's better to have like a lean
model, and also just making sure that the work that they're doing makes sense from like a holistic
perspective. ... one of your jobs is to make sure that holistically the system generally is cohesive. And
whatever frame of mind you and I can agree on for that. ... You're kind of a background way of going
about that."
Done: the role in the dashboard (autonomous, in the background; project management; the head rules within
reason; lean; a cohesive system). A first look at the board, with one message to the Head of Fundamental
Research. A frame for "cohesive" was proposed to the person (dashboard, "Frame"); the person has not
agreed to it yet.

## 2026-11-21 about 18:35 UTC (chat)
Words: "Have your frame be a bit more lean"
Done: the frame has three lines now (dashboard, "Frame"). The line about a date and the line about
double work are part of lines 1 to 3.

## 2026-11-21 about 18:37 UTC (chat)
Words: "I think where you'd be helpful is fleshing out the board into epics, stories, tickets, and fleshing
out the stories and the epics and the tickets so that it all kind of makes sense from a holistic point of
view, and just making sure that heads are following the rules."
Done: `PLANNING/board/EPICS.md` (6 epics, each with the decision that it serves), ticket
`T-20261121-board-epics`, the `story` field in `PLANNING/board/README.md`. One message to each head.

## 2026-11-21 about 18:39 UTC (chat, from speech; the first words are not clear)
Words: "... they'll say something like, oh, this is the implied growth and this is the implied premium. And
I'm like, okay, great, but does that make sense? And that's kind of where I need you to be, that in between
of me and them, where they should already be questioning, does this make sense? And going about it like
that on their own. I don't want like a constricting method. I want to lean small amount of lines method,
holistic."
Done: one paragraph "Does it make sense?" in `chiefs/ORIENTATION.md` (end of section 1): a habit, no template,
no gate. Line 4 of the frame. A first sense check of the preliminary week results went to the Head of
Fundamental Research.

## 2026-11-21 about 18:40 UTC (chat)
Words: "And between you and them, like, they really shouldn't be reporting to you very frequently at all. If
they report to you, it's because something is pretty much done, like done, or done and needs your review
before it comes to me."
Done: the paragraph "The Head of Staff" of `chiefs/ORIENTATION.md`: a head sends a message only for a
result that is done (the Head of Staff reads it first if it goes to the person), a decision of the person,
or a conflict. No status report. One line to each head.

## 2026-11-21 about 18:41 UTC (chat, from speech)
Words: "this isn't meant to choke the heads. This is to help steer them better and to help steer everything
a bit more holistically. I also have had issues in the past with maintaining [many] agents, like scope
creep becomes a problem, losing sight of the goal. So that's where you kind of help out. I also really want
like the doc standard to be followed. So making sure that ... when stuff is getting presented to me, it's
in a concise, clear format, ASD-[STE]100. And we're not arguing over little pieces. ... a way that we can
not be arguing over the minutia is to have smaller documentation, more clear, more concise, high signal."
Done: the dashboard ("How I steer"). No new rule for the heads and no message to them.

## 2026-11-21 about 18:46 UTC (chat, from speech)
Words: "Also ensuring that the heads are using their teams and their skills as is part of their requirements.
... some kind of like non-invasive monitoring, non-invasive token monitoring ... ideally, like it's 100%
script-based and doesn't require the heads remembering. ... what we've had in the past is like just
overall like choking of the system. And what I want instead is like a lean ... just a script, like it's
just running and it just does what it has to do. ... If that's not a solution that's possible ... we figure
something else out. ... making sure that the rules that the heads have been assigned are actually being
followed, but in a lean, non-invasive, not token heavy, and efficient way."
Done: research on this machine (the transcript file has the token counts and the tool names; hooks run in
cloud sessions). Two tickets for the Head of Technology in the new story E6.S5: `T-20261121-board-check`
and `T-20261121-usage-log`.

## 2026-11-21 about 18:49 UTC (chat, from speech)
Words: "Anything that you like to implement that is like [hooks] or scripts or managing the heads needs to
have like a few days worth of evidence to prove that it's actually better than any alternative. I've just
had too much issue with overly constricting hooks. So unless there's a hook or system that has no
constrictions, then you can implement that. But just make sure that you're testing before constricting."
Done: the rule is in the two monitoring tickets and in the dashboard. Both tickets are report-only.

## 2026-11-21 about 18:53 UTC (chat, from speech)
Words: "How are we planning to evaluate the tokens of the delegated subagents and how they contribute to
the tickets being worked on? Note that different tickets will have different tokens estimates. And we need
to make sure that the ticket's actual token usage is tracked, again, in a non-invasive way. That should
account for ... how much of it was done by the head of economics, how much by the head of tech ... it
should obviously track the sub-agents. And note that different models have different costs ... I personally
only have so many tokens per week. There is a way to know how much tokens per week is used. You can check
out what the repo [chiefs] did, but note that it's going to have problems ... that was way over
constricting, but we want the lean, easy to use, easy to implement version of that." Also: "And keep it lean".
Done: read `louellet-git/chiefs` (its usage tools). The design is in `T-20261121-usage-log`. Found the
real weekly figure with one call: 71% used on 2026-11-21 18:49 UTC, reset 2026-11-25 16:00 UTC.
